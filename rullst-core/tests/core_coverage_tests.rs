// tests/core_coverage_tests.rs — Comprehensive integration tests for Edge, Scalar, and DevOps in rullst-core.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_core::devops::DevOpsAgent;
use rullst_core::edge::{EdgeRequest, EdgeResponse, EdgeServer};
use rullst_core::scalar::scalar_docs_router;
use tower::ServiceExt;

#[tokio::test]
async fn test_edge_request_response_and_server() {
    let req = EdgeRequest::new("POST", "/api/data")
        .with_header("content-type", "application/json")
        .with_body(b"{\"ok\": true}".to_vec());

    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/api/data");
    assert_eq!(
        req.headers.get("content-type").map(|s| s.as_str()),
        Some("application/json")
    );
    assert_eq!(req.body, b"{\"ok\": true}");

    let res = EdgeResponse::new(200)
        .with_header("x-edge-cache", "HIT")
        .with_body(b"edge response payload".to_vec());

    assert_eq!(res.status, 200);
    assert_eq!(
        res.headers.get("x-edge-cache").map(|s| s.as_str()),
        Some("HIT")
    );
    assert_eq!(res.body, b"edge response payload");

    let server = EdgeServer::new(|_req| async move {
        EdgeResponse::new(200).with_body(b"Hello Edge".to_vec())
    })
    .with_port(8080);

    assert_eq!(server.port, 8080);
}

#[tokio::test]
async fn test_scalar_docs_router_endpoints() {
    let app = scalar_docs_router("/openapi.json");

    // 1. GET /docs
    let req = Request::builder().uri("/docs").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let html_body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let html_str = String::from_utf8_lossy(&html_body);
    assert!(html_str.contains("Scalar UI") || html_str.contains("openapi.json"));

    // 2. GET /openapi.json
    let req = Request::builder()
        .uri("/openapi.json")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[test]
fn test_devops_infrastructure_recommendations_full() {
    let agent = DevOpsAgent::new();

    // High latency + High memory + Saturated pool
    let recs = agent.analyze_telemetry(25_000, 750 * 1024 * 1024, 19, 20);
    assert_eq!(recs.len(), 3);

    let latency_rec = recs
        .iter()
        .find(|r| r.metric_name == "tokio_tick_latency_us")
        .unwrap();
    assert_eq!(latency_rec.urgency, "HIGH");

    let mem_rec = recs
        .iter()
        .find(|r| r.metric_name == "memory_rss_mb")
        .unwrap();
    assert_eq!(mem_rec.urgency, "MEDIUM");

    let pool_rec = recs
        .iter()
        .find(|r| r.metric_name == "db_pool_utilization")
        .unwrap();
    assert_eq!(pool_rec.urgency, "HIGH");
}
