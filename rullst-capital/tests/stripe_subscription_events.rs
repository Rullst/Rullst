use ring::hmac;
use rullst_capital::{BillingProvider, CapitalError, StripeProvider, SubscriptionStatus};
use serde_json::{Value, json};
use std::collections::HashMap;

fn snapshot() -> Value {
    json!({
        "id": "evt_fixture", "object": "event", "api_version": "2025-03-31.basil",
        "type": "customer.subscription.updated", "livemode": false,
        "data": {"object": {
            "id": "sub_fixture", "object": "subscription", "customer": "cus_fixture",
            "status": "active", "items": {"has_more": false, "data": [{
                "id": "si_fixture", "subscription": "sub_fixture", "quantity": 1,
                "price": {"id": "price_fixture", "type": "recurring"},
                "current_period_end": 1900000000
            }]}
        }}
    })
}

fn signed(value: &Value) -> Result<rullst_capital::WebhookEvent, CapitalError> {
    let bytes = serde_json::to_vec(value).unwrap();
    let timestamp = chrono::Utc::now().timestamp();
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, b"whsec_fixture"));
    context.update(format!("{timestamp}.").as_bytes());
    context.update(&bytes);
    let signature = format!("t={timestamp},v1={}", hex::encode(context.sign()));
    StripeProvider::new("sk_test_fixture", "whsec_fixture").handle_webhook(
        &bytes,
        &HashMap::from([("stripe-signature".into(), signature)]),
    )
}

#[test]
fn signed_basil_and_legacy_periods_preserve_identity_without_requiring_email() {
    let mut fixture = snapshot();
    let event = signed(&fixture).unwrap();
    assert_eq!(event.subscription_id, "sub_fixture");
    assert_eq!(event.customer_id, "cus_fixture");
    assert_eq!(event.plan_id, "price_fixture");
    assert!(event.customer_email.is_empty());
    assert_eq!(event.ends_at, Some(1900000000));

    fixture["api_version"] = json!("2024-10-28.acacia");
    let data = &mut fixture["data"]["object"];
    data["items"]["data"][0]["current_period_end"] = Value::Null;
    data["current_period_end"] = json!(1900000001);
    assert_eq!(signed(&fixture).unwrap().ends_at, Some(1900000001));

    let data = &mut fixture["data"]["object"];
    data["items"]["data"][0]["plan"] = json!({"id": "legacy_custom_plan"});
    data["items"]["data"][0]["price"] = Value::Null;
    assert_eq!(signed(&fixture).unwrap().plan_id, "legacy_custom_plan");
}

#[test]
fn signed_subscription_states_are_explicit_and_inconsistent_lifecycles_fail() {
    for (status, expected) in [
        ("active", SubscriptionStatus::Active),
        ("canceled", SubscriptionStatus::Canceled),
        ("trialing", SubscriptionStatus::Trialing),
        ("past_due", SubscriptionStatus::PastDue),
        ("unpaid", SubscriptionStatus::Unpaid),
        ("paused", SubscriptionStatus::Paused),
        ("incomplete", SubscriptionStatus::Unpaid),
        ("incomplete_expired", SubscriptionStatus::Unpaid),
    ] {
        let mut fixture = snapshot();
        fixture["data"]["object"]["status"] = json!(status);
        assert_eq!(signed(&fixture).unwrap().status, expected);
    }
    for (event, status) in [
        ("customer.subscription.created", "trialing"),
        ("customer.subscription.deleted", "canceled"),
        ("customer.subscription.paused", "paused"),
        ("customer.subscription.resumed", "active"),
    ] {
        let mut fixture = snapshot();
        fixture["type"] = json!(event);
        fixture["data"]["object"]["status"] = json!(status);
        assert!(signed(&fixture).is_ok(), "rejected {event}");
        fixture["data"]["object"]["status"] = json!("paid");
        assert!(signed(&fixture).is_err());
    }
    for (event, status) in [
        ("customer.subscription.deleted", "active"),
        ("customer.subscription.paused", "active"),
        ("customer.subscription.resumed", "paused"),
        ("customer.subscription.resumed", "canceled"),
        ("invoice.paid", "active"),
        ("customer.subscription.trial_will_end", "active"),
    ] {
        let mut fixture = snapshot();
        fixture["type"] = json!(event);
        fixture["data"]["object"]["status"] = json!(status);
        assert!(signed(&fixture).is_err(), "accepted {event}/{status}");
    }
}

#[test]
fn signed_confused_identities_periods_and_truncated_prices_are_rejected() {
    for (pointer, invalid) in [
        ("/type", Value::Null),
        ("/data/object/object", json!("invoice")),
        ("/data/object/id", json!("in_invoice")),
        ("/data/object/customer", json!("sub_other")),
        ("/data/object/status", Value::Null),
        ("/data/object/status", json!("approved")),
        ("/data/object/status", json!("unknown")),
        ("/data/object/items/has_more", json!(true)),
        ("/data/object/items/data/0/subscription", json!("sub_other")),
        ("/data/object/items/data/0/price/id", json!("")),
        ("/data/object/items/data/0/price/id", json!("a".repeat(201))),
        ("/data/object/items/data/0/price/type", json!("one_time")),
        ("/data/object/items/data/0/current_period_end", json!(0)),
        (
            "/data/object/items/data/0/current_period_end",
            json!("1900000000"),
        ),
    ] {
        let mut fixture = snapshot();
        *fixture.pointer_mut(pointer).unwrap() = invalid;
        assert!(signed(&fixture).is_err(), "accepted {pointer}");
    }
    let mut fixture = snapshot();
    fixture["data"]["object"]["current_period_end"] = json!(1900000001);
    assert!(signed(&fixture).is_err());
    fixture["data"]["object"]["current_period_end"] = json!(1900000000);
    assert!(signed(&fixture).is_ok());
    let item = fixture["data"]["object"]["items"]["data"][0].clone();
    fixture["data"]["object"]["items"]["data"] = json!([item.clone(), item]);
    assert!(signed(&fixture).is_err());
    fixture["data"]["object"]["items"]["data"] = json!([]);
    assert!(signed(&fixture).is_err());
}
