//! The benchmark: a bounded closed-loop HTTP load in which `concurrency`
//! workers each send one GET at a time until the duration ends. Bodies are
//! drained and counted as served (never decompressed), not stored.
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

pub(super) const METHOD: &str = "closed loop: each connection sends the next GET only after the \
    previous response body was fully read";
/// What every measuring request accepts, as a current browser does.
pub(super) const ACCEPT_ENCODING: &str = "zstd, br, gzip";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Connecting to a published site, TLS included.
const REMOTE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How footprint names itself to a published site.
const USER_AGENT: &str = concat!("cargo-rullst-footprint/", env!("CARGO_PKG_VERSION"));

/// What one benchmark run observed.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Outcome {
    /// Requests that received a complete response, any status.
    pub responses: u64,
    /// Connection failures, timeouts and truncated bodies.
    pub transport_errors: u64,
    /// Responses with a status of 400 or more.
    pub http_errors: u64,
    pub elapsed: Duration,
    /// Body bytes of the complete responses, as served.
    pub body_bytes: u64,
    /// The `Content-Encoding` of the complete responses (`identity` for
    /// none), sorted.
    pub encodings: BTreeSet<String>,
}

/// A client that never uses a proxy or follows redirects, so requests stay
/// on the validated loopback origin, and never decompresses a body, so the
/// bytes counted are the bytes served.
pub(super) fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .build()
}

/// The client that weighs a published site's `https://` page: like
/// [`client`] it follows no redirect and never decompresses a body, so it
/// counts the bytes the site sends; it refuses plain `http://`, uses the
/// system's proxy settings and names itself in `User-Agent`.
pub(super) fn remote_client() -> reqwest::Result<reqwest::Client> {
    remote_builder().https_only(true).build()
}

/// [`remote_client`] without the HTTPS-only rule, which loopback tests need.
fn remote_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_zstd()
        .no_deflate()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(REMOTE_CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
}

/// The response's `Content-Encoding`, `identity` when it has none.
pub(super) fn encoding(response: &reqwest::Response) -> String {
    response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "identity".to_string())
}

