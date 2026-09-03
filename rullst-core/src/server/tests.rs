#![cfg(test)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Router;
use crate::scheduler::Scheduler;
use crate::server::hotswap::HotSwapService;
use crate::server::server_middleware::{hmr_client_script, inject_hmr_script};
use std::sync::{Arc, Mutex, RwLock};

const TEST_RELOAD_TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn test_server_builder() {
    let router = Router::new();
    let server = Server::new(router).with_db("sqlite://test.db");

    assert_eq!(server.db_url, Some("sqlite://test.db".to_string()));
    assert!(server.scheduler.is_none());
}

#[test]
fn test_server_scheduler_attach() {
    let router = Router::new();
    let scheduler = Scheduler::new();
    let server = Server::new(router).schedule(scheduler);

    assert!(server.scheduler.is_some());
}

#[tokio::test]
async fn test_server_resilience_attach() {
    let router = Router::new();
    let shield = crate::resilience::TrafficShield::new(
        crate::resilience::TrafficShieldConfig::new().with_db_probe(false),
    );
    let limiter =
        crate::resilience::RateLimiter::new(crate::resilience::RateLimitConfig::per_second(10.0));
    let server = Server::new(router).shield(shield).rate_limit(limiter);

    assert!(server.shield.is_some());
    assert!(server.limiter.is_some());
}

#[tokio::test]
async fn test_hot_swap_service_call() {
    let router = axum::Router::new().route("/test", axum::routing::get(|| async { "swap ok" }));
    let current_router = Arc::new(RwLock::new(router));
    let mut service = HotSwapService {
        current_router,
        active_libraries: Arc::new(Mutex::new(vec![])),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from(TEST_RELOAD_TOKEN),
        lib_path: "".to_string(),
        is_dev: false,
        shield: None,
        limiter: None,
        lifecycle: None,
    };

    use tower_service::Service;
    let req = axum::http::Request::builder()
        .uri("/test")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(body_bytes, "swap ok");
}

#[tokio::test]
async fn test_hot_swap_service_panic() {
    async fn panic_handler() -> &'static str {
        panic!("Oops");
    }
    let router = axum::Router::new().route("/panic", axum::routing::get(panic_handler));
    let current_router = Arc::new(RwLock::new(router));
    let mut service = HotSwapService {
        current_router,
        active_libraries: Arc::new(Mutex::new(vec![])),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from(TEST_RELOAD_TOKEN),
        lib_path: "".to_string(),
        is_dev: false,
        shield: None,
        limiter: None,
        lifecycle: None,
    };

    use tower_service::Service;
    let req = axum::http::Request::builder()
        .uri("/panic")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_hot_swap_service_poisoned_lock() {
    let router = axum::Router::new().route("/test", axum::routing::get(|| async { "recovered" }));
    let current_router = Arc::new(RwLock::new(router));
    // Poison the lock by panicking in a write guard thread
    let lock_clone = current_router.clone();
    let _ = std::thread::spawn(move || {
        let _guard = lock_clone.write().unwrap();
        panic!("poisoning lock");
    })
    .join();

    assert!(current_router.is_poisoned());

    let mut service = HotSwapService {
        current_router,
        active_libraries: Arc::new(Mutex::new(vec![])),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from(TEST_RELOAD_TOKEN),
        lib_path: "".to_string(),
        is_dev: false,
        shield: None,
        limiter: None,
        lifecycle: None,
    };

    use tower_service::Service;
    let req = axum::http::Request::builder()
        .uri("/test")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let body_bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    assert_eq!(body_bytes, "recovered");
}

