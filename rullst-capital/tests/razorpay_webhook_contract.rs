//! HMAC-authenticated fixtures: signature validity alone is not an entitlement.
use ring::hmac;
use rullst_capital::{
    BillingProvider, CapitalError, RazorpayProvider, SubscriptionStatus, WebhookEvent,
};
use serde_json::{Value, json};
use std::collections::HashMap;

fn event(kind: &str, state: &str) -> Value {
    json!({"event":kind,"payload":{"subscription":{"entity":{
        "entity":"subscription", "id":"sub_fixture", "customer_id":"cust_fixture",
        "plan_id":"plan_fixture", "status":state, "current_end":1800000000
    }}}})
}

fn verify(value: &Value) -> Result<WebhookEvent, CapitalError> {
    let body = serde_json::to_vec(value).unwrap();
    let signature = hex::encode(hmac::sign(
        &hmac::Key::new(hmac::HMAC_SHA256, b"secret"),
        &body,
    ));
    RazorpayProvider::new("key", "secret", "secret").handle_webhook(
        &body,
        &HashMap::from([("x-razorpay-signature".to_string(), signature)]),
    )
}

#[test]
fn accepted_subscription_events_preserve_identity_and_agree_with_entity_state() {
    for (kind, state, status) in [
        ("activated", "active", SubscriptionStatus::Active),
        ("charged", "active", SubscriptionStatus::Active),
        ("resumed", "active", SubscriptionStatus::Active),
        ("cancelled", "cancelled", SubscriptionStatus::Canceled),
        ("pending", "pending", SubscriptionStatus::PastDue),
        ("halted", "halted", SubscriptionStatus::Unpaid),
        ("paused", "paused", SubscriptionStatus::Paused),
    ] {
        let mut payload = event(&format!("subscription.{kind}"), state);
        let result = verify(&payload).unwrap();
        assert_eq!(result.status, status);
        assert_eq!(result.subscription_id, "sub_fixture");
        assert_eq!(result.customer_id, "cust_fixture");
        assert_eq!(result.plan_id, "plan_fixture");
        assert_eq!(result.ends_at, Some(1800000000));
        assert!(result.customer_email.is_empty());
        payload["payload"]["subscription"]["entity"]["status"] = json!("inconsistent");
        assert!(verify(&payload).is_err());
    }
}

#[test]
fn authentication_payments_and_orders_are_not_active_subscriptions() {
    for kind in [
        "subscription.authenticated",
        "payment.captured",
        "payment.failed",
        "order.paid",
        "subscription.completed",
    ] {
        assert!(verify(&event(kind, "active")).is_err(), "{kind}");
    }
    assert!(verify(&event("subscription.authenticated", "authenticated")).is_err());
    let payment_only = json!({"event":"payment.captured","payload":{"payment":{"entity":{
        "id":"pay_fixture", "order_id":"order_fixture", "customer_id":"cust_fixture", "status":"captured"
    }}}});
    assert!(verify(&payment_only).is_err());
}

#[test]
fn missing_confused_and_malformed_subscription_identity_is_rejected() {
    let good = event("subscription.activated", "active");
    for field in ["entity", "id", "customer_id", "plan_id", "status"] {
        let mut payload = good.clone();
        payload["payload"]["subscription"]["entity"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(verify(&payload).is_err(), "missing {field}");
    }
    for (field, wrong) in [
        ("entity", json!("order")),
        ("id", json!("order_fixture")),
        ("id", json!("sub_")),
        ("plan_id", json!("plan_../../secret")),
        ("customer_id", json!("cust_".to_owned() + &"a".repeat(201))),
        ("current_end", json!(-1)),
        ("current_end", json!("1800000000")),
    ] {
        let mut payload = good.clone();
        payload["payload"]["subscription"]["entity"][field] = wrong;
        assert!(verify(&payload).is_err(), "invalid {field}");
    }
    let mut mismatched_customer = good;
    mismatched_customer["payload"]["payment"] = json!({"entity":{"customer_id":"cust_other"}});
    assert!(verify(&mismatched_customer).is_err());
}
