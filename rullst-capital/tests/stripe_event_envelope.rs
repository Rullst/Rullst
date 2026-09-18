use ring::hmac;
use rullst_capital::{
    CapitalError, StripeProvider, StripeSubscriptionEvent, SubscriptionStatus,
    WebhookVerificationMode,
};
use serde_json::{Value, json};
use std::collections::HashMap;

fn snapshot() -> Value {
    json!({
        "id":"evt_envelope", "object":"event", "type":"customer.subscription.updated",
        "api_version":"2025-03-31.basil", "created":1750000000, "livemode":false,
        "account":"acct_fixture", "pending_webhooks":1,
        "data":{"object":{
            "id":"sub_envelope", "object":"subscription", "customer":"cus_envelope",
            "customer_email":"private@example.com", "status":"active", "livemode":false,
            "metadata":{"rullst_owner_reference":"owner_opaque"},
            "items":{"has_more":false,"data":[{
                "price":{"id":"price_envelope","type":"recurring"},
                "current_period_end":1900000000
            }]}
        }}
    })
}

fn headers(payload: &[u8]) -> HashMap<String, String> {
    let now = chrono::Utc::now().timestamp();
    let mut context =
        hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, b"whsec_envelope"));
    context.update(format!("{now}.").as_bytes());
    context.update(payload);
    HashMap::from([(
        "stripe-signature".into(),
        format!("t={now},v1={}", hex::encode(context.sign())),
    )])
}

fn verify(value: &Value) -> Result<StripeSubscriptionEvent, CapitalError> {
    let payload = serde_json::to_vec(value).unwrap();
    StripeProvider::new("sk_test_fixture", "whsec_envelope")
        .verify_subscription_event(&payload, &headers(&payload))
}

#[test]
fn signed_envelope_preserves_scope_and_provider_state_without_preclaiming_delivery() {
    let mut fixture = snapshot();
    fixture["data"]["object"]["status"] = json!("incomplete");
    let event = verify(&fixture).unwrap();
    event.require_real().unwrap();
    assert_eq!(event.verification_mode(), WebhookVerificationMode::Real);
    assert_eq!(event.event_id(), "evt_envelope");
    assert_eq!(event.event_type(), "customer.subscription.updated");
    assert_eq!(event.api_version(), "2025-03-31.basil");
    assert_eq!(event.created_at(), 1750000000);
    assert!(!event.livemode());
    assert_eq!(event.connected_account(), Some("acct_fixture"));
    assert_eq!(event.owner_reference(), Some("owner_opaque"));
    assert_eq!(event.provider_status(), "incomplete");
    assert_eq!(event.subscription().status, SubscriptionStatus::Unpaid);
    assert_eq!(event.subscription().customer_id, "cus_envelope");
    assert_eq!(event.subscription().ends_at, Some(1900000000));
    // Verifying again remains possible after a failed caller transaction. The
    // caller's commit, not this verifier, must decide replay completion.
    assert_eq!(
        event.mutation_digest(),
        verify(&fixture).unwrap().mutation_digest()
    );
    let debug = format!("{event:?}");
    for private in [
        "evt_envelope",
        "acct_fixture",
        "owner_opaque",
        "private@example.com",
        "cus_envelope",
        "sub_envelope",
    ] {
        assert!(!debug.contains(private));
    }
    fixture["account"] = Value::Null;
    fixture["data"]["object"]["metadata"] = json!({});
    let platform = verify(&fixture).unwrap();
    assert_eq!(platform.connected_account(), None);
    assert_eq!(platform.owner_reference(), None);
}

