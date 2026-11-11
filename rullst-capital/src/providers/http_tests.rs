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
    let stripe_url = "https://checkout.stripe.com/c/pay/cs_test_fixture#opaque-state";
    assert_eq!(
        validate_checkout_url("stripe", stripe_url).unwrap(),
        stripe_url
    );
    assert!(validate_checkout_url("infinitepay", stripe_url).is_err());
    assert_eq!(
        validate_checkout_url("stripe", "https://checkout.example/session?id=1")
            .expect("valid checkout URL"),
        "https://checkout.example/session?id=1"
    );
    for invalid in [
        "https://checkout.example/a b",
        "http://checkout.example/session",
        "https://user:pass@checkout.example/session",
        "\nhttps://checkout.example/session",
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

#[test]
fn redirects_accept_loopback_http_until_a_live_dispatch() {
    for local in [
        "http://localhost:3000/paid",
        "http://LOCALHOST/paid?order=1",
        "http://127.0.0.1:8080/paid",
        "http://[::1]:3000/paid",
        "https://app.example/paid",
    ] {
        validate_redirect_url(local).expect(local);
    }
    for invalid in [
        "http://app.example/paid",
        "http://localhost.example/paid",
        "http://10.0.0.1/paid",
        "https://app.example/paid#done",
        "https://user:pass@app.example/paid",
        "https://user@app.example/paid",
        "https://:pass@app.example/paid",
        "ftp://localhost/paid",
        "/paid",
    ] {
        assert!(
            matches!(
                validate_redirect_url(invalid),
                Err(CapitalError::ConfigurationError(_))
            ),
            "{invalid}"
        );
    }
    let longest = format!("https://app.example/{}", "x".repeat(2048 - 20));
    assert_eq!(longest.len(), 2048);
    validate_redirect_url(&longest).expect("2048 bytes");
    assert!(validate_redirect_url(&format!("{longest}x")).is_err());
    for invalid in ["https://app.example/a b", "https://app.example/\u{1}"] {
        assert!(validate_redirect_url(invalid).is_err(), "{invalid}");
    }
    require_https_redirects(&["https://app.example/a", "https://app.example/b"])
        .expect("HTTPS redirects");
    for urls in [
        ["http://localhost:3000/a", "https://app.example/b"],
        ["https://app.example/a", "http://127.0.0.1/b"],
    ] {
        assert!(matches!(
            require_https_redirects(&urls),
            Err(CapitalError::ConfigurationError(_))
        ));
    }
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
            )
            .route(
                "/exact",
                // A JSON string of exactly 1 MiB, quotes included.
                get(|| async { format!("\"{}\"", "x".repeat(1_048_574)) }),
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

        let exact: String = send_json(client.get(format!("{base}/exact")), "fixture", "checkout")
            .await
            .expect("a 1 MiB response is within the limit");
        assert_eq!(exact.len(), 1_048_574);
        server.abort();
    }
}
