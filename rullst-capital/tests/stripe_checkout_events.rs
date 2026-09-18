use rullst_capital::StripeProvider;
use serde_json::{Value, json};
use std::collections::HashMap;

fn fixture(kind: &str) -> Value {
    json!({"id":"evt_checkout", "object":"event", "created":1800000000,
        "type":kind,"livemode":false,"api_version":"2025-03-31.basil",
        "data":{"object":{"id":"cs_owner","object":"checkout.session","mode":"subscription",
            "status":"complete","livemode":false,"client_reference_id":"owner_fixed","customer":"cus_owner",
            "subscription":"sub_owner","metadata":{"rullst_owner_reference":"owner_fixed","rullst_attempt_reference":"attempt_fixed"}}}})
}
fn sign(body: &Value) -> (Vec<u8>, HashMap<String, String>) {
    let body = serde_json::to_vec(body).unwrap();
    let timestamp = chrono::Utc::now().timestamp();
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, b"whsec_fixture");
    let mut signer = ring::hmac::Context::with_key(&key);
    signer.update(format!("{timestamp}.").as_bytes());
    signer.update(&body);
    let mut headers = HashMap::new();
    headers.insert(
        "stripe-signature".into(),
        format!("t={timestamp},v1={}", hex::encode(signer.sign())),
    );
    (body, headers)
}
#[test]
fn verifies_checkout_lifecycle_without_email_and_rejects_identity_confusion() {
    let provider = StripeProvider::new("sk_test_fixture", "whsec_fixture");
    for kind in [
        "checkout.session.completed",
        "checkout.session.async_payment_succeeded",
        "checkout.session.async_payment_failed",
        "checkout.session.expired",
    ] {
        let mut value = fixture(kind);
        if kind.ends_with("expired") {
            value["data"]["object"]["status"] = json!("expired");
            value["data"]["object"]["subscription"] = Value::Null;
        }
        let (body, headers) = sign(&value);
        let event = provider.verify_checkout_event(&body, &headers).unwrap();
        assert_eq!(event.owner_reference(), "owner_fixed");
        assert_eq!(event.session_id(), "cs_owner");
        assert_eq!(event.customer_id(), "cus_owner");
        assert_eq!(event.attempt_reference(), "attempt_fixed");
    }
    for path in [
        "/id",
        "/object",
        "/type",
        "/data/object/id",
        "/data/object/customer",
        "/data/object/subscription",
        "/data/object/client_reference_id",
        "/data/object/metadata/rullst_owner_reference",
        "/data/object/mode",
        "/data/object/status",
    ] {
        let mut value = fixture("checkout.session.completed");
        *value.pointer_mut(path).unwrap() = json!("wrong");
        let (body, headers) = sign(&value);
        assert!(
            provider.verify_checkout_event(&body, &headers).is_err(),
            "{path}"
        );
    }
    let mut value = fixture("checkout.session.completed");
    value["account"] = json!("acct_connected");
    let (body, headers) = sign(&value);
    assert!(provider.verify_checkout_event(&body, &headers).is_err());
    let (mut body, headers) = sign(&fixture("checkout.session.completed"));
    body.push(b' ');
    assert!(provider.verify_checkout_event(&body, &headers).is_err());
    assert!(
        StripeProvider::new("mock_key", "mock_secret")
            .verify_checkout_event(&body, &headers)
            .is_err()
    );
}

#[test]
fn delivery_counters_and_contact_do_not_change_the_durable_checkout_mutation() {
    let provider = StripeProvider::new("sk_test_fixture", "whsec_fixture");
    let value = fixture("checkout.session.completed");
    let (body, headers) = sign(&value);
    let first = provider.verify_checkout_event(&body, &headers).unwrap();
    let mut changed = value.clone();
    changed["pending_webhooks"] = json!(3);
    changed["data"]["object"]["customer_details"] = json!({"email":"changed@example.com"});
    let (body, headers) = sign(&changed);
    let retry = provider.verify_checkout_event(&body, &headers).unwrap();
    assert_eq!(first.mutation_digest(), retry.mutation_digest());
    assert_ne!(first.payload_digest(), retry.payload_digest());
    changed["data"]["object"]["subscription"] = json!("sub_other");
    let (body, headers) = sign(&changed);
    assert_ne!(
        first.mutation_digest(),
        provider
            .verify_checkout_event(&body, &headers)
            .unwrap()
            .mutation_digest()
    );
}