#[test]
fn mutation_digest_excludes_delivery_and_contact_fields_but_binds_domain_input() {
    let original = snapshot();
    let expected = verify(&original).unwrap();
    let mut delivery = original.clone();
    delivery["pending_webhooks"] = json!(2);
    delivery["data"]["object"]["customer_email"] = json!("changed@example.com");
    let changed_delivery = verify(&delivery).unwrap();
    assert_eq!(
        expected.mutation_digest(),
        changed_delivery.mutation_digest()
    );
    assert_ne!(expected.payload_digest(), changed_delivery.payload_digest());
    let pretty = serde_json::to_vec_pretty(&original).unwrap();
    let formatted = StripeProvider::new("sk_test_fixture", "whsec_envelope")
        .verify_subscription_event(&pretty, &headers(&pretty))
        .unwrap();
    assert_eq!(expected.mutation_digest(), formatted.mutation_digest());
    assert_ne!(expected.payload_digest(), formatted.payload_digest());
    for (pointer, changed) in [
        ("/id", json!("evt_other")),
        ("/type", json!("customer.subscription.created")),
        ("/api_version", json!("2025-06-30.basil")),
        ("/created", json!(1750000001)),
        ("/account", json!("acct_other")),
        ("/data/object/id", json!("sub_other")),
        ("/data/object/customer", json!("cus_other")),
        ("/data/object/status", json!("trialing")),
        (
            "/data/object/metadata/rullst_owner_reference",
            json!("owner_other"),
        ),
        ("/data/object/items/data/0/price/id", json!("price_other")),
        (
            "/data/object/items/data/0/current_period_end",
            json!(1900000001),
        ),
    ] {
        let mut fixture = original.clone();
        *fixture.pointer_mut(pointer).unwrap() = changed;
        assert_ne!(
            expected.mutation_digest(),
            verify(&fixture).unwrap().mutation_digest(),
            "unbound {pointer}"
        );
    }
    let mut live = original.clone();
    live["livemode"] = json!(true);
    live["data"]["object"]["livemode"] = json!(true);
    assert_ne!(
        expected.mutation_digest(),
        verify(&live).unwrap().mutation_digest()
    );
    let mut incomplete = original.clone();
    incomplete["data"]["object"]["status"] = json!("incomplete");
    let first = verify(&incomplete).unwrap();
    incomplete["data"]["object"]["status"] = json!("unpaid");
    let second = verify(&incomplete).unwrap();
    assert_eq!(first.subscription().status, second.subscription().status);
    assert_ne!(first.mutation_digest(), second.mutation_digest());
}

#[test]
fn malformed_signed_scope_metadata_and_modes_fail_closed() {
    for (pointer, invalid) in [
        ("/id", Value::Null),
        ("/id", json!("sub_confused")),
        ("/id", json!("evt_")),
        ("/object", json!("subscription")),
        ("/api_version", Value::Null),
        ("/api_version", json!("")),
        ("/api_version", json!("x".repeat(81))),
        ("/created", json!(0)),
        ("/created", json!("1750000000")),
        ("/account", json!("cus_other")),
        ("/livemode", Value::Null),
        ("/livemode", json!("false")),
        ("/data/object/livemode", json!(true)),
        ("/data/object/livemode", Value::Null),
        ("/data/object/metadata", Value::Null),
        (
            "/data/object/metadata/rullst_owner_reference",
            json!("email@example.com"),
        ),
        ("/data/object/metadata/rullst_owner_reference", Value::Null),
    ] {
        let mut fixture = snapshot();
        *fixture.pointer_mut(pointer).unwrap() = invalid;
        assert!(verify(&fixture).is_err(), "accepted malformed {pointer}");
    }
}

#[test]
fn invalid_signatures_are_rejected_and_mock_events_cannot_pass_real_mode_gate() {
    let payload = serde_json::to_vec(&snapshot()).unwrap();
    let provider = StripeProvider::new("sk_test_fixture", "whsec_envelope");
    let signed_headers = headers(&payload);
    let mut tampered = payload.clone();
    tampered.push(b' ');
    assert!(matches!(
        provider.verify_subscription_event(&tampered, &signed_headers),
        Err(CapitalError::InvalidSignature(_))
    ));
    assert!(
        provider
            .verify_subscription_event(&payload, &HashMap::new())
            .is_err()
    );
    assert!(matches!(
        provider.verify_subscription_event(&vec![b' '; 2 * 1024 * 1024 + 1], &HashMap::new()),
        Err(CapitalError::PayloadParseError(_))
    ));
    let mock = StripeProvider::new("mock_api", "mock_signature");
    let mock_headers = HashMap::from([("stripe-signature".into(), "mock_signature".into())]);
    let event = mock
        .verify_subscription_event(&payload, &mock_headers)
        .unwrap();
    assert_eq!(event.verification_mode(), WebhookVerificationMode::Mock);
    assert!(matches!(
        event.require_real(),
        Err(CapitalError::MockWebhookNotAllowed(_))
    ));
    assert_eq!(
        event.mutation_digest(),
        mock.verify_subscription_event(&payload, &mock_headers)
            .unwrap()
            .mutation_digest()
    );
    assert_ne!(
        event.mutation_digest(),
        provider
            .verify_subscription_event(&payload, &signed_headers)
            .unwrap()
            .mutation_digest()
    );
    assert!(
        StripeProvider::new("", "")
            .verify_subscription_event(&payload, &mock_headers)
            .is_err()
    );
}
