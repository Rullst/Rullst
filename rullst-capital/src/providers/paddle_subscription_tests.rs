use super::*;
use crate::providers::paddle_test_support::*;
use serde_json::json;

fn envelope() -> Value {
    json!({"event_id":EVENT,"event_type":"subscription.created","occurred_at":"2026-09-18T12:00:00Z","data":subscription()})
}
fn signed(value: &Value, timestamp: i64) -> (Vec<u8>, HashMap<String, String>) {
    let payload = serde_json::to_vec(value).unwrap();
    let mut message = format!("{timestamp}:").into_bytes();
    message.extend_from_slice(&payload);
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, SECRET.as_bytes());
    let tag = hex::encode(ring::hmac::sign(&key, &message));
    (
        payload,
        HashMap::from([(
            "paddle-signature".into(),
            format!("ts={timestamp};h1={tag}"),
        )]),
    )
}

#[tokio::test]
async fn signed_creation_binds_checkout_without_email_and_current_reads_reconcile() {
    let fixture = fixture().await;
    let checkout = fixture
        .provider
        .create_transaction_checkout(&request())
        .await
        .unwrap();
    let now = chrono::Utc::now().timestamp();
    let value = envelope();
    let (body, headers) = signed(&value, now);
    let verified = fixture
        .provider
        .verify_checkout_subscription(&request(), &checkout, &body, &headers)
        .unwrap();
    assert_eq!(verified.event_id(), EVENT);
    assert_eq!(verified.event_type(), "subscription.created");
    assert_eq!(verified.owner_reference(), "owner_opaque");
    assert_eq!(verified.attempt_reference(), "attempt_opaque");
    assert!(verified.occurred_at() > 0);
    assert_eq!(verified.transaction_id(), Some(TRANSACTION));
    assert_eq!(verified.provider_status(), "active");
    assert_eq!(verified.subscription().subscription_id, SUBSCRIPTION);
    assert!(verified.subscription().customer_email.is_empty());
    assert!(!verified.is_mock());
    verified.require_real().unwrap();
    let mut repeated = value;
    repeated["data"]["customer"] = json!({"email":"changed@example.test"});
    repeated["notification_id"] = json!("changed_delivery");
    let (body, headers) = signed(&repeated, now);
    assert_eq!(
        verified.mutation_digest(),
        fixture
            .provider
            .verify_checkout_subscription(&request(), &checkout, &body, &headers)
            .unwrap()
            .mutation_digest()
    );
    let current = fixture
        .provider
        .retrieve_bound_subscription(&request(), SUBSCRIPTION)
        .await
        .unwrap();
    assert_eq!(current.subscription().customer_id, CUSTOMER);
    assert_eq!(current.provider_status(), "active");
    assert_eq!(current.sandbox(), Some(true));
    current.require_real().unwrap();
    assert!(!format!("{verified:?}{current:?}").contains(CUSTOMER));
    let mut completed = transaction();
    completed["status"] = json!("completed");
    completed["subscription_id"] = json!(SUBSCRIPTION);
    completed["checkout"] = Value::Null;
    fixture.respond(
        "GET",
        &format!("/transactions/{TRANSACTION}"),
        200,
        completed,
    );
    let bound = fixture
        .provider
        .retrieve_transaction_checkout(&request(), TRANSACTION)
        .await
        .unwrap();
    for (event_type, status) in [
        ("subscription.updated", "past_due"),
        ("subscription.activated", "active"),
        ("subscription.resumed", "active"),
        ("subscription.trialing", "trialing"),
        ("subscription.paused", "paused"),
        ("subscription.canceled", "canceled"),
        ("subscription.past_due", "past_due"),
    ] {
        let mut value = envelope();
        value["event_type"] = json!(event_type);
        value["data"]["status"] = json!(status);
        value["data"]
            .as_object_mut()
            .unwrap()
            .remove("transaction_id");
        if matches!(status, "paused" | "canceled") {
            value["data"]["current_billing_period"] = Value::Null;
        }
        let (body, headers) = signed(&value, now);
        let event = fixture
            .provider
            .verify_checkout_subscription(&request(), &bound, &body, &headers)
            .unwrap();
        assert_eq!(event.provider_status(), status);
        assert_ne!(event.mutation_digest(), verified.mutation_digest());
        fixture.respond(
            "GET",
            &format!("/subscriptions/{SUBSCRIPTION}"),
            200,
            value["data"].clone(),
        );
        assert_eq!(
            fixture
                .provider
                .retrieve_bound_subscription(&request(), SUBSCRIPTION)
                .await
                .unwrap()
                .provider_status(),
            status
        );
    }
}

