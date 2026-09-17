//! Header-authenticated real HMAC fixtures; no provider network calls.
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::hmac;
use rullst_capital::{
    CapitalError,
    providers::{BillingProvider, PolarProvider, SubscriptionStatus},
};
use std::collections::HashMap;

const PAYLOAD: &[u8] = br#"{"type":"subscription.updated","data":{"id":"sub_1","customer_id":"cus_1","product_id":"prod_1","status":"active"}}"#;

fn headers(key: &[u8], payload: &[u8], timestamp: i64) -> HashMap<String, String> {
    let id = "evt_fixture";
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, key));
    context.update(format!("{id}.{timestamp}.").as_bytes());
    context.update(payload);
    HashMap::from([
        ("webhook-id".into(), id.into()),
        ("webhook-timestamp".into(), timestamp.to_string()),
        (
            "webhook-signature".into(),
            format!("v1,{}", STANDARD.encode(context.sign().as_ref())),
        ),
    ])
}

#[test]
fn polar_standard_headers_bind_body_id_time_and_documented_key_schemes() {
    let now = chrono::Utc::now().timestamp();
    let key = b"a-long-deterministic-signing-key-fixture";
    let standard_secret = format!("whsec_{}", STANDARD.encode(key));
    for (secret, signing_key) in [
        ("legacy-literal-secret", b"legacy-literal-secret".as_slice()),
        (standard_secret.as_str(), key.as_slice()),
    ] {
        let provider = PolarProvider::new("fixture-key", secret);
        let valid = headers(signing_key, PAYLOAD, now);
        assert_eq!(
            provider.handle_webhook(PAYLOAD, &valid).unwrap().status,
            SubscriptionStatus::Active
        );
        assert!(matches!(
            provider.verify_signature(PAYLOAD, "body-only-hex"),
            Err(CapitalError::UnsupportedOperation(_))
        ));
        for removed in ["webhook-id", "webhook-timestamp", "webhook-signature"] {
            let mut missing = valid.clone();
            missing.remove(removed);
            assert!(provider.handle_webhook(PAYLOAD, &missing).is_err());
        }
        for (name, value) in [
            ("webhook-id", "other"),
            ("webhook-id", "evt_fixture,other"),
            ("webhook-timestamp", "not-a-time"),
            ("webhook-signature", "v1,invalid!"),
            ("webhook-signature", "v2,AAAA"),
        ] {
            let mut altered = valid.clone();
            altered.insert(name.into(), value.into());
            assert!(provider.handle_webhook(PAYLOAD, &altered).is_err());
        }
        let mut duplicate = valid.clone();
        duplicate.insert("Webhook-Id".into(), "evt_fixture".into());
        assert!(provider.handle_webhook(PAYLOAD, &duplicate).is_err());
        assert!(provider.handle_webhook(b"{}", &valid).is_err());
        // This public API reads the real clock. A now + 301 fixture can enter
        // the accepted 300-second window when the clock ticks during the test.
        // Exact +/-300 and +/-301 boundaries use a fixed clock in unit tests.
        for timestamp in [now - 3600, now + 3600] {
            assert!(
                provider
                    .handle_webhook(PAYLOAD, &headers(signing_key, PAYLOAD, timestamp))
                    .is_err()
            );
        }
        let mut rotated = valid.clone();
        rotated.insert(
            "webhook-signature".into(),
            format!(
                "v1,{} {}",
                STANDARD.encode([0; 32]),
                valid["webhook-signature"]
            ),
        );
        assert!(provider.handle_webhook(PAYLOAD, &rotated).is_ok());
        assert!(
            provider
                .handle_webhook(b"{}", &headers(signing_key, b"{}", now))
                .is_err()
        );
    }
}

fn subscription_fixture() -> serde_json::Value {
    serde_json::json!({"type":"subscription.updated","data":{
        "id":"sub_1","customer_id":"cus_1","product_id":"prod_1","status":"active",
        "current_period_end":"2027-01-01T00:00:00Z", "cancel_at_period_end":false,
        "customer":{"id":"cus_1","email":"billing@example.test"}
    }})
}

fn signed_event(value: &serde_json::Value) -> Result<rullst_capital::WebhookEvent, CapitalError> {
    let body = serde_json::to_vec(value).unwrap();
    let secret = "fixture-polar-lifecycle-secret";
    PolarProvider::new("unused-fixture-key", secret).handle_webhook(
        &body,
        &headers(secret.as_bytes(), &body, chrono::Utc::now().timestamp()),
    )
}

#[test]
fn current_polar_periods_and_customer_contact_are_preserved_with_legacy_integer_support() {
    let mut value = subscription_fixture();
    let event = signed_event(&value).unwrap();
    assert_eq!(event.ends_at, Some(1_798_761_600));
    assert_eq!(event.customer_email, "billing@example.test");
    value["data"]["current_period_end"] = serde_json::json!(1_798_761_600);
    assert_eq!(signed_event(&value).unwrap().ends_at, event.ends_at);
}

#[test]
fn signed_non_subscription_events_and_confused_subscription_objects_are_rejected() {
    use serde_json::json;
    for (pointer, replacement) in [
        ("/type", json!("order.paid")),
        ("/type", json!("customer.updated")),
        ("/type", json!("subscription.future")),
        ("/type", serde_json::Value::Null),
        ("/data/id", json!("")),
        ("/data/customer_id", json!("other")),
        ("/data/customer_id", serde_json::Value::Null),
        ("/data/product_id", json!("")),
        ("/data/status", json!("paid")),
        ("/data/current_period_end", json!("invalid")),
        ("/data/current_period_end", json!(-1)),
        ("/data/customer/email", json!("bad\ncontact")),
    ] {
        let mut value = subscription_fixture();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(signed_event(&value).is_err(), "accepted {pointer}");
    }
    let mut value = subscription_fixture();
    value["data"]["user_id"] = json!("other");
    assert!(signed_event(&value).is_err());
}

#[test]
fn polar_scheduled_cancellation_and_final_revocation_are_distinct() {
    use serde_json::json;
    let mut value = subscription_fixture();
    value["type"] = json!("subscription.canceled");
    value["data"]["cancel_at_period_end"] = json!(true);
    assert_eq!(
        signed_event(&value).unwrap().status,
        SubscriptionStatus::Active
    );
    value["type"] = json!("subscription.revoked");
    assert!(signed_event(&value).is_err());
    value["data"]["status"] = json!("canceled");
    assert_eq!(
        signed_event(&value).unwrap().status,
        SubscriptionStatus::Canceled
    );
    value["type"] = json!("subscription.paused");
    assert!(signed_event(&value).is_err());
    value["data"]["status"] = json!("paused");
    assert_eq!(
        signed_event(&value).unwrap().status,
        SubscriptionStatus::Paused
    );
}
