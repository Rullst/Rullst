use super::*;
use crate::{StripeOneTimePrice, StripeProvider};
use serde_json::json;

fn request() -> StripePaymentCheckoutRequest {
    StripePaymentCheckoutRequest::new(
        "cus_owner",
        StripeOneTimePrice::new("price_once", 1990, "brl").unwrap(),
        "opaque_owner",
        "purchase_attempt_1",
        "https://app.example/success",
        "https://app.example/cancel",
    )
    .unwrap()
}

fn price() -> Value {
    json!({"id":"price_once","object":"price","type":"one_time","active":true,"livemode":false,"unit_amount":1990,"currency":"brl","recurring":null})
}

fn session() -> Value {
    json!({"id":"cs_test_once","object":"checkout.session","mode":"payment","status":"open","payment_status":"unpaid",
        "livemode":false,"customer":"cus_owner","client_reference_id":"opaque_owner","amount_total":1990,"currency":"brl",
        "success_url":"https://app.example/success","cancel_url":"https://app.example/cancel","expires_at":2000000000,
        "url":"https://checkout.stripe.com/c/pay/cs_test_once#allowed_fragment", "subscription":null,
        "metadata":{"rullst_owner_reference":"opaque_owner","rullst_attempt_reference":"purchase_attempt_1"},
        "line_items":{"has_more":false,"data":[{"quantity":1,"price":price()}]}})
}

fn paid() -> Value {
    let mut value = session();
    value["status"] = json!("complete");
    value["payment_status"] = json!("paid");
    value["payment_intent"] = json!({"id":"pi_once","object":"payment_intent","status":"succeeded","customer":"cus_owner",
        "amount_received":1990,"currency":"brl","livemode":false,
        "metadata":{"rullst_owner_reference":"opaque_owner","rullst_attempt_reference":"purchase_attempt_1"},
        "latest_charge":{"id":"ch_once","object":"charge","payment_intent":"pi_once","customer":"cus_owner","status":"succeeded",
        "paid":true,"captured":true,"livemode":false,"amount":1990,"amount_captured":1990,"currency":"brl","amount_refunded":0,"disputed":false}});
    value
}

#[test]
fn one_time_wire_has_distinct_mode_amount_binding_and_idempotency() {
    let request = request();
    assert_ne!(request.request_digest(), request.checkout.request_digest());
    let http = build_request(http_client().unwrap(), "sk_test_fixture", &request).unwrap();
    assert_eq!(http.headers()["Idempotency-Key"], "purchase_attempt_1");
    let body = std::str::from_utf8(http.body().unwrap().as_bytes().unwrap()).unwrap();
    assert!(body.contains("mode=payment&"));
    assert!(!body.contains("subscription_data"));
    assert!(body.contains("payment_intent_data[metadata][rullst_owner_reference]=opaque_owner"));
    assert!(body.contains("payment_method_types[0]=card"));
    assert!(validate_price(&request, &price(), true).is_ok());
    for (field, bad) in [
        ("id", json!("price_other")),
        ("type", json!("recurring")),
        ("active", json!(false)),
        ("unit_amount", json!(1991)),
        ("currency", json!("usd")),
    ] {
        let mut value = price();
        value[field] = bad;
        assert!(validate_price(&request, &value, true).is_err(), "{field}");
    }
}

#[test]
fn paid_receipt_requires_all_bindings_and_revokes_on_refund_or_dispute() {
    let request = request();
    let baseline = paid();
    assert_eq!(
        parse_receipt("cs_test_once", &request, &baseline)
            .unwrap()
            .state(),
        StripeOneTimePaymentState::Paid
    );
    for pointer in [
        "/customer",
        "/client_reference_id",
        "/metadata/rullst_owner_reference",
        "/metadata/rullst_attempt_reference",
        "/amount_total",
        "/currency",
        "/line_items/data/0/price/id",
        "/line_items/data/0/price/unit_amount",
        "/payment_intent/customer",
        "/payment_intent/amount_received",
        "/payment_intent/currency",
        "/payment_intent/metadata/rullst_owner_reference",
        "/payment_intent/latest_charge/payment_intent",
        "/payment_intent/latest_charge/customer",
        "/payment_intent/latest_charge/amount",
        "/payment_intent/latest_charge/paid",
        "/payment_intent/latest_charge/captured",
        "/payment_intent/latest_charge/livemode",
    ] {
        let mut invalid = baseline.clone();
        *invalid.pointer_mut(pointer).unwrap() = Value::Null;
        assert!(
            parse_receipt("cs_test_once", &request, &invalid).is_err(),
            "{pointer}"
        );
    }
    assert!(parse_receipt("cs_other", &request, &baseline).is_err());
    let mut refunded = baseline.clone();
    refunded["payment_intent"]["latest_charge"]["amount_refunded"] = json!(1);
    assert_eq!(
        parse_receipt("cs_test_once", &request, &refunded)
            .unwrap()
            .state(),
        StripeOneTimePaymentState::Refunded
    );
    let mut disputed = baseline;
    disputed["payment_intent"]["latest_charge"]["disputed"] = json!(true);
    assert_eq!(
        parse_receipt("cs_test_once", &request, &disputed)
            .unwrap()
            .state(),
        StripeOneTimePaymentState::Disputed
    );
    assert_eq!(
        parse_receipt("cs_test_once", &request, &session())
            .unwrap()
            .state(),
        StripeOneTimePaymentState::Unpaid
    );
}

#[tokio::test]
async fn one_time_mock_never_grants_paid_provenance() {
    let provider = StripeProvider::new("mock_key", "mock_hook");
    let request = request();
    let one = provider.create_one_time_checkout(&request).await.unwrap();
    let two = provider.create_one_time_checkout(&request).await.unwrap();
    assert_eq!(one.id(), two.id());
    assert_eq!(one.status(), StripeCheckoutStatus::Mock);
    let receipt = provider
        .read_one_time_receipt(one.id(), &request)
        .await
        .unwrap();
    assert_eq!(receipt.state(), StripeOneTimePaymentState::Mock);
    assert_eq!(receipt.livemode(), None);
}
