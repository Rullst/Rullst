//! The billing conformance kit against the built-in adapters (offline modes).
#![cfg(feature = "conformance")]

mod conformance_support;

use conformance_support::*;
use rullst_capital::conformance::{
    BillingHarness, CaseOutcome, SourcePayment, WebhookDelivery, check_billing_provider,
};
use rullst_capital::{InfinitePayProvider, OneOffCheckoutRequest, StripeProvider};

#[tokio::test]
async fn stripe_passes_in_mock_and_signed_webhook_modes() {
    for secret in ["mock_whsec_conformance", "whsec_conformance"] {
        let provider = StripeProvider::new("mock_sk_conformance", secret);
        let report = check_billing_provider(&provider, &stripe_harness(secret)).await;
        report.assert_passed();
        assert_eq!(report.provider(), "stripe");
        assert_eq!(report.failures().count(), 0);
        // A mock key cannot report paid, so the paid path is noted, not passed silently.
        assert!(
            report
                .case("confirmation.required")
                .unwrap()
                .detail()
                .contains("not exercised")
        );
    }
}

#[tokio::test]
async fn infinitepay_passes_its_offline_fixtures() {
    for provider in [
        InfinitePayProvider::checkout("").unwrap(),
        InfinitePayProvider::checkout("mock_loja").unwrap(),
    ] {
        let report = check_billing_provider(&provider, &infinitepay_harness()).await;
        report.assert_passed();
        assert_eq!(report.provider(), "infinitepay");
    }
    // The 13.0.0-alpha.4 constructor's mock-signed fixture.
    let legacy = InfinitePayProvider::new("mock_key", "mock_secret");
    let harness = Harness {
        request: brl_request(),
        delivery: Some(Box::new(|_: Option<&OneOffCheckoutRequest>| {
            WebhookDelivery::new(r#"{"id":"txn_1","status":"approved","plan_id":"plan_basic"}"#)
                .with_header("x-signature", "mock_secret")
        })),
        ledger: None,
    };
    check_billing_provider(&legacy, &harness)
        .await
        .assert_passed();
}

/// Supplies only the request: the trait's defaults simulate nothing.
struct RequestOnly;

impl BillingHarness for RequestOnly {
    fn one_off_request(&self) -> Option<OneOffCheckoutRequest> {
        usd_request()
    }
}

#[tokio::test]
async fn the_default_harness_methods_simulate_nothing() {
    let provider = StripeProvider::new("mock_sk_conformance", "mock_whsec_conformance");
    let report = check_billing_provider(&provider, &RequestOnly).await;
    report.assert_passed();
    assert_eq!(
        outcome(&report, "notification.replay_idempotent"),
        CaseOutcome::Skipped
    );
    assert_eq!(
        report
            .case("confirmation.amount_currency_bound")
            .unwrap()
            .detail(),
        "a source payment of a lower amount was not simulated; a source payment of a higher amount was not simulated; a source payment of another currency was not simulated"
    );
}

#[test]
fn deliveries_keep_the_body_and_lower_case_header_names() {
    let delivery = WebhookDelivery::new("{}").with_header("X-Signature", "abc");
    assert_eq!(delivery.payload(), b"{}");
    assert_eq!(
        delivery.headers().get("x-signature").map(String::as_str),
        Some("abc")
    );
    assert_eq!(delivery.headers().len(), 1);
    let payment = SourcePayment::paid(1_500, "usd");
    assert_eq!(
        (
            payment.is_paid(),
            payment.amount_minor(),
            payment.currency()
        ),
        (true, 1_500, "usd")
    );
    let unpaid = SourcePayment::unpaid();
    assert_eq!(
        (unpaid.is_paid(), unpaid.amount_minor(), unpaid.currency()),
        (false, 0, "")
    );
}
