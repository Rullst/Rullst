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

async fn status(app: AxumRouter, uri: &str) -> StatusCode {
    app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn borrowed_inner_routers_are_the_configured_router() {
    let mut router = Router::new().route("/ready", get(handler));
    assert_eq!(
        status(router.as_axum().clone(), "/ready").await,
        StatusCode::OK
    );
    assert_eq!(
        status(std::ops::Deref::deref(&router).clone(), "/ready").await,
        StatusCode::OK
    );

    let added = std::mem::take(router.as_axum_mut()).route("/added", get(handler));
    *router.as_axum_mut() = added;
    assert_eq!(
        status(router.as_axum().clone(), "/added").await,
        StatusCode::OK
    );
    let replaced =
        std::mem::take(std::ops::DerefMut::deref_mut(&mut router)).route("/replaced", get(handler));
    *std::ops::DerefMut::deref_mut(&mut router) = replaced;
    let app = router.into_axum();
    assert_eq!(status(app.clone(), "/replaced").await, StatusCode::OK);
    assert_eq!(status(app, "/ready").await, StatusCode::OK);
}

#[tokio::test]
async fn the_fallback_answers_unmatched_paths() {
    let app = Router::new()
        .route("/ready", get(handler))
        .fallback(|| async { (StatusCode::IM_A_TEAPOT, "fallback") })
        .into_axum();
    assert_eq!(
        status(app.clone(), "/missing").await,
        StatusCode::IM_A_TEAPOT
    );
    assert_eq!(status(app, "/ready").await, StatusCode::OK);
}
