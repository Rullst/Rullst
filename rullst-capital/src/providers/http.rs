use crate::{CapitalError, ProviderFailure};
use reqwest::{RequestBuilder, Response, redirect::Policy};
use serde::de::DeserializeOwned;
use std::{sync::OnceLock, time::Duration};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_PROVIDER_URL_BYTES: usize = 2048;

static HTTP_CLIENT: OnceLock<Result<reqwest::Client, CapitalError>> = OnceLock::new();

pub(crate) fn client() -> Result<&'static reqwest::Client, CapitalError> {
    match HTTP_CLIENT.get_or_init(build_client) {
        Ok(client) => Ok(client),
        Err(error) => Err(error.clone()),
    }
}

fn build_client() -> Result<reqwest::Client, CapitalError> {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .build()
        .map_err(|_| {
            CapitalError::ConfigurationError(
                "failed to construct the bounded payment-provider HTTP client".to_string(),
            )
        })
}

pub(crate) async fn send(
    request: RequestBuilder,
    provider: &'static str,
    operation: &'static str,
) -> Result<Response, CapitalError> {
    let request = request
        .build()
        .map_err(|_| ProviderFailure::request_build(provider, operation))?;
    execute(request, provider, operation).await
}

pub(crate) async fn execute(
    request: reqwest::Request,
    provider: &'static str,
    operation: &'static str,
) -> Result<Response, CapitalError> {
    let response = client()?
        .execute(request)
        .await
        .map_err(|_| ProviderFailure::transport(provider, operation))?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_retry_after);
    Err(ProviderFailure::http_response(provider, operation, status.as_u16(), retry_after).into())
}

pub(crate) async fn send_json<T: DeserializeOwned>(
    request: RequestBuilder,
    provider: &'static str,
    operation: &'static str,
) -> Result<T, CapitalError> {
    let response = send(request, provider, operation).await?;
    read_json(response, provider, operation).await
}

pub(crate) async fn read_json<T: DeserializeOwned>(
    mut response: Response,
    provider: &'static str,
    operation: &'static str,
) -> Result<T, CapitalError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ProviderFailure::response_too_large(provider, operation).into());
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ProviderFailure::transport(provider, operation))?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(ProviderFailure::response_too_large(provider, operation).into());
        }
        body.extend_from_slice(&chunk);
    }

    serde_json::from_slice(&body)
        .map_err(|_| ProviderFailure::invalid_response(provider, operation).into())
}

