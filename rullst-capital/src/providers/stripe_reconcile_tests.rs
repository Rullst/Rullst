use super::*;
use serde_json::json;
fn request() -> StripeCheckoutRequest {
    StripeCheckoutRequest::new(
        "cus_owner",
        "price_pro",
        "owner_opaque",
        "attempt_fixed",
        "https://app.example/return",
        "https://app.example/cancel",
    )
    .unwrap()
}
fn body() -> Value {
    json!({"id":"cs_fixed", "object":"checkout.session", "mode":"subscription", "status":"complete",
        "customer":"cus_owner", "client_reference_id":"owner_opaque", "livemode":false,
        "metadata":{"rullst_owner_reference":"owner_opaque","rullst_attempt_reference":"attempt_fixed"},
        "success_url":"https://app.example/return", "cancel_url":"https://app.example/cancel",
        "subscription":"sub_owner", "url":null, "expires_at":1800000000,
        "line_items":{"object":"list","has_more":false,"data":[{"quantity":1,"price":{"id":"price_pro","type":"recurring"}}]}})
}
fn configuration<T: std::fmt::Debug>(result: Result<T, CapitalError>) -> bool {
    matches!(result, Err(CapitalError::ConfigurationError(_)))
}

#[tokio::test]
async fn confused_mode_and_unsafe_identifiers_fail_before_http() {
    let provider = super::super::StripeProvider::new("sk_test_fixture", "whsec_fixture");
    let customer = StripeCustomerRequest::new("owner", "intent").unwrap();
    assert!(configuration(
        provider
            .retrieve_checkout(&request(), "cs_fixed", true)
            .await
    ));
    assert!(configuration(
        provider
            .retrieve_checkout(&request(), "cs_../other", false)
            .await
    ));
    assert!(configuration(
        provider
            .find_checkout(&request(), 1800000000, None, true)
            .await
    ));
    assert!(configuration(
        provider.find_checkout(&request(), 0, None, false).await
    ));
    assert!(configuration(
        provider
            .find_checkout(&request(), 1800000000, Some("sub_?query"), false)
            .await
    ));
    assert!(configuration(
        provider.verify_account("acct_other", true).await
    ));
    assert!(configuration(
        provider.verify_account("acct_../x", false).await
    ));
    assert!(configuration(
        provider
            .create_bound_customer_portal("cus_../other", "https://app.example")
            .await
    ));
    assert!(configuration(
        provider
            .create_bound_customer_portal("cus_owner", "http://app.example")
            .await
    ));
    assert!(configuration(
        provider
            .retrieve_bound_customer(&customer, "cus_../other")
            .await
    ));
}

#[tokio::test]
async fn offline_and_unknown_keys_are_not_reported_as_provider_drift() {
    let customer = StripeCustomerRequest::new("owner", "intent").unwrap();
    for key in ["", "mock_key"] {
        let provider = super::super::StripeProvider::new(key, "whsec_fixture");
        let unsupported = |result: Result<(), CapitalError>| {
            matches!(result, Err(CapitalError::UnsupportedOperation(_)))
        };
        assert!(unsupported(
            provider
                .retrieve_checkout(&request(), "cs_fixed", false)
                .await
                .map(drop)
        ));
        assert!(unsupported(
            provider
                .find_checkout(&request(), 1800000000, None, false)
                .await
                .map(drop)
        ));
        assert!(unsupported(provider.verify_account("acct_1", false).await));
        assert!(unsupported(
            provider
                .retrieve_bound_customer(&customer, "cus_owner")
                .await
                .map(drop)
        ));
        assert!(unsupported(
            provider.find_bound_customer(&customer).await.map(drop)
        ));
    }
    let publishable = super::super::StripeProvider::new("pk_test_fixture", "whsec_fixture");
    assert!(configuration(
        publishable.find_bound_customer(&customer).await
    ));
}

#[test]
fn completed_checkout_binds_every_identity_without_contact_data() {
    let request = request();
    let original = body();
    let snapshot = parse_checkout(&request, &original, false).unwrap();
    assert_eq!(snapshot.subscription_id(), Some("sub_owner"));
    assert!(snapshot.url().is_none());
    for path in [
        "/id",
        "/customer",
        "/client_reference_id",
        "/metadata/rullst_owner_reference",
        "/metadata/rullst_attempt_reference",
        "/subscription",
        "/mode",
        "/status",
        "/line_items/data/0/price/id",
        "/line_items/data/0/price/type",
        "/success_url",
        "/cancel_url",
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = json!("wrong");
        assert!(parse_checkout(&request, &changed, false).is_err(), "{path}");
    }
    assert!(parse_checkout(&request, &original, true).is_err());
    for (path, value) in [
        ("/line_items/has_more", json!(true)),
        ("/line_items/data/0/quantity", json!(2)),
        ("/subscription", Value::Null),
        ("/expires_at", json!(0)),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = value;
        assert!(parse_checkout(&request, &changed, false).is_err());
    }
}
#[test]
fn open_expired_and_truncated_recovery_contracts() {
    let mut body = body();
    body["status"] = json!("open");
    body["subscription"] = Value::Null;
    assert!(parse_checkout(&request(), &body, false).is_err());
    body["url"] = json!("https://checkout.stripe.com/c/pay/cs_fixed#opaque");
    assert!(
        parse_checkout(&request(), &body, false)
            .unwrap()
            .url()
            .unwrap()
            .ends_with("#opaque")
    );
    body["status"] = json!("expired");
    assert!(
        parse_checkout(&request(), &body, false)
            .unwrap()
            .url()
            .is_none()
    );
    assert!(bounded_list(&json!({"object":"list","data":[],"has_more":true})).is_err());
    assert!(
        bounded_list(&json!({"object":"list","data":[],"has_more":false}))
            .unwrap()
            .is_empty()
    );
}
