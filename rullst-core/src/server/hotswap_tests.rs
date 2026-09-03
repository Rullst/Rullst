#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex, RwLock};
use std::task::Poll;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower_service::Service;

use super::*;

const TEST_RELOAD_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn service(router: axum::Router) -> HotSwapService {
    HotSwapService {
        current_router: Arc::new(RwLock::new(router)),
        active_libraries: Arc::new(Mutex::new(Vec::new())),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from(TEST_RELOAD_TOKEN),
        lib_path: "/definitely/missing/rullst-app".to_owned(),
        is_dev: true,
        shield: None,
        limiter: None,
        lifecycle: None,
    }
}

async fn body_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("bounded response body");
    String::from_utf8(body.to_vec()).expect("UTF-8 fixture response")
}

#[tokio::test]
async fn forwards_normal_requests_and_reports_missing_reload_library() {
    let router = axum::Router::new().route("/ok", axum::routing::get(|| async { "ready" }));
    let mut service = service(router);

    let response = service
        .call(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "ready");

    let response = service
        .call(
            Request::builder()
                .method("POST")
                .uri("/_rullst/internal/reload_dylib")
                .header(RELOAD_TOKEN_HEADER, TEST_RELOAD_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body_text(response).await.contains("Dylib not found"));
    assert!(service.active_libraries.lock().unwrap().is_empty());
}

#[tokio::test]
async fn reload_endpoint_rejects_missing_and_incorrect_tokens() {
    for token in [None, Some("incorrect-token")] {
        let mut service = service(axum::Router::new());
        let mut request = Request::builder()
            .method("POST")
            .uri("/_rullst/internal/reload_dylib");
        if let Some(token) = token {
            request = request.header(RELOAD_TOKEN_HEADER, token);
        }
        let response = service
            .call(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(body_text(response).await.is_empty());
    }
}

#[tokio::test]
async fn hmr_channel_is_reserved_and_requires_a_websocket_upgrade() {
    let router = axum::Router::new().route(
        "/_rullst_hmr",
        axum::routing::get(|| async { "application shadow" }),
    );
    let mut service = service(router);
    let response = service
        .call(
            Request::builder()
                .uri("/_rullst_hmr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_client_error());
    assert!(!body_text(response).await.contains("application shadow"));
}

#[test]
fn retained_library_generation_limit_is_bounded() {
    assert!(!generation_limit_reached(MAX_HOT_RELOAD_GENERATIONS - 1));
    assert!(generation_limit_reached(MAX_HOT_RELOAD_GENERATIONS));
}

#[tokio::test]
async fn poisoned_router_lock_recovers_and_keeps_serving() {
    let router = axum::Router::new().route("/health", axum::routing::get(|| async { "healthy" }));
    let mut service = service(router);
    let current = Arc::clone(&service.current_router);
    let _ = std::thread::spawn(move || {
        let _guard = current.write().unwrap();
        panic!("poison test-only router lock");
    })
    .join();

    let response = service
        .call(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "healthy");
}

#[tokio::test]
async fn panic_and_cancellation_errors_become_bounded_error_responses() {
    let string_panic = tokio::spawn(async {
        std::panic::panic_any(String::from("owned panic message"));
    })
    .await
    .unwrap_err();
    let response = HotSwapService::handle_panic_error(string_panic)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body_text(response).await.contains("owned panic message"));

    let opaque_panic = tokio::spawn(async { std::panic::panic_any(7_u8) })
        .await
        .unwrap_err();
    let response = HotSwapService::handle_panic_error(opaque_panic)
        .await
        .unwrap();
    assert!(
        body_text(response)
            .await
            .contains("Unhandled application panic")
    );

    let cancelled = tokio::spawn(std::future::pending::<()>());
    cancelled.abort();
    let response = HotSwapService::handle_panic_error(cancelled.await.unwrap_err())
        .await
        .unwrap();
    assert!(
        body_text(response)
            .await
            .contains("Request task was cancelled or aborted")
    );
}

#[tokio::test]
async fn service_readiness_and_fallback_error_response_are_infallible() {
    let mut service = service(axum::Router::new());
    let readiness = std::future::poll_fn(|context| {
        let result =
            <HotSwapService as Service<axum::extract::Request>>::poll_ready(&mut service, context);
        match result {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => Poll::Pending,
        }
    })
    .await;
    assert!(readiness.is_ok());

    let response = HotSwapService::handle_oneshot_error().unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body_text(response).await.is_empty());
}

#[tokio::test]
async fn application_lifecycle_also_gates_hot_swapped_routes() {
    let router = axum::Router::new().route("/ok", axum::routing::get(|| async { "ready" }));
    let mut service = service(router);
    let lifecycle = crate::lifecycle::ApplicationLifecycle::new();
    service.lifecycle = Some(lifecycle.clone());

    let starting = service
        .call(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(starting.status(), StatusCode::SERVICE_UNAVAILABLE);

    lifecycle.mark_ready().unwrap();
    let ready = service
        .call(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(ready.status(), StatusCode::OK);

    lifecycle.begin_draining().unwrap();
    let draining = service
        .call(Request::builder().uri("/ok").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(draining.status(), StatusCode::SERVICE_UNAVAILABLE);
}
