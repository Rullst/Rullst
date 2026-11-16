//! The request nonce is drawn on first use: by a handler that reads it, or by
//! the header layer for a document's policy, never for other responses.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::headers::{CspNonce, headers_middleware};
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, header},
    response::{Html, Json},
    routing::get,
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type Seen = Arc<Mutex<Vec<CspNonce>>>;

/// Routes that keep the request nonce without reading it, or read it, and
/// answer JSON or HTML.
fn app(seen: &Seen) -> Router {
    let (json, page, read) = (seen.clone(), seen.clone(), seen.clone());
    Router::new()
        .route(
            "/json",
            get(move |Extension(nonce): Extension<CspNonce>| async move {
                json.lock().unwrap().push(nonce);
                Json(["ok"])
            }),
        )
        .route(
            "/page",
            get(move |Extension(nonce): Extension<CspNonce>| async move {
                page.lock().unwrap().push(nonce);
                Html("<p>page</p>")
            }),
        )
        .route(
            "/json-reading-the-nonce",
            get(move |Extension(nonce): Extension<CspNonce>| async move {
                let value = nonce.as_str().to_owned();
                read.lock().unwrap().push(nonce);
                Json([value])
            }),
        )
        .layer(axum::middleware::from_fn(headers_middleware))
}

/// The policy of `path`, and the nonce its handler kept.
async fn policy_and_nonce(path: &str) -> (String, CspNonce) {
    let seen = Seen::default();
    let response = app(&seen)
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let policy = response.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap()
        .to_owned();
    let nonce = seen.lock().unwrap().pop().expect("the handler ran");
    (policy, nonce)
}

#[tokio::test]
async fn a_response_that_is_not_a_document_draws_no_nonce() {
    let (policy, nonce) = policy_and_nonce("/json").await;
    assert!(!nonce.is_drawn());
    assert!(!policy.contains("'nonce-"), "{policy}");
}

#[tokio::test]
async fn a_document_draws_the_nonce_its_policy_and_handler_share() {
    let (policy, nonce) = policy_and_nonce("/page").await;
    assert!(nonce.is_drawn());
    assert!(
        policy.contains(&format!("'nonce-{}'", nonce.as_str())),
        "{policy}"
    );
}

#[tokio::test]
async fn a_handler_that_reads_the_nonce_draws_it() {
    let (policy, nonce) = policy_and_nonce("/json-reading-the-nonce").await;
    assert!(nonce.is_drawn());
    // Still not a document: the policy carries no nonce.
    assert!(!policy.contains("'nonce-"), "{policy}");
}

#[tokio::test]
async fn every_request_gets_its_own_nonce() {
    let seen = Seen::default();
    let app = app(&seen);
    for _ in 0..2 {
        app.clone()
            .oneshot(Request::get("/page").body(Body::empty()).unwrap())
            .await
            .unwrap();
    }
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_ne!(seen[0].as_str(), seen[1].as_str());
}

#[test]
fn layers_of_one_request_share_one_value() {
    let mut extensions = axum::http::Extensions::new();
    let outer = CspNonce::get_or_insert(&mut extensions);
    let inner = CspNonce::get_or_insert(&mut extensions);
    assert!(!outer.is_drawn());
    let value = inner.as_str().to_owned();
    assert!(outer.is_drawn());
    assert_eq!(outer.as_str(), value);
    assert_eq!(extensions.get::<CspNonce>().unwrap().as_str(), value);
}

#[test]
fn nonces_compare_and_print_by_value() {
    let nonce = CspNonce::generate();
    assert!(nonce.is_drawn());
    assert_eq!(nonce, nonce.clone());
    assert_ne!(nonce, CspNonce::generate());
    assert_eq!(
        format!("{nonce:?}"),
        format!("CspNonce({:?})", nonce.as_str())
    );
    // Two undrawn nonces are drawn to be compared, so they differ.
    let mut first = axum::http::Extensions::new();
    let mut second = axum::http::Extensions::new();
    assert_ne!(
        CspNonce::get_or_insert(&mut first),
        CspNonce::get_or_insert(&mut second)
    );
}