#[tokio::test]
async fn test_hot_swap_service_reload_route() {
    // This test only verifies that the route matching works correctly,
    // we can't fully test dylib reloading here without complex setup.
    let router = axum::Router::new().route("/", axum::routing::get(|| async { "root" }));
    let current_router = Arc::new(RwLock::new(router));
    let mut service = HotSwapService {
        current_router: current_router.clone(),
        active_libraries: Arc::new(Mutex::new(vec![])),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from(TEST_RELOAD_TOKEN),
        lib_path: "".to_string(),
        is_dev: true,
        shield: None,
        limiter: None,
        lifecycle: None,
    };

    use tower_service::Service;

    // 1. Valid request to reload (we expect a 500 error because the lib path is empty/invalid)
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/_rullst/internal/reload_dylib")
        .header(
            crate::server::hotswap::RELOAD_TOKEN_HEADER,
            TEST_RELOAD_TOKEN,
        )
        .body(axum::body::Body::empty())
        .unwrap();
    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);

    // 2. Invalid method
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/_rullst/internal/reload_dylib")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = service.call(req).await.unwrap();
    assert_ne!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR); // Will be 404 because not handled by HotSwapService reload block

    // 3. Invalid URI
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/_rullst/internal/other")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = service.call(req).await.unwrap();
    assert_ne!(res.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_inject_hmr_script() {
    let router = axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async {
                axum::response::Html("<html><body>Hello</body></html>")
            }),
        )
        .layer(axum::middleware::from_fn(inject_hmr_script));

    use tower_service::Service;
    let mut service = router;

    let req = axum::http::Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = service.call(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(res.into_body(), 10240).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("Hello"));
    assert!(body_str.contains("Rullst authenticated local hot reload"));
    assert!(body_str.contains("src=\"/_rullst/hmr-client.js\""));
    assert!(!body_str.contains("unpkg.com"));
    assert!(!body_str.contains("morphdom"));

    let response = hmr_client_script().await;
    assert_eq!(
        response.headers().get(axum::http::header::CACHE_CONTROL),
        Some(&axum::http::HeaderValue::from_static("no-store"))
    );
    assert_eq!(
        response.headers().get(axum::http::header::CONTENT_TYPE),
        Some(&axum::http::HeaderValue::from_static(
            "application/javascript; charset=utf-8"
        ))
    );
    let client = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let client = String::from_utf8(client.to_vec()).unwrap();
    assert!(client.contains("window.location.reload()"));
    assert!(client.contains("window.location.host}/_rullst_hmr"));
    assert!(!client.contains("currentPort + 1"));
    assert!(client.contains("Math.min(retryDelayMs * 2, 5000)"));
}

#[tokio::test]
async fn hmr_injection_preserves_html_declared_above_its_buffer_limit() {
    let router = axum::Router::new()
        .route(
            "/large",
            axum::routing::get(|| async {
                axum::response::Response::builder()
                    .header(axum::http::header::CONTENT_TYPE, "text/html")
                    .header(axum::http::header::CONTENT_LENGTH, 10 * 1024 * 1024 + 1)
                    .body(axum::body::Body::from("preserved"))
                    .unwrap()
            }),
        )
        .layer(axum::middleware::from_fn(inject_hmr_script));

    use tower::ServiceExt;
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/large")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.headers().get(axum::http::header::CONTENT_LENGTH),
        Some(&axum::http::HeaderValue::from_static("10485761"))
    );
    let body = axum::body::to_bytes(response.into_body(), 64)
        .await
        .unwrap();
    assert_eq!(body, "preserved");
}

#[tokio::test]
async fn test_server_timing_middleware() {
    use crate::server::server_middleware::server_timing_middleware;

    let router = axum::Router::new()
        .route("/api/ping", axum::routing::get(|| async { "pong" }))
        .layer(axum::middleware::from_fn(server_timing_middleware));

    use tower_service::Service;
    let mut service = router;

    let req = axum::http::Request::builder()
        .uri("/api/ping")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = service.call(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::OK);
    let timing = res.headers().get("server-timing");
    assert!(timing.is_some());
    let timing_str = timing.unwrap().to_str().unwrap();
    assert!(timing_str.contains("app;dur="));
}
