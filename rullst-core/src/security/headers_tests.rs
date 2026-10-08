//! Precedence of the Core security-header baseline over inner choices.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::headers::{CspNonce, SecurityHeadersApplied, headers_middleware};
use axum::{
    Extension, Router,
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Request, Response, header},
    routing::get,
};
use tower::ServiceExt;

/// Every security header the baseline adds, with a value it never uses.
const CUSTOM: &[(&str, &str)] = &[
    ("x-frame-options", "SAMEORIGIN"),
    ("x-content-type-options", "nosniff; custom"),
    ("x-xss-protection", "1"),
    ("strict-transport-security", "max-age=60"),
    ("permissions-policy", "camera=(self)"),
    ("cross-origin-opener-policy", "same-origin-allow-popups"),
    ("cross-origin-resource-policy", "cross-origin"),
    ("cross-origin-embedder-policy", "unsafe-none"),
    ("referrer-policy", "same-origin"),
    (
        "content-security-policy",
        "default-src 'self' https://cdn.example",
    ),
];

async fn through_baseline(headers: HeaderMap, marked: bool) -> Response<Body> {
    let app = Router::new()
        .route(
            "/",
            get(move || {
                let headers = headers.clone();
                async move {
                    let mut response = Response::new(Body::from("page"));
                    *response.headers_mut() = headers;
                    if marked {
                        response.extensions_mut().insert(SecurityHeadersApplied);
                    }
                    response
                }
            }),
        )
        .layer(axum::middleware::from_fn(headers_middleware));
    app.oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap()
}

fn custom_headers() -> HeaderMap {
    CUSTOM
        .iter()
        .map(|&(name, value)| {
            (
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            )
        })
        .collect()
}

#[tokio::test]
async fn explicit_inner_values_win_over_the_baseline() {
    for marked in [false, true] {
        let response = through_baseline(custom_headers(), marked).await;
        for (name, value) in CUSTOM {
            assert_eq!(
                response.headers().get(*name).unwrap(),
                value,
                "{name}, marked={marked}"
            );
            assert_eq!(response.headers().get_all(*name).iter().count(), 1);
        }
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
}

#[tokio::test]
async fn a_marked_response_keeps_the_omissions_of_its_layer() {
    let mut headers = HeaderMap::new();
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    let response = through_baseline(headers, true).await;
    assert_eq!(response.headers()[header::X_FRAME_OPTIONS], "DENY");
    for (name, _) in CUSTOM.iter().skip(1) {
        assert!(!response.headers().contains_key(*name), "{name} was added");
    }
    // The cache default is not a security header choice of the inner layer.
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
}

#[tokio::test]
async fn unmarked_responses_get_every_default() {
    let response = through_baseline(HeaderMap::new(), false).await;
    for (name, expected) in [
        ("x-frame-options", "DENY"),
        ("x-content-type-options", "nosniff"),
        ("x-xss-protection", "0"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains; preload",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=(), payment=(), usb=()",
        ),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
        ("cross-origin-embedder-policy", "require-corp"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        ("cache-control", "no-store"),
    ] {
        assert_eq!(response.headers()[name], expected, "{name}");
    }
    assert!(
        response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap()
            .contains("'nonce-")
    );
}

#[tokio::test]
async fn an_exact_no_referrer_among_handler_values_is_normalized() {
    let mut headers = HeaderMap::new();
    headers.append(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.append(
        header::REFERRER_POLICY,
        HeaderValue::from_static("unsafe-url"),
    );
    let response = through_baseline(headers, false).await;
    let values: Vec<_> = response
        .headers()
        .get_all(header::REFERRER_POLICY)
        .iter()
        .collect();
    assert_eq!(values, ["no-referrer"]);
}

#[tokio::test]
async fn a_marked_inner_csp_keeps_the_shared_nonce() {
    // An inner layer that renders its CSP from the request nonce, as SecureHeadersLayer does.
    let app = Router::new()
        .route(
            "/",
            get(|Extension(nonce): Extension<CspNonce>| async move {
                let mut response = Response::new(Body::from(nonce.to_string()));
                let policy = format!("script-src 'nonce-{nonce}'");
                response.headers_mut().insert(
                    header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_str(&policy).unwrap(),
                );
                response.extensions_mut().insert(SecurityHeadersApplied);
                response
            }),
        )
        .layer(axum::middleware::from_fn(headers_middleware));
    let response = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .to_owned();
    let body = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .unwrap();
    let nonce = std::str::from_utf8(&body).unwrap();
    assert_eq!(csp, format!("script-src 'nonce-{nonce}'"));
}