#[tokio::test]
async fn signed_but_confused_events_bad_signatures_and_unbound_lifecycle_fail() {
    let fixture = fixture().await;
    let input = request();
    let checkout = fixture
        .provider
        .create_transaction_checkout(&input)
        .await
        .unwrap();
    let now = chrono::Utc::now().timestamp();
    for (pointer, value) in [
        ("/event_id", json!("bad")),
        ("/event_type", json!("transaction.completed")),
        ("/event_type", json!("subscription.canceled")),
        ("/occurred_at", json!("invalid")),
        ("/data/id", json!("sub_bad")),
        ("/data/customer_id", json!("ctm_other")),
        ("/data/status", json!("unrecognized")),
        ("/data/collection_mode", json!("manual")),
        ("/data/custom_data/rullst_owner_reference", json!("other")),
        ("/data/custom_data/rullst_attempt_reference", json!("other")),
        ("/data/items/0/price/id", json!("pri_other")),
        ("/data/transaction_id", Value::Null),
        (
            "/data/transaction_id",
            json!("txn_01hv8m0mnx3sj85e7gxc6kga04"),
        ),
        ("/data/current_billing_period", Value::Null),
        (
            "/data/current_billing_period/ends_at",
            json!("2026-01-01T00:00:00Z"),
        ),
    ] {
        let mut bad = envelope();
        *bad.pointer_mut(pointer).unwrap() = value;
        let (body, headers) = signed(&bad, now);
        assert!(
            fixture
                .provider
                .verify_checkout_subscription(&input, &checkout, &body, &headers)
                .is_err(),
            "{pointer}"
        );
    }
    let (body, headers) = signed(&envelope(), now - 301);
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&input, &checkout, &body, &headers)
            .is_err()
    );
    let (body, mut headers) = signed(&envelope(), now);
    headers.insert("paddle-signature".into(), "ts=1;h1=invalid".into());
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&input, &checkout, &body, &headers)
            .is_err()
    );
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&input, &checkout, &body, &HashMap::new())
            .is_err()
    );
    let oversized = vec![b'x'; 2 * 1024 * 1024 + 1];
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&input, &checkout, &oversized, &headers)
            .is_err()
    );
    let mut update = envelope();
    update["event_type"] = json!("subscription.updated");
    let (body, headers) = signed(&update, now);
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&input, &checkout, &body, &headers)
            .is_err()
    );
    let other = PaddleCheckoutRequest::new(
        CUSTOMER,
        PRICE,
        "owner_opaque",
        "other_attempt",
        "https://app.example/pay",
    )
    .unwrap();
    assert!(
        fixture
            .provider
            .verify_checkout_subscription(&other, &checkout, &body, &headers)
            .is_err()
    );
    let wrong_mode = PaddleProvider::new("unused", SECRET);
    assert!(
        wrong_mode
            .verify_checkout_subscription(&input, &checkout, &body, &headers)
            .is_err()
    );
    let mut foreign = subscription();
    foreign["id"] = json!("sub_01hv8x29kz0t586xy6zn1a62nz");
    fixture.respond(
        "GET",
        &format!("/subscriptions/{SUBSCRIPTION}"),
        200,
        foreign,
    );
    assert!(
        fixture
            .provider
            .retrieve_bound_subscription(&input, SUBSCRIPTION)
            .await
            .is_err()
    );
    assert!(
        fixture
            .provider
            .retrieve_bound_subscription(&input, "sub_/../foreign")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn mocks_cannot_be_used_as_real_evidence_and_management_uses_selected_api() {
    let provider = PaddleProvider::new("mock_key", "mock_secret");
    let input = request();
    let checkout = provider.create_transaction_checkout(&input).await.unwrap();
    let mut value = envelope();
    value["data"]["transaction_id"] = json!(checkout.id());
    let body = serde_json::to_vec(&value).unwrap();
    let headers = HashMap::from([("paddle-signature".into(), "mock_secret".into())]);
    let event = provider
        .verify_checkout_subscription(&input, &checkout, &body, &headers)
        .unwrap();
    assert!(event.is_mock());
    assert!(event.require_real().is_err());
    let state = provider
        .retrieve_bound_subscription(&input, SUBSCRIPTION)
        .await
        .unwrap();
    assert_eq!(state.sandbox(), None);
    assert!(state.require_real().is_err());
    let fixture = fixture().await;
    for (action, terminal) in [("cancel", "canceled"), ("pause", "paused")] {
        let path = format!("/subscriptions/{SUBSCRIPTION}/{action}");
        let mut value = subscription();
        value["scheduled_change"] = json!({"action":action,"effective_at":"2026-10-01T00:00:00Z"});
        fixture.respond("POST", &path, 200, value.clone());
        if action == "cancel" {
            fixture
                .provider
                .cancel_subscription(SUBSCRIPTION)
                .await
                .unwrap();
        } else {
            fixture
                .provider
                .pause_subscription(SUBSCRIPTION)
                .await
                .unwrap();
        }
        value["status"] = json!(terminal);
        value["scheduled_change"] = Value::Null;
        fixture.respond("POST", &path, 200, value.clone());
        fixture
            .provider
            .change_subscription(SUBSCRIPTION, action)
            .await
            .unwrap();
        value["id"] = json!("sub_foreign");
        fixture.respond("POST", &path, 200, value);
        assert!(
            fixture
                .provider
                .change_subscription(SUBSCRIPTION, action)
                .await
                .is_err()
        );
        fixture.respond("POST", &path, 200, subscription());
        assert!(
            fixture
                .provider
                .change_subscription(SUBSCRIPTION, action)
                .await
                .is_err()
        );
    }
    let before = fixture.requests.lock().unwrap().len();
    assert!(
        fixture
            .provider
            .cancel_subscription("sub_../wrong")
            .await
            .is_err()
    );
    assert!(
        fixture
            .provider
            .pause_subscription("sub_../wrong")
            .await
            .is_err()
    );
    assert_eq!(before, fixture.requests.lock().unwrap().len());
}
