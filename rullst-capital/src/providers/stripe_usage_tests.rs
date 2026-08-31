use super::*;
use crate::providers::BillingProvider;
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use tokio::sync::mpsc;

#[derive(Debug)]
struct CapturedRequest {
    authorization: Option<String>,
    content_type: Option<String>,
    idempotency_key: Option<String>,
    body: String,
}

async fn stripe_fixture(
    State(sender): State<mpsc::UnboundedSender<CapturedRequest>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body = String::from_utf8_lossy(&body).into_owned();
    sender
        .send(CapturedRequest {
            authorization: header(&headers, "authorization"),
            content_type: header(&headers, "content-type"),
            idempotency_key: header(&headers, "idempotency-key"),
            body: body.clone(),
        })
        .expect("capture receiver remains open");

    if body.contains("event_name=failure") {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "provider-secret-body-must-not-escape",
        )
            .into_response();
    }
    if body.contains("event_name=oversized") {
        return Json(serde_json::json!({"padding": "x".repeat(1024 * 1024)})).into_response();
    }
    Json(serde_json::json!({
        "object": "billing.meter_event",
        "created": 1_000,
        "event_name": "lesson_minutes",
        "identifier": "usage-event-123",
        "livemode": false,
        "payload": {
            "value": "7",
            "stripe_customer_id": "cus_123"
        },
        "timestamp": 1_000
    }))
    .into_response()
}

#[tokio::test]
async fn current_meter_event_protocol_is_exact_bounded_and_redacted() {
    let (endpoint, mut receiver, server) = start_fixture().await;
    let lesson_event = event("lesson_minutes");
    let receipt = execute_at("sk_fixture", &endpoint, &lesson_event, 1_000)
        .await
        .expect("bound meter event response");
    assert!(receipt.is_live_accepted());
    assert_eq!(receipt.provider(), "stripe");
    assert_eq!(receipt.record_id(), "usage-event-123");
    assert_eq!(
        receipt.deduplication(),
        UsageDeduplication::ProviderRollingWindow
    );

    let captured = receiver.recv().await.expect("captured request");
    assert_eq!(captured.authorization.as_deref(), Some("Bearer sk_fixture"));
    assert_eq!(
        captured.content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(captured.idempotency_key.as_deref(), Some("usage-event-123"));
    assert_eq!(
        captured.body,
        "event_name=lesson_minutes&payload%5Bvalue%5D=7&payload%5Bstripe_customer_id%5D=cus_123&identifier=usage-event-123&timestamp=1000"
    );

    let failure = execute_at("sk_fixture", &endpoint, &event("failure"), 1_000)
        .await
        .expect_err("HTTP failure must remain typed");
    assert!(!failure.to_string().contains("provider-secret-body"));
    let _ = receiver.recv().await.expect("captured failure request");

    let oversized = execute_at("sk_fixture", &endpoint, &event("oversized"), 1_000)
        .await
        .expect_err("oversized response must fail closed");
    assert!(oversized.to_string().contains("exceeded 1 MiB"));
    let _ = receiver.recv().await.expect("captured oversized request");

    server.abort();
}

#[tokio::test]
async fn mock_is_deterministic_and_response_mismatch_fails_closed() {
    let provider = StripeProvider::new("mock_usage", "mock_webhook");
    let event = StripeMeterEvent::new("cus_123", "lesson_minutes", 7, "usage-event-123")
        .expect("valid current event");
    let first = provider
        .report_metered_usage(&event)
        .await
        .expect("mock receipt");
    let second = provider
        .report_metered_usage(&event)
        .await
        .expect("mock receipt");
    assert_eq!(first, second);
    assert_eq!(first.status(), UsageStatus::Mock);
    assert!(!first.is_live_accepted());

    let mismatched = serde_json::json!({
        "object": "billing.meter_event",
        "event_name": "lesson_minutes",
        "identifier": "different",
        "payload": {"value": "7", "stripe_customer_id": "cus_123"},
        "timestamp": event.occurred_at()
    });
    assert!(matches!(
        bind_response(&event, &mismatched),
        Err(CapitalError::ProviderRequestFailed(_))
    ));

    let live = StripeProvider::new("sk_live_fixture", "mock_webhook");
    assert!(matches!(
        BillingProvider::report_usage(&live, "legacy_item", "lesson_minutes", 7).await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        BillingProvider::report_usage(&provider, "legacy_item", "lesson_minutes", 0).await,
        Err(CapitalError::InvalidUsage(_))
    ));
}

fn event(event_name: &str) -> StripeMeterEvent {
    StripeMeterEvent::new_at("cus_123", event_name, 7, 1_000, "usage-event-123", 1_000)
        .expect("valid fixture event")
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn start_fixture() -> (
    String,
    mpsc::UnboundedReceiver<CapturedRequest>,
    tokio::task::JoinHandle<()>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let router = Router::new()
        .route("/v1/billing/meter_events", post(stripe_fixture))
        .with_state(sender);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Stripe fixture");
    let address = listener.local_addr().expect("read fixture address");
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve Stripe fixture");
    });
    (
        format!("http://{address}/v1/billing/meter_events"),
        receiver,
        server,
    )
}
