//! Signed Paddle deliveries: only subscription lifecycle events become state.
use ring::hmac;
use rullst_capital::{
    BillingProvider, CapitalError, PaddleProvider, SubscriptionStatus, WebhookEvent,
};
use serde_json::{Value, json};
use std::collections::HashMap;

const SUBSCRIPTION: &str = "sub_01hv8x29kz0t586xy6zn1a62ny";
const CUSTOMER: &str = "ctm_01hv6y1jedq4p1n0yqn5ba3ky4";
const PRICE: &str = "pri_01gsz8x8sawmvhz1pv30nge1ke";
const KEY: &str = "pdl_ntfset_legacy_contract_fixture";

fn subscription(kind: &str, status: &str) -> Value {
    json!({"event_id":"evt_01hv8x2af22vrrz7k67g06x1kq","event_type":kind,"data":{
        "id":SUBSCRIPTION, "customer_id":CUSTOMER, "status":status,
        "items":[{"price":{"id":PRICE}}],
        "current_billing_period":{"starts_at":"2026-09-01T00:00:00Z","ends_at":"2026-10-01T00:00:00Z"}
    }})
}

fn verify(value: &Value) -> Result<WebhookEvent, CapitalError> {
    let body = serde_json::to_vec(value).unwrap();
    let now = chrono::Utc::now().timestamp();
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, KEY.as_bytes()));
    context.update(format!("{now}:").as_bytes());
    context.update(&body);
    let header = format!("ts={now};h1={}", hex::encode(context.sign().as_ref()));
    PaddleProvider::new("fixture-key", KEY).handle_webhook(
        &body,
        &HashMap::from([("paddle-signature".to_string(), header)]),
    )
}

fn rejected(value: &Value) -> bool {
    matches!(verify(value), Err(CapitalError::PayloadParseError(_)))
}

#[test]
fn signed_non_subscription_events_are_not_subscription_state() {
    let chargeback = json!({"event_type":"adjustment.created","data":{
        "id":"adj_01hv8x29kz0t586xy6zn1a62ny", "action":"chargeback", "status":"approved",
        "customer_id":CUSTOMER, "subscription_id":SUBSCRIPTION
    }});
    let one_time = json!({"event_type":"transaction.completed","data":{
        "id":"txn_01hv8m0mnx3sj85e7gxc6kga03", "status":"completed", "customer_id":CUSTOMER,
        "items":[{"price":{"id":PRICE}}]
    }});
    assert!(rejected(&chargeback));
    assert!(rejected(&one_time));
    for kind in [
        "transaction.paid",
        "customer.updated",
        "price.created",
        "address.created",
        "subscription.future_event",
    ] {
        assert!(rejected(&subscription(kind, "active")), "{kind}");
    }
    let mut untyped = subscription("subscription.updated", "active");
    untyped.as_object_mut().unwrap().remove("event_type");
    assert!(rejected(&untyped));
}

#[test]
fn subscription_events_map_only_paddle_statuses_that_agree_with_the_event() {
    for (kind, state, status) in [
        (
            "subscription.created",
            "trialing",
            SubscriptionStatus::Trialing,
        ),
        (
            "subscription.updated",
            "past_due",
            SubscriptionStatus::PastDue,
        ),
        (
            "subscription.imported",
            "active",
            SubscriptionStatus::Active,
        ),
        (
            "subscription.activated",
            "active",
            SubscriptionStatus::Active,
        ),
        ("subscription.resumed", "active", SubscriptionStatus::Active),
        (
            "subscription.trialing",
            "trialing",
            SubscriptionStatus::Trialing,
        ),
        (
            "subscription.past_due",
            "past_due",
            SubscriptionStatus::PastDue,
        ),
        ("subscription.paused", "paused", SubscriptionStatus::Paused),
        (
            "subscription.canceled",
            "canceled",
            SubscriptionStatus::Canceled,
        ),
    ] {
        let event = verify(&subscription(kind, state)).unwrap();
        assert_eq!(event.status, status, "{kind}");
        assert_eq!(event.subscription_id, SUBSCRIPTION);
        assert_eq!(event.customer_id, CUSTOMER);
        assert_eq!(event.plan_id, PRICE);
        assert_eq!(event.ends_at, Some(1_790_812_800));
        assert!(event.customer_email.is_empty());
    }
    for (kind, state) in [
        ("subscription.activated", "canceled"),
        ("subscription.canceled", "active"),
        ("subscription.paused", "active"),
        ("subscription.updated", "completed"),
        ("subscription.updated", "approved"),
        ("subscription.updated", "paid"),
    ] {
        assert!(rejected(&subscription(kind, state)), "{kind} {state}");
    }
}

#[test]
fn subscription_identities_are_required_instead_of_defaulted() {
    let good = subscription("subscription.updated", "active");
    for (pointer, wrong) in [
        ("/data/id", json!("txn_01hv8m0mnx3sj85e7gxc6kga03")),
        ("/data/id", json!("sub_pad_100")),
        ("/data/customer_id", json!("ct_999")),
        (
            "/data/items/0/price/id",
            json!("pro_01gsz8x8sawmvhz1pv30nge1ke"),
        ),
        ("/data/current_billing_period/ends_at", json!("next month")),
        ("/data/status", json!(null)),
    ] {
        let mut payload = good.clone();
        *payload.pointer_mut(pointer).unwrap() = wrong;
        assert!(rejected(&payload), "{pointer}");
    }
    for field in ["id", "customer_id", "items", "status"] {
        let mut payload = good.clone();
        payload["data"].as_object_mut().unwrap().remove(field);
        assert!(rejected(&payload), "{field}");
    }
    let mut unscheduled = subscription("subscription.canceled", "canceled");
    unscheduled["data"]["current_billing_period"] = Value::Null;
    assert_eq!(verify(&unscheduled).unwrap().ends_at, None);
}
