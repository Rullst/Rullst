//! Fixtures shared by the billing conformance tests.
#![allow(dead_code)]

use hmac::{KeyInit, Mac};
use rullst_capital::OneOffCheckoutRequest;
use rullst_capital::conformance::{
    BillingHarness, CaseOutcome, ConformanceReport, SourcePayment, WebhookDelivery,
};
use std::sync::{Arc, Mutex};

/// What the in-memory source of truth reports.
pub type Ledger = Arc<Mutex<SourcePayment>>;

pub const SUBSCRIPTION_EVENT: &str = r#"{"type":"customer.subscription.updated","data":{"object":{"object":"subscription","id":"sub_conformance","customer":"cus_conformance","status":"active","items":{"has_more":false,"data":[{"price":{"id":"price_pro"}}]}}}}"#;

pub fn usd_request() -> Option<OneOffCheckoutRequest> {
    OneOffCheckoutRequest::new(
        "order-1",
        2_500,
        "usd",
        "Workshop seat",
        "https://app.example/paid",
        "https://app.example/cart",
    )
    .ok()
}

pub fn brl_request() -> Option<OneOffCheckoutRequest> {
    OneOffCheckoutRequest::new(
        "pedido-1",
        15_000,
        "BRL",
        "Aulas de novembro",
        "https://app.example/pagamento/retorno",
        "https://app.example/pagamento",
    )
    .ok()
}

/// Stripe's `t=<unix>,v1=<HMAC-SHA256("<t>.<body>")>` signature.
pub fn stripe_signed(payload: &str, secret: &str) -> WebhookDelivery {
    let now = chrono::Utc::now().timestamp();
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{now}.{payload}").as_bytes());
    let tag = mac.finalize().into_bytes();
    WebhookDelivery::new(payload).with_header(
        "Stripe-Signature",
        format!("t={now},v1={}", hex::encode(tag)),
    )
}

/// Builds the authentic delivery for a request.
pub type Delivery = Box<dyn Fn(Option<&OneOffCheckoutRequest>) -> WebhookDelivery + Send + Sync>;

/// A harness with fixed fixtures and an optional in-memory source of truth.
pub struct Harness {
    pub request: Option<OneOffCheckoutRequest>,
    pub delivery: Option<Delivery>,
    pub ledger: Option<Ledger>,
}

impl BillingHarness for Harness {
    fn one_off_request(&self) -> Option<OneOffCheckoutRequest> {
        self.request.clone()
    }
    fn authentic_delivery(
        &self,
        request: Option<&OneOffCheckoutRequest>,
    ) -> Option<WebhookDelivery> {
        self.delivery.as_ref().map(|delivery| delivery(request))
    }
    fn set_source_payment(&self, _: &OneOffCheckoutRequest, payment: &SourcePayment) -> bool {
        self.ledger
            .as_ref()
            .map(|ledger| *ledger.lock().unwrap() = payment.clone())
            .is_some()
    }
}

pub fn stripe_harness(secret: &'static str) -> Harness {
    Harness {
        request: usd_request(),
        delivery: Some(Box::new(move |_: Option<&OneOffCheckoutRequest>| {
            if secret.starts_with("mock_") {
                WebhookDelivery::new(SUBSCRIPTION_EVENT).with_header("stripe-signature", secret)
            } else {
                stripe_signed(SUBSCRIPTION_EVENT, secret)
            }
        })),
        ledger: None,
    }
}

pub fn infinitepay_harness() -> Harness {
    Harness {
        request: brl_request(),
        delivery: Some(Box::new(|request: Option<&OneOffCheckoutRequest>| {
            let reference = request.map_or("pedido-1", OneOffCheckoutRequest::reference);
            WebhookDelivery::new(format!(
                r#"{{"order_nsu":"{reference}","transaction_nsu":"nsu-1","invoice_slug":"slug-1","amount":15000}}"#
            ))
        })),
        ledger: None,
    }
}

pub fn outcome(report: &ConformanceReport, id: &str) -> CaseOutcome {
    report.case(id).map(|case| case.outcome()).unwrap()
}
