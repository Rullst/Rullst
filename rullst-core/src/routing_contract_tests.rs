#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::convert::Infallible;
use tower::ServiceExt;

async fn handler() -> &'static str {
    "ok"
}

#[tokio::test]
async fn defaults_deref_and_free_websocket_wrapper_interoperate_with_axum() {
    let mut router = Router::default();
    let _: &AxumRouter = std::ops::Deref::deref(&router);
    let _: &mut AxumRouter = std::ops::DerefMut::deref_mut(&mut router);

    let app = router.route("/socket", ws(handler)).into_axum();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/socket")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible router");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn fallback_service_handles_unmatched_requests() {
    let service = tower::service_fn(|_request: axum::extract::Request| async {
        Ok::<_, Infallible>(
            axum::http::Response::builder()
                .status(StatusCode::IM_A_TEAPOT)
                .body(Body::from("fallback service"))
                .expect("valid response"),
        )
    });
    let app = Router::new().fallback_service(service).into_axum();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .expect("valid request"),
        )
        .await
        .expect("infallible router");
    assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
}
