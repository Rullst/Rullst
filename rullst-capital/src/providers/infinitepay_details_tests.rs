//! The payment method, the instalments and the per-link notification URL.

use super::tests::{api, live, notification, request};
use super::*;
use crate::{OneOffPaymentState, PaymentMethod};
use serde_json::json;

const LINK_HOOK: &str = "https://app.example/webhooks/infinitepay/42?token=AbC-_9";

fn confirmed(extra: Value) -> OneOffPayment {
    let mut reply = json!({"success": true, "paid": true, "amount": 15_000, "paid_amount": 15_600});
    for (field, value) in extra.as_object().unwrap() {
        reply[field] = value.clone();
    }
    let check: api::PaymentCheck = serde_json::from_value(reply).unwrap();
    api::confirmation(&request(), "nsu-1", &check).unwrap()
}

#[test]
fn a_paid_check_reports_the_capture_method_and_installments() {
    let card = confirmed(json!({"capture_method": "credit_card", "installments": 3}));
    assert_eq!(card.payment_method(), Some(&PaymentMethod::CreditCard));
    assert_eq!(card.installments(), Some(3));
    let pix = confirmed(json!({"capture_method": "pix", "installments": 1}));
    assert_eq!(pix.payment_method(), Some(&PaymentMethod::Pix));
    assert_eq!(pix.installments(), Some(1));
    let other = confirmed(json!({"capture_method": "Debit_Card"}));
    assert!(matches!(
        other.payment_method(),
        Some(PaymentMethod::Other(_))
    ));
    assert_eq!(
        other.payment_method().map(PaymentMethod::as_str),
        Some("debit_card")
    );
    assert_eq!(other.installments(), None);
    assert_eq!(
        confirmed(json!({"installments": 255})).installments(),
        Some(255)
    );
    // A value the adapter cannot read is left out; the payment still counts.
    for unreadable in [
        json!({}),
        json!({"capture_method": null, "installments": null}),
        json!({"capture_method": 7, "installments": "2"}),
        json!({"capture_method": "", "installments": 0}),
        json!({"capture_method": "pix card", "installments": 256}),
        json!({"capture_method": "x".repeat(65), "installments": -1}),
    ] {
        let payment = confirmed(unreadable.clone());
        assert!(payment.is_paid(), "{unreadable}");
        assert_eq!(
            (payment.payment_method(), payment.installments()),
            (None, None),
            "{unreadable}"
        );
    }
    let check: api::PaymentCheck = serde_json::from_value(json!({
        "success": true, "paid": false, "capture_method": "pix", "installments": 1
    }))
    .unwrap();
    let unpaid = api::confirmation(&request(), "nsu-1", &check).unwrap();
    assert_eq!(unpaid.state(), OneOffPaymentState::Unpaid);
    assert_eq!(
        (unpaid.payment_method(), unpaid.installments()),
        (None, None)
    );
}

#[tokio::test]
async fn a_request_notification_url_replaces_the_providers_for_its_link() {
    let (base, fixture) = api().await;
    let notified = request().with_notification_url(LINK_HOOK).unwrap();
    let without_default = InfinitePayProvider::checkout("loja")
        .unwrap()
        .with_api_base(base.as_str());
    for (provider, request, sent) in [
        (live(&base), notified.clone(), Some(LINK_HOOK)),
        (
            live(&base),
            request(),
            Some("https://app.example/webhooks/infinitepay"),
        ),
        (without_default, notified.clone(), Some(LINK_HOOK)),
    ] {
        let checkout = provider.create_one_off_checkout(&request).await.unwrap();
        assert_eq!(checkout.notification_url(), request.notification_url());
        assert_eq!(checkout.request_digest(), request.request_digest());
        let link = fixture.seen.lock().unwrap().pop().unwrap().2;
        assert_eq!(link["webhook_url"].as_str(), sent);
    }
    let bare = InfinitePayProvider::checkout("loja")
        .unwrap()
        .with_api_base(base.as_str());
    bare.create_one_off_checkout(&request()).await.unwrap();
    let link = fixture.seen.lock().unwrap().pop().unwrap().2;
    assert!(link.get("webhook_url").is_none());
    // The URL routes the notification; confirmation is unchanged.
    let payment = live(&base)
        .confirm_one_off_payment(&notified, &notification())
        .await
        .unwrap();
    assert!(payment.is_paid());
    assert_eq!(payment.request_digest(), notified.request_digest());
}

#[tokio::test]
async fn mock_handles_report_the_notification_url_they_would_send() {
    let notified = request().with_notification_url(LINK_HOOK).unwrap();
    for provider in [
        InfinitePayProvider::checkout("mock_loja").unwrap(),
        InfinitePayProvider::new("mock_key", "mock_webhook"),
    ] {
        let checkout = provider.create_one_off_checkout(&notified).await.unwrap();
        assert!(checkout.is_mock());
        assert_eq!(checkout.notification_url(), Some(LINK_HOOK));
        let plain = provider.create_one_off_checkout(&request()).await.unwrap();
        assert_eq!(plain.notification_url(), None);
    }
}
