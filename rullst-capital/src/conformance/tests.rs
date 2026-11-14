//! The kit against InfinitePay's live code path, with a local Checkout API
//! standing in for the source of truth (no network).

use super::*;
use crate::InfinitePayProvider;
use axum::{Json, Router, extract::State, routing::post};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Source(Arc<Mutex<Value>>);

async fn checkout_api() -> (String, Source) {
    let source = Source::default();
    *source.0.lock().unwrap() = json!({ "success": true, "paid": false });
    let app = Router::new()
        .route(
            "/links",
            post(|| async {
                Json(json!({ "url": "https://checkout.infinitepay.io/loja?lenc=conformance" }))
            }),
        )
        .route(
            "/payment_check",
            post(|State(source): State<Source>| async move {
                let reply = source.0.lock().unwrap().clone();
                Json(reply)
            }),
        )
        .with_state(source.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{address}"), source)
}

struct InfinitePayHarness {
    source: Source,
}

impl BillingHarness for InfinitePayHarness {
    fn one_off_request(&self) -> Option<OneOffCheckoutRequest> {
        OneOffCheckoutRequest::new(
            "order-2026-0001",
            15_000,
            "BRL",
            "Aulas de novembro",
            "https://app.example/pagamento/retorno",
            "https://app.example/pagamento",
        )
        .ok()
    }

    fn authentic_delivery(
        &self,
        request: Option<&OneOffCheckoutRequest>,
    ) -> Option<WebhookDelivery> {
        let reference = request.map_or("order-2026-0001", OneOffCheckoutRequest::reference);
        let body = format!(
            r#"{{"order_nsu":"{reference}","transaction_nsu":"nsu-1","invoice_slug":"slug-1","amount":15000,"paid_amount":15000,"capture_method":"pix"}}"#
        );
        Some(WebhookDelivery::new(body).with_header("Content-Type", "application/json"))
    }

    fn set_source_payment(&self, _: &OneOffCheckoutRequest, payment: &SourcePayment) -> bool {
        // payment_check reports no currency: InfinitePay charges only BRL.
        if payment.is_paid() && payment.currency() != "brl" {
            return false;
        }
        *self.source.0.lock().unwrap() = json!({
            "success": true,
            "paid": payment.is_paid(),
            "amount": payment.amount_minor(),
            "paid_amount": payment.amount_minor(),
        });
        true
    }
}

#[tokio::test]
async fn infinitepay_confirms_at_the_source_against_a_local_checkout_api() {
    let (base, source) = checkout_api().await;
    let provider = InfinitePayProvider::checkout("$loja")
        .unwrap()
        .with_webhook_url("https://app.example/webhooks/infinitepay")
        .unwrap()
        .with_api_base(base);
    let report = check_billing_provider(&provider, &InfinitePayHarness { source }).await;
    report.assert_passed();
    assert_eq!(report.provider(), "infinitepay");
    // Every path ran: the paid control, both wrong amounts and another order.
    assert_eq!(report.case("confirmation.required").unwrap().detail(), "");
    assert_eq!(
        report
            .case("confirmation.amount_currency_bound")
            .unwrap()
            .detail(),
        "a source payment of another currency was not simulated"
    );
    assert_eq!(
        report
            .cases()
            .iter()
            .filter(|case| case.outcome() == CaseOutcome::Passed)
            .count(),
        9
    );
}

#[tokio::test]
async fn a_source_that_reports_paid_is_not_trusted_for_another_amount() {
    let (base, source) = checkout_api().await;
    let provider = InfinitePayProvider::checkout("loja")
        .unwrap()
        .with_webhook_url("https://app.example/webhooks/infinitepay")
        .unwrap()
        .with_api_base(base);
    // A harness that always reports the paid amount 1 cent short.
    struct Short(InfinitePayHarness);
    impl BillingHarness for Short {
        fn one_off_request(&self) -> Option<OneOffCheckoutRequest> {
            self.0.one_off_request()
        }
        fn authentic_delivery(
            &self,
            request: Option<&OneOffCheckoutRequest>,
        ) -> Option<WebhookDelivery> {
            self.0.authentic_delivery(request)
        }
        fn set_source_payment(
            &self,
            request: &OneOffCheckoutRequest,
            payment: &SourcePayment,
        ) -> bool {
            let short =
                SourcePayment::paid(payment.amount_minor().saturating_sub(1), payment.currency());
            let payment = if payment.is_paid() { &short } else { payment };
            self.0.set_source_payment(request, payment)
        }
    }
    let report = check_billing_provider(&provider, &Short(InfinitePayHarness { source })).await;
    let required = report.case("confirmation.required").unwrap();
    assert_eq!(required.outcome(), CaseOutcome::Failed);
    assert!(
        required.detail().starts_with(
            "a payment the source confirms with the request's amount and currency is paid"
        ),
        "{report}"
    );
}
