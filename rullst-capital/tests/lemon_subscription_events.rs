//! Real-HMAC protocol fixtures, not provider sandbox acceptance.
use ring::hmac;
use rullst_capital::{BillingProvider, LemonSqueezyProvider, SubscriptionStatus};
use serde_json::{Value, json};
use std::collections::HashMap;

fn payload(kind: &str, status: &str) -> Value {
    json!({"meta":{"event_name":kind,"test_mode":true},
        "data":{"type":"subscriptions","id":"123","attributes":{
            "customer_id":456,"variant_id":789,"store_id":42,"test_mode":true,
            "status":status,"ends_at":null}}})
}

fn decode(body: Value) -> Result<rullst_capital::WebhookEvent, rullst_capital::CapitalError> {
    let bytes = serde_json::to_vec(&body).unwrap();
    let signature = hex::encode(hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, b"fixture-secret"),
        &bytes,
    ));
    LemonSqueezyProvider::new("fixture-key", "fixture-secret")
        .with_store_id("42")
        .unwrap()
        .handle_webhook(&bytes, &HashMap::from([("x-signature".into(), signature)]))
}

#[test]
fn lemon_normalizes_subscription_states_without_payment_aliases() {
    for (status, expected) in [
        ("active", SubscriptionStatus::Active),
        ("on_trial", SubscriptionStatus::Trialing),
        ("paused", SubscriptionStatus::Paused),
        ("past_due", SubscriptionStatus::PastDue),
        ("unpaid", SubscriptionStatus::Unpaid),
        ("cancelled", SubscriptionStatus::Canceled),
        ("expired", SubscriptionStatus::Canceled),
    ] {
        let mut body = payload("subscription_updated", status);
        if matches!(status, "cancelled" | "expired") {
            body["data"]["attributes"]["ends_at"] = json!("2030-03-17T12:00:00Z");
        }
        let event = decode(body).unwrap();
        assert_eq!(event.status, expected, "{status}");
        assert_eq!(event.subscription_id, "123");
        assert_eq!(event.customer_id, "456");
        assert_eq!(event.plan_id, "789");
        assert_eq!(event.customer_email, "");
        if matches!(status, "cancelled" | "expired") {
            assert!(event.ends_at.is_some());
        }
    }
    for status in [
        "paid",
        "approved",
        "completed",
        "trialing",
        "future-status",
        "",
    ] {
        assert!(
            decode(payload("subscription_updated", status)).is_err(),
            "{status}"
        );
    }
}

#[test]
fn lemon_requires_subscription_kind_identity_store_and_mode() {
    let good = payload("subscription_created", "active");
    for (path, invalid) in [
        ("/meta/event_name", json!("subscription_payment_success")),
        ("/data/type", json!("subscription-invoices")),
        ("/data/id", json!("null")),
        ("/data/attributes/customer_id", json!({})),
        ("/data/attributes/variant_id", json!(0)),
        ("/data/attributes/store_id", json!(43)),
        ("/data/attributes/test_mode", json!("true")),
    ] {
        let mut wrong = good.clone();
        *wrong.pointer_mut(path).unwrap() = invalid;
        assert!(decode(wrong.clone()).is_err(), "{path}");
        *wrong.pointer_mut(path).unwrap() = Value::Null;
        assert!(decode(wrong).is_err(), "missing {path}");
    }
    for field in ["id", "attributes/customer_id", "attributes/variant_id"] {
        for invalid in [
            json!(-1),
            json!(1.5),
            json!("01"),
            json!("1 "),
            json!("9".repeat(300)),
        ] {
            let mut wrong = good.clone();
            *wrong.pointer_mut(&format!("/data/{field}")).unwrap() = invalid;
            assert!(decode(wrong).is_err());
        }
    }
    let mut wrong = good.clone();
    wrong["meta"]["test_mode"] = json!(false);
    assert!(decode(wrong).is_err());
    let mut wrong = good;
    wrong["data"]["attributes"]["user_email"] = json!("injected\ncontact");
    assert!(decode(wrong).is_err());
}

#[test]
fn lemon_lifecycle_rejects_confused_events_and_malformed_expiry() {
    for (kind, status) in [
        ("subscription_cancelled", "cancelled"),
        ("subscription_expired", "expired"),
        ("subscription_paused", "paused"),
        ("subscription_resumed", "active"),
        ("subscription_unpaused", "active"),
    ] {
        let mut body = payload(kind, status);
        if matches!(status, "cancelled" | "expired") {
            body["data"]["attributes"]["ends_at"] = json!("2030-03-17T12:00:00Z");
        }
        assert!(decode(body.clone()).is_ok(), "{kind}");
        body["data"]["attributes"]["status"] = json!(if status == "active" {
            "cancelled"
        } else {
            "active"
        });
        assert!(decode(body).is_err(), "conflicting {kind}");
    }
    for kind in [
        "subscription_payment_success",
        "subscription_payment_failed",
        "subscription_payment_refunded",
        "order_created",
        "subscription_future",
    ] {
        assert!(decode(payload(kind, "active")).is_err(), "{kind}");
    }
    for end in [
        Value::Null,
        json!("tomorrow"),
        json!(123),
        json!("1960-01-01T00:00:00Z"),
    ] {
        let mut wrong = payload("subscription_cancelled", "cancelled");
        wrong["data"]["attributes"]["ends_at"] = end;
        assert!(decode(wrong).is_err());
    }
}
