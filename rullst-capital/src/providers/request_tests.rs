use super::*;
use crate::{ProviderFailureClass, ProviderFailureKind};
use axum::{
    Router,
    body::Bytes,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<(HeaderMap, Vec<u8>)>>>;

async fn fixture() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let record = |seen: Seen| {
        move |headers: HeaderMap, body: Bytes| async move {
            seen.lock().unwrap().push((headers, body.to_vec()));
            axum::Json(serde_json::json!({"ok": true}))
        }
    };
    let app = Router::new()
        .route("/json", post(record(seen.clone())))
        .route("/plain", get(|| async { "plain text" }))
        .route(
            "/redirect",
            get(|| async { (StatusCode::FOUND, [("location", "/plain")]) }),
        )
        .route(
            "/limited",
            get(|| async { (StatusCode::TOO_MANY_REQUESTS, [("retry-after", "7")]) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}"), seen)
}

fn failure(error: CapitalError) -> ProviderFailure {
    match error {
        CapitalError::Provider(failure) => failure,
        other => panic!("expected a provider failure, got {other}"),
    }
}

#[tokio::test]
async fn json_requests_carry_headers_and_decode_bounded_json() {
    let (base, seen) = fixture().await;
    let reply: serde_json::Value = ProviderRequest::post_json(
        "acmepay",
        "check",
        format!("{base}/json"),
        &serde_json::json!({"order": "order_42"}),
    )
    .unwrap()
    .bearer_auth("secret-key")
    .header("X-Api-Key", "inchk_fixture")
    .send_json()
    .await
    .unwrap();
    assert_eq!(reply["ok"], true);
    let form = ProviderRequest::post_form("acmepay", "form", format!("{base}/json"), "a=1&b=2")
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(form, br#"{"ok":true}"#);
    let seen = seen.lock().unwrap();
    let (headers, body) = &seen[0];
    assert_eq!(headers["authorization"], "Bearer secret-key");
    assert_eq!(headers["x-api-key"], "inchk_fixture");
    assert_eq!(headers["content-type"], "application/json");
    assert_eq!(body, br#"{"order":"order_42"}"#);
    assert_eq!(
        seen[1].0["content-type"],
        "application/x-www-form-urlencoded"
    );
    assert_eq!(seen[1].1, b"a=1&b=2");
}

#[tokio::test]
async fn failures_are_classified_redacted_and_redirects_are_not_followed() {
    let (base, _) = fixture().await;
    let plain = ProviderRequest::get("acmepay", "read", format!("{base}/plain"))
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(plain, b"plain text");
    let invalid = failure(
        ProviderRequest::get("acmepay", "read", format!("{base}/plain"))
            .unwrap()
            .send_json::<serde_json::Value>()
            .await
            .unwrap_err(),
    );
    assert_eq!(invalid.kind(), ProviderFailureKind::InvalidResponse);
    let redirect = failure(
        ProviderRequest::get(
            "acmepay",
            "read",
            format!("{base}/redirect?token=url-secret"),
        )
        .unwrap()
        .send()
        .await
        .unwrap_err(),
    );
    assert_eq!(redirect.status(), Some(302));
    assert!(!format!("{redirect:?} {redirect}").contains("url-secret"));
    let limited = failure(
        ProviderRequest::get("acmepay", "read", format!("{base}/limited"))
            .unwrap()
            .send()
            .await
            .unwrap_err(),
    );
    assert_eq!(limited.class(), ProviderFailureClass::RateLimited);
    assert_eq!(
        limited.retry_after(),
        Some(std::time::Duration::from_secs(7))
    );
    assert_eq!(limited.provider(), "acmepay");
    assert_eq!(limited.operation(), "read");
    let bad_header = failure(
        ProviderRequest::get("acmepay", "read", format!("{base}/plain"))
            .unwrap()
            .header("x-fixture", "header\nsecret")
            .send()
            .await
            .unwrap_err(),
    );
    assert_eq!(bad_header.kind(), ProviderFailureKind::RequestBuild);
}

#[test]
fn urls_must_be_https_or_loopback_http_without_credentials() {
    for url in [
        "https://api.acme.example/v1",
        "http://127.0.0.1:9000/v1",
        "http://localhost/v1",
    ] {
        assert!(
            ProviderRequest::get("acmepay", "read", url).is_ok(),
            "{url}"
        );
    }
    let longest = format!("https://api.acme.example/{}", "x".repeat(2048 - 25));
    assert_eq!(longest.len(), 2048);
    assert!(ProviderRequest::get("acmepay", "read", longest.as_str()).is_ok());
    assert!(ProviderRequest::get("acmepay", "read", format!("{longest}x")).is_err());
    let exact = "x".repeat(1024 * 1024);
    assert!(
        ProviderRequest::post_form("acmepay", "form", "https://api.acme.example", exact).is_ok()
    );
    for url in [
        "http://api.acme.example/v1",
        "https://user:pass@api.acme.example/v1",
        "https://user@api.acme.example/v1",
        "https://:pass@api.acme.example/v1",
        "ftp://api.acme.example/v1",
        "/relative",
    ] {
        let error = ProviderRequest::get("acmepay", "read", url).unwrap_err();
        assert_eq!(
            failure(error).kind(),
            ProviderFailureKind::RequestBuild,
            "{url}"
        );
    }
    let oversized = "x".repeat(1024 * 1024 + 1);
    assert!(
        ProviderRequest::post_form("acmepay", "form", "https://api.acme.example", oversized)
            .is_err()
    );
    let debug = format!(
        "{:?}",
        ProviderRequest::get(
            "acmepay",
            "read",
            "https://api.acme.example/v1?token=secret"
        )
        .unwrap()
        .bearer_auth("secret")
    );
    assert!(!debug.contains("secret") && debug.contains("acmepay"));
}
