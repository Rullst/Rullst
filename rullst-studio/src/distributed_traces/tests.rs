use super::auth::unix_time;
use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

const KEY: &[u8] = b"0123456789abcdef0123456789abcdef";

fn span(now: u64, suffix: u8, operation: &str, duration_us: u64) -> DistributedTraceSpanV1 {
    DistributedTraceSpanV1 {
        trace_id: "0123456789abcdef0123456789abcdef".to_string(),
        span_id: format!("0123456789abcde{suffix:x}"),
        parent_span_id: None,
        operation: operation.to_string(),
        kind: DistributedTraceKind::Sql,
        started_at_unix_us: now * 1_000_000,
        duration_us,
        status: DistributedTraceStatus::Ok,
    }
}

fn request(signed: &SignedTraceBatch) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/")
        .header(TRACE_SOURCE_HEADER, signed.source())
        .header(TRACE_TIMESTAMP_HEADER, signed.timestamp())
        .header(TRACE_NONCE_HEADER, signed.nonce())
        .header(TRACE_SIGNATURE_HEADER, signed.signature())
        .header("content-type", "application/json")
        .body(Body::from(signed.body().to_vec()))
        .expect("valid trace test request")
}

#[tokio::test]
async fn signed_batches_are_accepted_once_and_deduplicated_across_nonces() {
    let now = unix_time().expect("test clock");
    let store = DistributedTraceStore::new(8).expect("trace store");
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    let signer = TraceBatchSigner::new("api-1", key.clone()).expect("trace signer");
    let ingestor = TraceIngestor::new(store.clone(), "api-1", key).expect("trace ingestor");
    let batch = TraceBatchV1::new(vec![span(now, 1, "users.fetch", 50)]);
    let signed = signer.sign_at(&batch, now, [1; 16]).expect("signed trace");

    let accepted = ingestor
        .router()
        .oneshot(request(&signed))
        .await
        .expect("accepted response");
    assert_eq!(accepted.status(), StatusCode::ACCEPTED);
    assert_eq!(store.snapshot().expect("snapshot").len(), 1);

    let replay = ingestor
        .router()
        .oneshot(request(&signed))
        .await
        .expect("replay response");
    assert_eq!(replay.status(), StatusCode::CONFLICT);

    let duplicate = signer
        .sign_at(&batch, now, [2; 16])
        .expect("second signed trace");
    let response = ingestor
        .router()
        .oneshot(request(&duplicate))
        .await
        .expect("duplicate response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(store.snapshot().expect("snapshot").len(), 1);
}

#[tokio::test]
async fn body_source_signature_and_timestamp_tampering_fail_closed() {
    let now = unix_time().expect("test clock");
    let store = DistributedTraceStore::new(8).expect("trace store");
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    let signer = TraceBatchSigner::new("api-1", key.clone()).expect("trace signer");
    let ingestor = TraceIngestor::new(store.clone(), "api-1", key).expect("trace ingestor");
    let batch = TraceBatchV1::new(vec![span(now, 1, "users.fetch", 50)]);

    for (index, mutation) in ["body", "source", "signature", "timestamp"]
        .into_iter()
        .enumerate()
    {
        let signed = signer
            .sign_at(
                &batch,
                now,
                [u8::try_from(index + 1).expect("small index"); 16],
            )
            .expect("signed trace");
        let mut request = request(&signed);
        match mutation {
            "body" => *request.body_mut() = Body::from(b"{}".to_vec()),
            "source" => {
                request
                    .headers_mut()
                    .insert(TRACE_SOURCE_HEADER, "api-2".parse().expect("header value"));
            }
            "signature" => {
                request.headers_mut().insert(
                    TRACE_SIGNATURE_HEADER,
                    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
                        .parse()
                        .expect("header value"),
                );
            }
            "timestamp" => {
                request.headers_mut().insert(
                    TRACE_TIMESTAMP_HEADER,
                    now.saturating_sub(120)
                        .to_string()
                        .parse()
                        .expect("header value"),
                );
            }
            _ => unreachable!("fixed mutation catalog"),
        }
        let response = ingestor
            .router()
            .oneshot(request)
            .await
            .expect("denied response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{mutation}");
    }
    let other_source_signer = TraceBatchSigner::new(
        "api-2",
        TraceIngestionKey::new(KEY).expect("second trace key"),
    )
    .expect("second source signer");
    let other_source = other_source_signer
        .sign_at(&batch, now, [7; 16])
        .expect("other-source signature");
    let response = ingestor
        .router()
        .oneshot(request(&other_source))
        .await
        .expect("other-source response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(store.snapshot().expect("snapshot").is_empty());
}

#[tokio::test]
async fn invalid_authenticated_schema_is_rejected_before_storage() {
    let now = unix_time().expect("test clock");
    let store = DistributedTraceStore::new(8).expect("trace store");
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    let signer = TraceBatchSigner::new("api-1", key.clone()).expect("trace signer");
    let ingestor = TraceIngestor::new(store.clone(), "api-1", key).expect("trace ingestor");
    let mut invalid = span(now, 1, "users.fetch", 50);
    invalid.parent_span_id = Some(invalid.span_id.clone());
    let signed = signer
        .sign_at(&TraceBatchV1::new(vec![invalid]), now, [8; 16])
        .expect("signed invalid schema");

    let response = ingestor
        .router()
        .oneshot(request(&signed))
        .await
        .expect("schema response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(store.snapshot().expect("snapshot").is_empty());
}

#[tokio::test]
async fn router_rejects_oversized_bodies_before_authentication() {
    let store = DistributedTraceStore::new(8).expect("trace store");
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    let ingestor = TraceIngestor::new(store, "api-1", key).expect("trace ingestor");
    let response = ingestor
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .body(Body::from(vec![b'x'; MAX_TRACE_BATCH_BYTES + 1]))
                .expect("oversized request"),
        )
        .await
        .expect("oversized response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn concurrent_replay_allows_exactly_one_commit() {
    let now = unix_time().expect("test clock");
    let store = DistributedTraceStore::new(8).expect("trace store");
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    let signer = TraceBatchSigner::new("api-1", key.clone()).expect("trace signer");
    let ingestor = TraceIngestor::new(store.clone(), "api-1", key).expect("trace ingestor");
    let signed = Arc::new(
        signer
            .sign_at(
                &TraceBatchV1::new(vec![span(now, 1, "users.fetch", 50)]),
                now,
                [9; 16],
            )
            .expect("signed trace"),
    );

    let mut tasks = Vec::new();
    for _ in 0..16 {
        let router = ingestor.router();
        let signed = Arc::clone(&signed);
        tasks.push(tokio::spawn(async move {
            router
                .oneshot(request(&signed))
                .await
                .expect("concurrent response")
                .status()
        }));
    }
    let mut accepted = 0;
    for task in tasks {
        if task.await.expect("concurrent task") == StatusCode::ACCEPTED {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 1);
    assert_eq!(store.snapshot().expect("snapshot").len(), 1);
}

#[test]
fn store_is_bounded_and_profiler_reports_only_bounded_sql_signals() {
    let now = 1_800_000_000;
    let store = DistributedTraceStore::new(3).expect("trace store");
    store
        .insert_batch(
            "api-1",
            now,
            vec![
                span(now, 4, "evicts-first", 30),
                span(now, 1, "users.fetch", 10),
                span(now, 2, "users.fetch", 20),
                span(now, 3, "users.fetch", SLOW_QUERY_THRESHOLD_US),
            ],
        )
        .expect("stored spans");

    let snapshot = store.snapshot().expect("snapshot");
    assert_eq!(snapshot.len(), 3);
    assert!(
        !snapshot
            .iter()
            .any(|stored| stored.span.span_id.ends_with('4'))
    );
    let findings = store.query_findings().expect("query findings");
    assert!(findings.iter().any(|finding| {
        finding.kind == QueryFindingKind::RepeatedOperation && finding.occurrences == 3
    }));
    assert!(
        findings
            .iter()
            .any(|finding| finding.kind == QueryFindingKind::SlowOperation)
    );
}

#[test]
fn secret_debug_is_redacted_and_policy_rejects_weak_inputs() {
    let key = TraceIngestionKey::new(KEY).expect("trace key");
    assert!(!format!("{key:?}").contains("012345"));
    assert!(matches!(
        TraceIngestionKey::new(b"weak"),
        Err(TraceIngestionError::InvalidKey)
    ));
    assert!(matches!(
        DistributedTraceStore::new(0),
        Err(TraceIngestionError::InvalidCapacity)
    ));
    assert!(matches!(
        DistributedTraceStore::new(MAX_TRACE_STORE_CAPACITY + 1),
        Err(TraceIngestionError::InvalidCapacity)
    ));
}
