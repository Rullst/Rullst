//! Buffering limits of the PII middleware and the edges of e-mail detection.

#![allow(clippy::expect_used)]

use super::{mask_pii, pii_masking_middleware};
use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderValue, Request, StatusCode, header},
    response::Response,
    routing::get,
};
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::ServiceExt;

/// Serves the response `build` makes through the PII middleware.
async fn through_pii_layer(
    build: impl Fn() -> Response + Clone + Send + Sync + 'static,
) -> Response {
    let app = Router::new()
        .route(
            "/",
            get(move || {
                let build = build.clone();
                async move { build() }
            }),
        )
        .layer(axum::middleware::from_fn(pii_masking_middleware));
    app.oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

fn text_response(body: Bytes, extra: &[(header::HeaderName, &'static str)]) -> Response {
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    for (name, value) in extra {
        response
            .headers_mut()
            .insert(name.clone(), HeaderValue::from_static(value));
    }
    response
}

async fn body_bytes(response: Response) -> Bytes {
    axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .expect("body")
}

/// A text body of exactly `len` bytes: an address followed by spaces, so
/// every masking pass stays linear.
fn padded_address(len: usize) -> Bytes {
    let mut text = String::from("contact ana@example.com");
    text.push_str(&" ".repeat(len - text.len()));
    Bytes::from(text)
}

#[tokio::test]
async fn responses_up_to_two_mebibytes_are_masked_and_larger_ones_pass() {
    let exact = padded_address(2_097_152);
    let response = through_pii_layer(move || text_response(exact.clone(), &[])).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_LENGTH),
        Some(&HeaderValue::from_static("2097152"))
    );
    let body = body_bytes(response).await;
    assert_eq!(body.len(), 2_097_152);
    assert!(body.starts_with(b"contact a**@example.com "));

    // One byte more is not buffered, so it passes through unmasked.
    let over = padded_address(2_097_153);
    let body = body_bytes(through_pii_layer(move || text_response(over.clone(), &[])).await).await;
    assert_eq!(body.len(), 2_097_153);
    assert!(body.starts_with(b"contact ana@example.com "));
}

#[tokio::test]
async fn a_declared_content_length_must_match_the_body() {
    let text = Bytes::from_static(b"contact ana@example.com");
    let matching = text.clone();
    let response = through_pii_layer(move || {
        text_response(matching.clone(), &[(header::CONTENT_LENGTH, "23")])
    })
    .await;
    assert_eq!(
        response.headers().get(header::CONTENT_LENGTH),
        Some(&HeaderValue::from_static("23"))
    );
    assert_eq!(body_bytes(response).await, "contact a**@example.com");

    // A length that disagrees with the body is not buffered.
    let response =
        through_pii_layer(move || text_response(text.clone(), &[(header::CONTENT_LENGTH, "24")]))
            .await;
    assert_eq!(
        response.headers().get(header::CONTENT_LENGTH),
        Some(&HeaderValue::from_static("24"))
    );
}

#[tokio::test]
async fn only_identity_encoded_responses_are_masked() {
    let gzip = through_pii_layer(|| {
        text_response(
            Bytes::from_static(b"contact ana@example.com"),
            &[(header::CONTENT_ENCODING, "gzip")],
        )
    })
    .await;
    assert_eq!(body_bytes(gzip).await, "contact ana@example.com");

    let identity = through_pii_layer(|| {
        text_response(
            Bytes::from_static(b"contact ana@example.com"),
            &[(header::CONTENT_ENCODING, "Identity")],
        )
    })
    .await;
    assert_eq!(body_bytes(identity).await, "contact a**@example.com");
}

/// A body that announces its exact length and then fails.
struct FailingBody;

impl http_body::Body for FailingBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, std::io::Error>>> {
        Poll::Ready(Some(Err(std::io::Error::other("connection reset"))))
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(23)
    }
}

#[tokio::test]
async fn a_body_that_fails_while_buffering_is_withheld() {
    let response = through_pii_layer(|| {
        let mut response = Response::new(Body::new(FailingBody));
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        response
    })
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static("no-store"))
    );
    assert_eq!(body_bytes(response).await, "response masking failed");
}

#[test]
fn top_level_domains_must_be_at_least_two_letters() {
    for unchanged in [
        "mail ab@example.c today",
        "mail ab@example.c0m today",
        "mail ab@cdn.example.12 today",
    ] {
        assert_eq!(mask_pii(unchanged), unchanged);
    }
    assert_eq!(mask_pii("mail ab@example.co"), "mail a*@example.co");
}

#[test]
fn a_domain_needs_more_than_three_characters() {
    assert_eq!(mask_pii("mail ab@.io today"), "mail ab@.io today");
    assert_eq!(mask_pii("mail ab@a.io today"), "mail a*@a.io today");
}
