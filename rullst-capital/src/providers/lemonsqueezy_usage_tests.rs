use super::*;
use crate::providers::BillingProvider;
use axum::{Json, Router, extract::State, http::HeaderMap, response::IntoResponse, routing::post};
use tokio::sync::mpsc;

#[derive(Debug)]
struct CapturedRequest {
    authorization: Option<String>,
    accept: Option<String>,
    content_type: Option<String>,
    body: Value,
}

async fn lemon_fixture(
    State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    sender
        .send(CapturedRequest {
            authorization: header(&headers, "authorization"),
            accept: header(&headers, "accept"),
            content_type: header(&headers, "content-type"),
            body,
        })
        .expect("capture receiver remains open");
    Json(serde_json::json!({
        "jsonapi": {"version": "1.0"},
        "data": {
            "type": "usage-records",
            "id": "91",
            "attributes": {
                "subscription_item_id": 42,
                "quantity": 5,
                "action": "increment"
            }
        }
    }))
}

#[tokio::test]
async fn current_usage_record_protocol_is_exact_and_bound_to_response() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let router = Router::new()
        .route("/v1/usage-records", post(lemon_fixture))
        .with_state(sender);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Lemon fixture");
    let address = listener.local_addr().expect("read fixture address");
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve Lemon fixture");
    });
    let endpoint = format!("http://{address}/v1/usage-records");
    let record = record();
    let receipt = execute_at("lemon_fixture", &endpoint, &record)
        .await
        .expect("bound usage-record response");
    assert!(receipt.is_live_accepted());
    assert_eq!(receipt.provider(), "lemonsqueezy");
    assert_eq!(receipt.record_id(), "91");
    assert_eq!(
        receipt.deduplication(),
        UsageDeduplication::ApplicationOutboxRequired
    );

    let captured = receiver.recv().await.expect("captured request");
    assert_eq!(
        captured.authorization.as_deref(),
        Some("Bearer lemon_fixture")
    );
    assert_eq!(captured.accept.as_deref(), Some("application/vnd.api+json"));
    assert_eq!(
        captured.content_type.as_deref(),
        Some("application/vnd.api+json")
    );
    assert_eq!(captured.body, request_body(&record));
    assert!(
        captured
            .body
            .pointer("/data/relationships/subscription-item/data")
            .is_some()
    );

    server.abort();
}

#[tokio::test]
async fn mock_is_deterministic_and_mismatches_fail_closed() {
    let provider = LemonSqueezyProvider::new("mock_usage", "mock_webhook");
    let record = record();
    let first = provider
        .report_metered_usage(&record)
        .await
        .expect("mock receipt");
    let second = provider
        .report_metered_usage(&record)
        .await
        .expect("mock receipt");
    assert_eq!(first, second);
    assert_eq!(first.status(), UsageStatus::Mock);
    assert!(!first.is_live_accepted());

    for mismatched in [
        serde_json::json!({
            "data": {"type": "other", "id": "91", "attributes": {
                "subscription_item_id": 42, "quantity": 5, "action": "increment"
            }}
        }),
        serde_json::json!({
            "data": {"type": "usage-records", "id": "91", "attributes": {
                "subscription_item_id": 43, "quantity": 5, "action": "increment"
            }}
        }),
        serde_json::json!({
            "data": {"type": "usage-records", "id": "91", "attributes": {
                "subscription_item_id": 42, "quantity": 6, "action": "increment"
            }}
        }),
    ] {
        assert!(matches!(
            bind_response(&record, &mismatched),
            Err(CapitalError::ProviderRequestFailed(_))
        ));
    }

    let live = LemonSqueezyProvider::new("lemon_live_fixture", "mock_webhook");
    assert!(matches!(
        BillingProvider::report_usage(&live, "42", "lesson_minutes", 5).await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        BillingProvider::report_usage(&provider, "42", "lesson_minutes", 0).await,
        Err(CapitalError::InvalidUsage(_))
    ));
}

fn record() -> LemonSqueezyUsageRecord {
    LemonSqueezyUsageRecord::new(
        "42",
        "ai_exercises",
        5,
        crate::LemonSqueezyUsageAction::Increment,
        "school-7:usage-99",
    )
    .expect("valid usage record")
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
