//! Request-scheme, HTTP/2 and explicit-policy edge cases of the CSWSH guard.

use super::*;
use axum::{Router, body::Body, http::Request as HttpRequest, middleware, routing::any};
use tower::ServiceExt;

fn guarded_app() -> Router {
    Router::new()
        .route("/ws", any(|| async { "upgraded" }))
        .layer(middleware::from_fn(cswsh_guard_middleware))
}

async fn h2_connect(uri: &str, origin: &str) -> StatusCode {
    let request = HttpRequest::builder()
        .version(axum::http::Version::HTTP_2)
        .method(axum::http::Method::CONNECT)
        .uri(uri)
        .header(header::ORIGIN, origin)
        .body(Body::empty())
        .unwrap();
    guarded_app().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn an_http_request_scheme_must_match_an_http_origin() {
    assert_eq!(
        h2_connect("http://app.example.com/ws", "http://app.example.com").await,
        StatusCode::OK
    );
    assert_eq!(
        h2_connect("http://app.example.com/ws", "https://app.example.com").await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn a_scheme_other_than_http_or_https_adds_no_scheme_constraint() {
    assert_eq!(
        h2_connect("ws://app.example.com/ws", "https://app.example.com").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn ordinary_http2_requests_and_http1_connect_are_not_upgrades() {
    let h2_get = HttpRequest::builder()
        .version(axum::http::Version::HTTP_2)
        .uri("https://app.example.com/ws")
        .header(header::ORIGIN, "https://attacker.example")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        guarded_app().oneshot(h2_get).await.unwrap().status(),
        StatusCode::OK
    );
    let h1_connect = HttpRequest::builder()
        .method(axum::http::Method::CONNECT)
        .uri("/ws")
        .header(header::HOST, "app.example.com")
        .header(header::ORIGIN, "https://attacker.example")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        guarded_app().oneshot(h1_connect).await.unwrap().status(),
        StatusCode::OK
    );
}

fn upgrade_with_policy(origin: &str, policy: CswsPolicy) -> HttpRequest<Body> {
    let mut request = HttpRequest::builder()
        .uri("/ws")
        .header(header::UPGRADE, "websocket")
        .header(header::HOST, "app.example.com")
        .header(header::ORIGIN, origin)
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(policy);
    request
}

#[tokio::test]
async fn an_empty_allowlist_falls_back_to_the_same_host_check() {
    let policy = CswsPolicy::default().allow_missing_origin(true);
    let same_origin = upgrade_with_policy("http://app.example.com", policy.clone());
    assert_eq!(
        guarded_app().oneshot(same_origin).await.unwrap().status(),
        StatusCode::OK
    );
    let cross_origin = upgrade_with_policy("http://attacker.example", policy);
    assert_eq!(
        guarded_app().oneshot(cross_origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn allowing_a_missing_origin_does_not_admit_a_malformed_one() {
    let policy = CswsPolicy::default().allow_missing_origin(true);
    let malformed = upgrade_with_policy("not-an-origin", policy);
    assert_eq!(
        guarded_app().oneshot(malformed).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}
