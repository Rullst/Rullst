use super::*;
#[cfg(feature = "axum")]
use crate::capital::{SubscriptionStatus, WebhookEvent};
use crate::providers::{LemonSqueezyProvider, StripeProvider};
#[cfg(feature = "axum")]
use axum::http::{Method, Version};

#[cfg(feature = "axum")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingExtension(&'static str);

#[cfg(feature = "axum")]
fn event() -> WebhookEvent {
    WebhookEvent {
        subscription_id: "sub_1".to_string(),
        customer_id: "cus_1".to_string(),
        customer_email: "customer@example.com".to_string(),
        plan_id: "plan_1".to_string(),
        status: SubscriptionStatus::Active,
        ends_at: None,
    }
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn reconstructed_request_preserves_parts_extensions_and_body() {
    let mut request = Request::builder()
        .method(Method::PATCH)
        .uri("/billing/webhook?tenant=acme")
        .version(Version::HTTP_2)
        .header("x-trace-id", "trace-123")
        .body(Body::from("original-body"))
        .unwrap();
    request
        .extensions_mut()
        .insert(ExistingExtension("preserved"));

    let (parts, body) = request.into_parts();
    let body = axum::body::to_bytes(body, 1024).await.unwrap();
    let request = rebuild_request(parts, body, event());

    assert_eq!(request.method(), Method::PATCH);
    assert_eq!(request.uri(), "/billing/webhook?tenant=acme");
    assert_eq!(request.version(), Version::HTTP_2);
    assert_eq!(request.headers()["x-trace-id"], "trace-123");
    assert_eq!(
        request.extensions().get::<ExistingExtension>(),
        Some(&ExistingExtension("preserved"))
    );
    assert!(request.extensions().get::<WebhookEvent>().is_some());
    let body = axum::body::to_bytes(request.into_body(), 1024)
        .await
        .unwrap();
    assert_eq!(body, "original-body");
}

#[test]
fn replay_store_rejects_duplicates_and_expires_entries() {
    let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(10)).unwrap();
    let start = Instant::now();

    assert!(
        store
            .check_and_record_at("first".to_string(), start)
            .is_ok()
    );
    assert!(matches!(
        store.check_and_record_at("first".to_string(), start + Duration::from_secs(1)),
        Err(CapitalError::WebhookReplay(_))
    ));
    assert!(
        store
            .check_and_record_at("first".to_string(), start + Duration::from_secs(10))
            .is_ok()
    );
}

#[test]
fn replay_store_is_bounded_and_provider_scoped() {
    let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(60)).unwrap();
    assert!(store.record_payload("stripe", b"same body").is_ok());
    assert!(store.record_payload("paddle", b"same body").is_ok());
    assert!(matches!(
        store.record_payload("stripe", b"same body"),
        Err(CapitalError::WebhookReplay(_))
    ));
    assert_eq!(
        store.record_payload("stripe", b"different body"),
        Err(CapitalError::WebhookReplayStoreFull)
    );
}

#[test]
fn semantic_event_keys_are_provider_scoped_and_validated() {
    let store = InMemoryWebhookReplayStore::new(2, Duration::from_secs(60)).unwrap();
    assert!(store.record_event_key("stripe", "evt_123").is_ok());
    assert!(store.record_event_key("paddle", "evt_123").is_ok());
    assert!(matches!(
        store.record_event_key("stripe", "evt_123"),
        Err(CapitalError::WebhookReplay(_))
    ));
    assert!(
        InMemoryWebhookReplayStore::default()
            .record_event_key("stripe", "contains spaces")
            .is_err()
    );
}

#[test]
fn replay_store_rejects_invalid_configuration() {
    assert!(InMemoryWebhookReplayStore::new(0, Duration::from_secs(1)).is_err());
    assert!(InMemoryWebhookReplayStore::new(1, Duration::ZERO).is_err());
}

#[tokio::test]
async fn canonical_verifier_decodes_stripe_and_lemonsqueezy_and_rejects_mock_in_production() {
    let stripe_payload = serde_json::to_vec(&serde_json::json!({
        "type": "customer.subscription.updated",
        "data": { "object": {
            "object": "subscription",
            "id": "sub_stripe",
            "customer": "cus_stripe",
            "items": { "has_more": false, "data": [{ "price": { "id": "price_stripe" } }] },
            "status": "active"
        }}
    }))
    .expect("fixture serialization must succeed");
    let stripe_headers = HashMap::from([(
        "stripe-signature".to_string(),
        "mock_stripe_signature".to_string(),
    )]);
    let stripe = StripeProvider::new("mock_api", "mock_stripe_signature");
    let stripe_store =
        WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default()));
    let stripe_event = verify_payload(
        &stripe,
        &stripe_payload,
        &stripe_headers,
        &stripe_store,
        true,
    )
    .await
    .expect("local mock signature must produce a normalized event");
    assert_eq!(stripe_event.subscription_id, "sub_stripe");

    let lemon_payload = serde_json::to_vec(&serde_json::json!({
        "meta": { "event_name": "subscription_updated" },
        "data": {
            "type": "subscriptions",
            "id": "123",
            "attributes": {
                "customer_id": 42,
                "store_id": 42,
                "test_mode": true,
                "user_email": "lemon@example.com",
                "variant_id": 7,
                "status": "active"
            }
        }
    }))
    .expect("fixture serialization must succeed");
    let lemon_headers = HashMap::from([(
        "x-signature".to_string(),
        "mock_lemon_signature".to_string(),
    )]);
    let lemon = LemonSqueezyProvider::new("mock_api", "mock_lemon_signature");
    let lemon_store = WebhookReplayBackend::Memory(Arc::new(InMemoryWebhookReplayStore::default()));
    let lemon_event = verify_payload(&lemon, &lemon_payload, &lemon_headers, &lemon_store, true)
        .await
        .expect("local mock signature must produce a normalized event");
    assert_eq!(lemon_event.subscription_id, "123");

    assert!(matches!(
        verify_payload(
            &stripe,
            &stripe_payload,
            &stripe_headers,
            &WebhookReplayBackend::Memory(Arc::new(
                InMemoryWebhookReplayStore::default(),
            )),
            false,
        )
        .await,
        Err(CapitalError::MockWebhookNotAllowed(provider)) if provider == "stripe"
    ));
}

#[test]
fn replay_store_debug_does_not_print_the_ledger() {
    let store = InMemoryWebhookReplayStore::new(4, Duration::from_secs(60)).unwrap();
    store.check_and_record("evt_debug_fixture").unwrap();
    let debug = format!("{:?}", WebhookReplayBackend::Memory(Arc::new(store)));
    assert!(debug.contains("max_entries: 4"));
    assert!(!debug.contains("evt_debug_fixture"));
}
