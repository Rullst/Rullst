use super::*;
use serde_json::json;

fn lookup() -> StripeSubscriptionLookup {
    StripeSubscriptionLookup::new(
        "sub_lookup",
        "cus_lookup",
        "owner_lookup",
        "price_lookup",
        false,
    )
    .unwrap()
}

fn body() -> Value {
    json!({"id":"sub_lookup","object":"subscription","customer":"cus_lookup",
        "status":"active","livemode":false,"metadata":{"rullst_owner_reference":"owner_lookup"},
        "items":{"has_more":false,"data":[{"subscription":"sub_lookup",
            "price":{"id":"price_lookup","type":"recurring"},"current_period_end":1900000000}]}})
}

#[test]
fn lookup_request_is_a_pinned_read_and_rejects_confused_credential_mode() {
    let client = reqwest::Client::new();
    let request = build_request(&client, "sk_test_fixture", &lookup()).unwrap();
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().as_str(),
        "https://api.stripe.com/v1/subscriptions/sub_lookup"
    );
    assert_eq!(request.headers()["Stripe-Version"], "2025-03-31.basil");
    assert!(request.body().is_none());
    assert!(!request.headers().contains_key("Idempotency-Key"));
    assert!(build_request(&client, "sk_live_fixture", &lookup()).is_err());
    assert!(build_request(&client, "rk_live_fixture", &lookup()).is_err());
    for id in [
        "",
        "sub_",
        "sub_../escape",
        "sub_foo?expand=customer",
        "cus_wrong",
    ] {
        assert!(StripeSubscriptionLookup::new(id, "cus_fixture", "owner", "price", false).is_err());
    }
    assert!(
        StripeSubscriptionLookup::new("sub_ok", "cus_ok", "owner", "x".repeat(201), false).is_err()
    );
    assert!(StripeSubscriptionLookup::new("sub_ok", "sub_wrong", "owner", "price", false).is_err());
    assert!(
        StripeSubscriptionLookup::new(
            "sub_ok",
            "cus_ok",
            "owner email@example.com",
            "price",
            false
        )
        .is_err()
    );
}

#[test]
fn lookup_binds_customer_owner_price_subscription_and_mode() {
    let lookup = lookup();
    let good = body();
    let snapshot = parse_response(&lookup, &good).unwrap();
    snapshot.require_real().unwrap();
    assert_eq!(snapshot.source(), StripeSubscriptionSource::Retrieved);
    assert_eq!(snapshot.subscription().ends_at, Some(1900000000));
    assert_eq!(snapshot.livemode(), Some(false));
    assert_eq!(snapshot.owner_reference(), "owner_lookup");
    for (path, bad) in [
        ("/id", json!("sub_other")),
        ("/object", json!("customer")),
        ("/customer", json!("cus_other")),
        ("/livemode", json!(true)),
        ("/metadata/rullst_owner_reference", json!("owner_other")),
        ("/items/data/0/price/id", json!("price_other")),
        ("/items/data/0/subscription", json!("sub_other")),
        ("/items/data/0/price/type", json!("one_time")),
        ("/items/has_more", json!(true)),
    ] {
        let mut wrong = good.clone();
        *wrong.pointer_mut(path).unwrap() = bad;
        assert!(parse_response(&lookup, &wrong).is_err(), "accepted {path}");
        *wrong.pointer_mut(path).unwrap() = Value::Null;
        if path != "/items/data/0/subscription" {
            assert!(
                parse_response(&lookup, &wrong).is_err(),
                "accepted missing {path}"
            );
        }
    }
    let mut wrong = good.clone();
    wrong["deleted"] = json!(true);
    assert!(parse_response(&lookup, &wrong).is_err());
    wrong = good.clone();
    wrong["items"]["data"]
        .as_array_mut()
        .unwrap()
        .push(good["items"]["data"][0].clone());
    assert!(parse_response(&lookup, &wrong).is_err());
    wrong = good;
    wrong["current_period_end"] = json!(1900000001);
    assert!(parse_response(&lookup, &wrong).is_err());
}

#[test]
fn current_snapshot_preserves_all_subscription_states_and_redacts_identity() {
    for (provider_status, normalized) in [
        ("active", SubscriptionStatus::Active),
        ("trialing", SubscriptionStatus::Trialing),
        ("canceled", SubscriptionStatus::Canceled),
        ("paused", SubscriptionStatus::Paused),
        ("past_due", SubscriptionStatus::PastDue),
        ("unpaid", SubscriptionStatus::Unpaid),
        ("incomplete", SubscriptionStatus::Unpaid),
        ("incomplete_expired", SubscriptionStatus::Unpaid),
    ] {
        let mut body = body();
        body["status"] = json!(provider_status);
        let snapshot = parse_response(&lookup(), &body).unwrap();
        assert_eq!(snapshot.provider_status(), provider_status);
        assert_eq!(snapshot.subscription().status, normalized);
        let debug = format!("{:?} {snapshot:?}", lookup());
        for private in ["cus_lookup", "sub_lookup", "price_lookup", "owner_lookup"] {
            assert!(!debug.contains(private));
        }
    }
    for status in [json!("paid"), json!("succeeded"), json!(null)] {
        let mut wrong = body();
        wrong["status"] = status;
        assert!(parse_response(&lookup(), &wrong).is_err());
    }
}

#[tokio::test]
async fn mock_reads_are_deterministic_non_entitled_and_rejected_as_real_evidence() {
    for key in ["", "mock_key"] {
        let provider = super::super::StripeProvider::new(key, "mock_webhook");
        let first = provider.retrieve_subscription(&lookup()).await.unwrap();
        let second = provider.retrieve_subscription(&lookup()).await.unwrap();
        assert_eq!(first.source(), StripeSubscriptionSource::Mock);
        assert_eq!(first.livemode(), None);
        assert_eq!(first.subscription().status, SubscriptionStatus::Unpaid);
        assert_eq!(first.provider_status(), "incomplete");
        assert_eq!(
            first.subscription().subscription_id,
            second.subscription().subscription_id
        );
        assert!(first.require_real().is_err());
    }
}