pub(crate) fn validate_checkout_url(
    provider: &'static str,
    value: &str,
) -> Result<String, CapitalError> {
    let parsed = if value.len() <= MAX_PROVIDER_URL_BYTES {
        reqwest::Url::parse(value).ok()
    } else {
        None
    };
    let valid = parsed.as_ref().is_some_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
    });
    if !valid {
        return Err(ProviderFailure::contract_mismatch(provider, "create checkout").into());
    }
    Ok(value.to_string())
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
        .map(|duration| duration.min(MAX_RETRY_AFTER))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProviderFailureClass, ProviderFailureKind};

    #[test]
    fn retry_after_accepts_only_bounded_delta_seconds() {
        assert_eq!(parse_retry_after(" 30 "), Some(Duration::from_secs(30)));
        assert_eq!(
            parse_retry_after("999999999"),
            Some(Duration::from_secs(24 * 60 * 60))
        );
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
        assert_eq!(parse_retry_after("-1"), None);
    }

    #[test]
    fn checkout_urls_require_bounded_credential_free_https() {
        assert_eq!(
            validate_checkout_url("stripe", "https://checkout.example/session?id=1")
                .expect("valid checkout URL"),
            "https://checkout.example/session?id=1"
        );
        for invalid in [
            "http://checkout.example/session",
            "https://user:pass@checkout.example/session",
            "https://checkout.example/session#secret",
            "javascript:alert(1)",
            "/relative",
        ] {
            assert!(validate_checkout_url("stripe", invalid).is_err());
        }
        assert!(
            validate_checkout_url(
                "stripe",
                &format!(
                    "https://checkout.example/{}",
                    "x".repeat(MAX_PROVIDER_URL_BYTES)
                )
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn malformed_request_is_permanent_and_redacted_before_transport() {
        let client = build_client().expect("bounded client");
        let error = send(
            client
                .get("https://example.invalid/?token=url-secret")
                .header("x-fixture", "header\nsecret"),
            "fixture",
            "checkout",
        )
        .await
        .expect_err("invalid header must stop request construction");
        let failure = match error {
            CapitalError::Provider(failure) => failure,
            other => panic!("expected provider failure, received {other}"),
        };
        assert_eq!(failure.kind(), ProviderFailureKind::RequestBuild);
        assert_eq!(failure.class(), ProviderFailureClass::Permanent);
        let evidence = format!("{failure:?} {failure}");
        assert!(!evidence.contains("url-secret"));
        assert!(!evidence.contains("header"));
    }

    #[cfg(feature = "axum")]
    mod live_fixture {
        use super::*;
        use axum::{
            Router,
            body::Body,
            http::{HeaderValue, Response as AxumResponse, StatusCode},
            routing::get,
        };
        use serde_json::Value;

        async fn start_fixture() -> (String, tokio::task::JoinHandle<()>) {
            let app = Router::new()
                .route(
                    "/redirect",
                    get(|| async {
                        AxumResponse::builder()
                            .status(StatusCode::FOUND)
                            .header("location", "/success")
                            .body(Body::empty())
                            .expect("valid fixture response")
                    }),
                )
                .route(
                    "/success",
                    get(|| async { axum::Json(serde_json::json!({"ok": true})) }),
                )
                .route(
                    "/limited",
                    get(|| async {
                        let mut response = AxumResponse::new(Body::empty());
                        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                        response
                            .headers_mut()
                            .insert("retry-after", HeaderValue::from_static("999999"));
                        response
                    }),
                )
                .route(
                    "/unavailable",
                    get(|| async { StatusCode::SERVICE_UNAVAILABLE }),
                )
                .route(
                    "/rejected",
                    get(|| async { StatusCode::UNPROCESSABLE_ENTITY }),
                )
                .route("/malformed", get(|| async { "not-json" }))
                .route(
                    "/oversized",
                    get(|| async { "x".repeat(MAX_RESPONSE_BYTES + 1) }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind provider fixture");
            let address = listener.local_addr().expect("fixture address");
            let server = tokio::spawn(async move {
                axum::serve(listener, app)
                    .await
                    .expect("serve provider fixture");
            });
            (format!("http://{address}"), server)
        }

        fn provider_failure(error: CapitalError) -> crate::ProviderFailure {
            match error {
                CapitalError::Provider(failure) => failure,
                other => panic!("expected provider failure, received {other}"),
            }
        }

        #[tokio::test]
        async fn status_contract_is_redacted_classified_and_does_not_follow_redirects() {
            let (base, server) = start_fixture().await;
            let client = build_client().expect("bounded client");

            let redirect = provider_failure(
                send(
                    client.get(format!("{base}/redirect")),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("redirect must not be followed"),
            );
            assert_eq!(redirect.status(), Some(302));
            assert_eq!(redirect.class(), ProviderFailureClass::Permanent);

            let limited = provider_failure(
                send(client.get(format!("{base}/limited")), "fixture", "checkout")
                    .await
                    .expect_err("rate limit must fail"),
            );
            assert_eq!(limited.class(), ProviderFailureClass::RateLimited);
            assert_eq!(limited.retry_after(), Some(MAX_RETRY_AFTER));

            let unavailable = provider_failure(
                send(
                    client.get(format!("{base}/unavailable")),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("unavailability must fail"),
            );
            assert_eq!(unavailable.class(), ProviderFailureClass::Transient);

            let rejected = provider_failure(
                send(
                    client.get(format!("{base}/rejected")),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("rejection must fail"),
            );
            assert_eq!(rejected.class(), ProviderFailureClass::Permanent);
            assert!(!rejected.to_string().contains(&base));
            server.abort();
        }

        #[tokio::test]
        async fn transport_failure_does_not_expose_request_secrets() {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind disconnect fixture");
            let address = listener.local_addr().expect("disconnect fixture address");
            let disconnect = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("accept one request");
                drop(stream);
            });
            let client = build_client().expect("bounded client");
            let failure = provider_failure(
                send(
                    client
                        .get(format!("http://{address}/?token=url-secret"))
                        .bearer_auth("header-secret"),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("disconnected transport must fail"),
            );
            disconnect.await.expect("disconnect fixture task");

            assert_eq!(failure.kind(), ProviderFailureKind::Transport);
            assert_eq!(failure.class(), ProviderFailureClass::Transient);
            let evidence = format!("{failure:?} {failure}");
            assert!(!evidence.contains("url-secret"));
            assert!(!evidence.contains("header-secret"));
            assert!(!evidence.contains(&address.to_string()));
        }

        #[tokio::test]
        async fn successful_json_is_bounded_and_typed() {
            let (base, server) = start_fixture().await;
            let client = build_client().expect("bounded client");
            let success: Value =
                send_json(client.get(format!("{base}/success")), "fixture", "checkout")
                    .await
                    .expect("bounded JSON");
            assert_eq!(success["ok"], true);

            let malformed = provider_failure(
                send_json::<Value>(
                    client.get(format!("{base}/malformed")),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("malformed JSON must fail"),
            );
            assert_eq!(malformed.kind(), ProviderFailureKind::InvalidResponse);

            let oversized = provider_failure(
                send_json::<Value>(
                    client.get(format!("{base}/oversized")),
                    "fixture",
                    "checkout",
                )
                .await
                .expect_err("oversized JSON must fail"),
            );
            assert_eq!(oversized.kind(), ProviderFailureKind::ResponseTooLarge);
            server.abort();
        }
    }
}
