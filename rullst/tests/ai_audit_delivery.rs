#![cfg(feature = "ai")]
#![allow(clippy::expect_used)]

use rullst::ai::{AiCancellation, AuditDeliveryClient, AuditDeliveryMode};

#[tokio::test]
async fn umbrella_ai_feature_exposes_bounded_audit_delivery() {
    let client = AuditDeliveryClient::try_cloud(
        "https://unreachable.invalid/audit",
        "facade-test",
        "key-2026",
        "mock_audit",
    )
    .expect("valid offline audit client");
    let receipt = client
        .publish(
            "event-facade-001",
            1_788_000_000_000,
            &serde_json::json!({"kind": "rag.completed"}),
            &AiCancellation::new(),
        )
        .await
        .expect("facade audit fixture succeeds");

    assert_eq!(receipt.event_id(), "event-facade-001");
    assert_eq!(receipt.mode(), AuditDeliveryMode::OfflineMock);
}
