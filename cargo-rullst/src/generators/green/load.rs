//! The benchmark: a bounded closed-loop HTTP load in which `concurrency`
//! workers each send one GET at a time until the duration ends. Bodies are
//! drained, not stored.
use std::time::{Duration, Instant};

pub(super) const METHOD: &str = "closed loop: each connection sends the next GET only after the \
    previous response body was fully read";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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
}

/// A client that never uses a proxy or follows redirects, so requests stay
/// on the validated loopback origin.
pub(super) fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(REQUEST_TIMEOUT)
        .build()
}

async fn worker(client: reqwest::Client, url: reqwest::Url, deadline: Instant) -> Outcome {
    let mut tally = Outcome::default();
    while Instant::now() < deadline {
        let result = async {
            let mut response = client.get(url.clone()).send().await?;
            let status = response.status();
            while response.chunk().await?.is_some() {}
            Ok::<_, reqwest::Error>(status)
        }
        .await;
        match result {
            Ok(status) => {
                tally.responses += 1;
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
    }
    outcome.elapsed = started.elapsed();
    outcome
}