async fn worker(client: reqwest::Client, url: reqwest::Url, deadline: Instant) -> Outcome {
    let mut tally = Outcome::default();
    while Instant::now() < deadline {
        let result = async {
            let mut response = client
                .get(url.clone())
                .header(reqwest::header::ACCEPT_ENCODING, ACCEPT_ENCODING)
                .send()
                .await?;
            let status = response.status();
            let encoding = encoding(&response);
            let mut bytes = 0_u64;
            while let Some(chunk) = response.chunk().await? {
                bytes += chunk.len() as u64;
            }
            Ok::<_, reqwest::Error>((status, encoding, bytes))
        }
        .await;
        match result {
            Ok((status, encoding, bytes)) => {
                tally.responses += 1;
                tally.body_bytes += bytes;
                tally.encodings.insert(encoding);
                if status.as_u16() >= 400 {
                    tally.http_errors += 1;
                }
            }
            Err(_) => {
                tally.transport_errors += 1;
                // A refused connection fails instantly; do not spin.
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    tally
}

/// Runs the load and waits for in-flight requests to finish.
pub(super) async fn run(
    client: &reqwest::Client,
    url: &reqwest::Url,
    duration: Duration,
    concurrency: u16,
) -> Outcome {
    let workers = usize::from(concurrency.max(1));
    let started = Instant::now();
    let deadline = started + duration;
    let handles: Vec<_> = (0..workers)
        .map(|_| tokio::spawn(worker(client.clone(), url.clone(), deadline)))
        .collect();
    let mut outcome = Outcome::default();
    for handle in handles {
        let Ok(tally) = handle.await else {
            continue;
        };
        outcome.responses += tally.responses;
        outcome.transport_errors += tally.transport_errors;
        outcome.http_errors += tally.http_errors;
        outcome.body_bytes += tally.body_bytes;
        outcome.encodings.extend(tally.encodings);
    }
    outcome.elapsed = started.elapsed();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generators::footprint::tests::support::TestServer;

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn the_client_never_follows_a_redirect_off_the_measured_path() {
        let server = TestServer::start();
        let status = block_on(async {
            client()
                .unwrap()
                .get(server.url("/redirect"))
                .send()
                .await
                .unwrap()
                .status()
        });
        assert_eq!(status.as_u16(), 302);
        assert_eq!(server.paths(), ["/redirect"]);
    }

    #[test]
    fn the_remote_client_counts_like_the_local_one_and_refuses_plain_http() {
        let server = TestServer::start();
        let refused = block_on(remote_client().unwrap().get(server.url("/")).send());
        assert!(refused.is_err(), "{refused:?}");
        assert!(server.paths().is_empty());
        let client = remote_builder().build().unwrap();
        let (status, body) = block_on(async {
            let redirect = client.get(server.url("/redirect")).send().await.unwrap();
            let compressed = client.get(server.url("/compressed")).send().await.unwrap();
            (redirect.status(), compressed.bytes().await.unwrap())
        });
        assert_eq!(status.as_u16(), 302);
        // The five bytes labelled `br` are not decoded.
        assert_eq!(body.len(), 5);
        assert_eq!(server.paths(), ["/redirect", "/compressed"]);
        let agent = format!("cargo-rullst-footprint/{}", env!("CARGO_PKG_VERSION"));
        for request in server.requests() {
            assert_eq!(request.header("user-agent"), Some(agent.as_str()));
        }
    }

    #[test]
    fn every_connection_keeps_sending_until_the_duration_ends() {
        let server = TestServer::start();
        let client = client().unwrap();
        let duration = Duration::from_millis(300);
        let outcome = block_on(run(&client, &server.url("/"), duration, 2));
        assert!(outcome.responses >= 2, "{outcome:?}");
        assert_eq!((outcome.http_errors, outcome.transport_errors), (0, 0));
        assert!(outcome.elapsed >= duration, "{outcome:?}");
        assert!(server.paths().iter().all(|path| path == "/"));
        // Each "ok" body is two bytes; nothing was compressed.
        assert_eq!(outcome.body_bytes, 2 * outcome.responses, "{outcome:?}");
        assert_eq!(outcome.encodings.iter().collect::<Vec<_>>(), ["identity"]);
        for request in server.requests() {
            assert_eq!(request.header("accept-encoding"), Some(ACCEPT_ENCODING));
        }
    }

    #[test]
    fn compressed_bodies_are_counted_as_served() {
        let server = TestServer::start();
        let client = client().unwrap();
        let duration = Duration::from_millis(200);
        let outcome = block_on(run(&client, &server.url("/compressed"), duration, 1));
        assert!(outcome.responses >= 1, "{outcome:?}");
        // The five bytes labelled `br` are not decoded.
        assert_eq!(outcome.body_bytes, 5 * outcome.responses, "{outcome:?}");
        assert_eq!(outcome.encodings.iter().collect::<Vec<_>>(), ["br"]);
    }

    #[test]
    fn error_statuses_and_failed_connections_are_counted_apart() {
        let server = TestServer::start();
        let client = client().unwrap();
        let duration = Duration::from_millis(200);
        let missing = block_on(run(&client, &server.url("/missing"), duration, 2));
        assert!(missing.responses >= 2, "{missing:?}");
        assert_eq!(missing.http_errors, missing.responses);
        assert_eq!(missing.transport_errors, 0);

        assert_eq!(missing.body_bytes, 7 * missing.responses);

        let hangup = block_on(run(&client, &server.url("/hangup"), duration, 2));
        assert_eq!((hangup.responses, hangup.http_errors), (0, 0));
        assert!(hangup.transport_errors >= 2, "{hangup:?}");
        assert_eq!(hangup.body_bytes, 0);
        assert!(hangup.encodings.is_empty());
    }
}
