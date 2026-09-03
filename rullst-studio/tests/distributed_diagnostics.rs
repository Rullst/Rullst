#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::body::Body;
use axum::extract::{ConnectInfo, Request as AxumRequest};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use rullst_studio::distributed_traces::{
    DistributedTraceKind, DistributedTraceSpanV1, DistributedTraceStatus, DistributedTraceStore,
    TRACE_NONCE_HEADER, TRACE_SIGNATURE_HEADER, TRACE_SOURCE_HEADER, TRACE_TIMESTAMP_HEADER,
    TraceBatchSigner, TraceBatchV1, TraceIngestionKey, TraceIngestor,
};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

async fn inject_loopback(mut request: AxumRequest, next: Next) -> axum::response::Response {
    request.headers_mut().insert(
        axum::http::header::HOST,
        axum::http::HeaderValue::from_static("127.0.0.1:5555"),
    );
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:42000"
            .parse::<SocketAddr>()
            .expect("loopback peer"),
    ));
    next.run(request).await
}

#[tokio::test]
async fn authenticated_push_is_visible_only_through_the_local_studio_router() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock")
        .as_secs();
    let store = DistributedTraceStore::new(32).expect("trace store");
    let key = TraceIngestionKey::new(b"0123456789abcdef0123456789abcdef").expect("trace key");
    let signer = TraceBatchSigner::new("worker-1", key.clone()).expect("trace signer");
    let ingestor = TraceIngestor::new(store.clone(), "worker-1", key).expect("trace ingestor");
    let signed = signer
        .sign(&TraceBatchV1::new(vec![DistributedTraceSpanV1 {
            trace_id: "0123456789abcdef0123456789abcdef".to_string(),
            span_id: "0123456789abcdef".to_string(),
            parent_span_id: None,
            operation: "courses.load".to_string(),
            kind: DistributedTraceKind::Sql,
            started_at_unix_us: now * 1_000_000,
            duration_us: 42,
            status: DistributedTraceStatus::Ok,
        }]))
        .expect("signed batch");
    let ingest_request = Request::builder()
        .method("POST")
        .uri("/")
        .header(TRACE_SOURCE_HEADER, signed.source())
        .header(TRACE_TIMESTAMP_HEADER, signed.timestamp())
        .header(TRACE_NONCE_HEADER, signed.nonce())
        .header(TRACE_SIGNATURE_HEADER, signed.signature())
        .header("content-type", "application/json")
        .body(Body::from(signed.body().to_vec()))
        .expect("ingestion request");
    let ingestion = ingestor
        .router()
        .oneshot(ingest_request)
        .await
        .expect("ingestion response");
    assert_eq!(ingestion.status(), StatusCode::ACCEPTED);
    let read_attempt = ingestor
        .router()
        .oneshot(
            Request::builder()
                .uri("/studio/traces")
                .body(Body::empty())
                .expect("read attempt"),
        )
        .await
        .expect("read attempt response");
    assert_eq!(read_attempt.status(), StatusCode::NOT_FOUND);

    let cache = rullst_core::Cache::memory();
    cache
        .put("course:42", "cached-course", Some(60))
        .await
        .expect("cache fixture");
    let studio = Studio::new()
        .with_distributed_traces(store)
        .with_cache(cache)
        .into_router(LocalStudioAccess::loopback_only())
        .expect("local Studio")
        .layer(axum::middleware::from_fn(inject_loopback));

    let trace_response = studio
        .clone()
        .oneshot(
            Request::builder()
                .uri("/studio/traces")
                .body(Body::empty())
                .expect("trace page request"),
        )
        .await
        .expect("trace page response");
    let trace_body = axum::body::to_bytes(trace_response.into_body(), usize::MAX)
        .await
        .expect("trace page body");
    let trace_body = String::from_utf8(trace_body.to_vec()).expect("trace page UTF-8");
    assert!(trace_body.contains("worker-1"));
    assert!(trace_body.contains("courses.load"));

    let cache_response = studio
        .oneshot(
            Request::builder()
                .uri("/studio/cache")
                .body(Body::empty())
                .expect("cache page request"),
        )
        .await
        .expect("cache page response");
    let cache_body = axum::body::to_bytes(cache_response.into_body(), usize::MAX)
        .await
        .expect("cache page body");
    let cache_body = String::from_utf8(cache_body.to_vec()).expect("cache page UTF-8");
    assert!(cache_body.contains("cache-"), "{cache_body}");
    assert!(!cache_body.contains("course:42"));
    assert!(!cache_body.contains("cached-course"));
}
