// tests/cors_scaffold_tests.rs — Behavioral coverage for the generated CORS policy.

#![allow(clippy::unwrap_used, clippy::expect_used)]

extern crate self as rullst;

pub mod server {
    pub use axum::http::{HeaderValue, Method, Uri, header};
}

#[allow(dead_code)]
#[path = "../src/generators/cors_middleware.rs.template"]
mod generated_cors;

use axum::{Router, body::Body, http::Request, routing::get};
use generated_cors::{CorsConfig, CorsConfigError};
use server::{HeaderValue, header};
use tower::ServiceExt;

fn app(allow_credentials: bool) -> Router {
    let config = CorsConfig::new(["https://app.example.com", "http://localhost:3000"])
        .unwrap()
        .with_credentials(allow_credentials);

    Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(config.into_layer())
}

async fn request_with_origin(origin: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri("/");
    if let Some(origin) = origin {
        request = request.header(header::ORIGIN, origin);
    }

    app(false)
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn allowlisted_origin_receives_cors_headers_without_credentials() {
    let response = request_with_origin(Some("https://app.example.com")).await;

    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&HeaderValue::from_static("https://app.example.com"))
    );
    assert_eq!(
        response.headers().get(header::VARY),
        Some(&HeaderValue::from_static("origin"))
    );
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .is_none()
    );
}

#[tokio::test]
async fn unknown_and_absent_origins_are_not_authorized() {
    for origin in [Some("https://attacker.example"), None] {
        let response = request_with_origin(origin).await;
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none()
        );
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .is_none()
        );
    }
}

#[tokio::test]
async fn allowlisted_preflight_uses_the_explicit_policy() {
    let request = Request::builder()
        .method(server::Method::OPTIONS)
        .uri("/")
        .header(header::ORIGIN, "https://app.example.com")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
        .body(Body::empty())
        .unwrap();
    let response = app(false).oneshot(request).await.unwrap();

    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&HeaderValue::from_static("https://app.example.com"))
    );
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_METHODS)
            .is_some()
    );
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
            .is_some()
    );
    assert_eq!(
        response.headers().get(header::VARY),
        Some(&HeaderValue::from_static("origin"))
    );
    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .is_none()
    );
}

#[tokio::test]
async fn credentials_require_an_explicit_opt_in() {
    let request = Request::builder()
        .uri("/")
        .header(header::ORIGIN, "https://app.example.com")
        .body(Body::empty())
        .unwrap();
    let response = app(true).oneshot(request).await.unwrap();

    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS),
        Some(&HeaderValue::from_static("true"))
    );
}

#[test]
fn wildcard_empty_and_non_origin_values_fail_closed() {
    assert_eq!(
        CorsConfig::new(["*"]).unwrap_err(),
        CorsConfigError::WildcardOrigin
    );
    assert_eq!(
        CorsConfig::new(["", "  "]).unwrap_err(),
        CorsConfigError::EmptyAllowlist
    );
    assert!(matches!(
        CorsConfig::new(["https://app.example.com/path"]),
        Err(CorsConfigError::InvalidOrigin(_))
    ));
    assert!(matches!(
        CorsConfig::new(["not-an-origin"]),
        Err(CorsConfigError::InvalidOrigin(_))
    ));
}
