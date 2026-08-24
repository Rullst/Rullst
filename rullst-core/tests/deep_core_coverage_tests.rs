#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_core::Router;
use rullst_core::di::Container;
use rullst_core::scheduler::Scheduler;
use rullst_core::server::server_middleware::{server_timing_middleware, zstd_static_middleware};
use tower::ServiceExt;

#[tokio::test]
async fn test_core_routing_all_verbs_and_escape_hatches() {
    let mut router = Router::new();
    router = router.route("/get", axum::routing::get(|| async { "get_ok" }));
    router = router.route("/post", axum::routing::post(|| async { "post_ok" }));
    router = router.route("/put", axum::routing::put(|| async { "put_ok" }));
    router = router.route("/delete", axum::routing::delete(|| async { "delete_ok" }));
    router = router.route("/patch", axum::routing::patch(|| async { "patch_ok" }));

    let app: axum::Router = router.into_axum();

    let req = Request::builder()
        .method("GET")
        .uri("/get")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/post")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("PUT")
        .uri("/put")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("DELETE")
        .uri("/delete")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let req = Request::builder()
        .method("PATCH")
        .uri("/patch")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_core_middleware_server_timing_and_zstd() {
    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "healthy" }))
        .layer(axum::middleware::from_fn(server_timing_middleware))
        .layer(axum::middleware::from_fn(zstd_static_middleware));

    let req = Request::builder()
        .uri("/health")
        .header("Accept-Encoding", "gzip, zstd")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers().contains_key("server-timing"));
}

#[tokio::test]
async fn test_core_di_and_scheduler_advanced() {
    // 1. Dependency Injection container
    #[derive(Clone, Debug, PartialEq)]
    struct AppConfig {
        name: String,
    }

    let mut container = Container::new();
    container.register(AppConfig {
        name: "Rullst Prod".into(),
    });

    let resolved = container.resolve::<AppConfig>();
    assert!(resolved.is_ok());
    assert_eq!(resolved.unwrap().name, "Rullst Prod");

    // 2. Scheduler task lifecycle
    let scheduler = Scheduler::new();
    let scheduled = scheduler.task("*/5 * * * *", || async {
        // Heartbeat tick
    });
    assert!(scheduled.is_ok());
}
