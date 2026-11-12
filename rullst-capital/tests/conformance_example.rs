//! An application-owned payment provider that passes the billing conformance
//! kit, without a network. `docs/src/provider-conformance.md` walks through it.
#![cfg(feature = "conformance")]

use async_trait::async_trait;
use ring::hmac;
use rullst_capital::conformance::{
    BillingHarness, SourcePayment, WebhookDelivery, check_billing_provider,
};
use rullst_capital::{
    BillingProvider, CapitalError, OneOffCheckout, OneOffCheckoutRequest, OneOffPayment,
    PaymentNotification, SubscriptionStatus, WebhookEvent, WebhookVerificationMode,
};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

// ANCHOR: provider
/// ExamplePay's HTTP API as the adapter needs it. Production code implements
/// it with an HTTP client; tests use a local fake.
pub trait ExamplePayApi: Send + Sync {
    /// Creates a payment link for the server-side request; returns its URL.
    fn create_link(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> impl Future<Output = Result<String, CapitalError>> + Send;

    /// The charged amount and currency of the order's payment, `None` until
    /// the buyer pays.
    fn payment(
        &self,
        reference: &str,
    ) -> impl Future<Output = Result<Option<(u64, String)>, CapitalError>> + Send;
}

/// Adapter for the fictitious ExamplePay gateway. Webhooks carry
/// `examplepay-signature: <hex HMAC-SHA256 of the body>`.
pub struct ExamplePay<A> {
    webhook_secret: String,
    api: A,
}

impl<A: ExamplePayApi> ExamplePay<A> {
    pub fn new(webhook_secret: impl Into<String>, api: A) -> Self {
        Self {
            webhook_secret: webhook_secret.into(),
            api,
        }
    }
}

fn unsupported(what: &str) -> CapitalError {
    CapitalError::UnsupportedOperation(format!("ExamplePay has no {what}"))
}

#[async_trait]
impl<A: ExamplePayApi> BillingProvider for ExamplePay<A> {
    fn name(&self) -> &'static str {
        "examplepay"
    }

    fn webhook_verification_mode(&self) -> Result<WebhookVerificationMode, CapitalError> {
        if self.webhook_secret.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "ExamplePay webhook secret is empty".into(),
            ));
        }
        Ok(WebhookVerificationMode::Real)
    }

    async fn create_checkout_session(
        &self,
        customer_email: &str,
        plan_id: &str,
        _redirect_url: &str,
    ) -> Result<String, CapitalError> {
        if customer_email.trim().is_empty() || plan_id.trim().is_empty() {
            return Err(CapitalError::ConfigurationError(
                "an e-mail address and a plan are required".into(),
            ));
        }
        Err(unsupported("plans; use create_one_off_checkout"))
    }

    async fn create_one_off_checkout(
        &self,
        request: &OneOffCheckoutRequest,
    ) -> Result<OneOffCheckout, CapitalError> {
        let url = self.api.create_link(request).await?;
        // Binds the returned URL to the server-side request and checks it.
        OneOffCheckout::from_verified_provider_response(self.name(), request, url)?
            .with_session_id(request.reference())
    }

    async fn confirm_one_off_payment(
        &self,
        request: &OneOffCheckoutRequest,
        notification: &PaymentNotification,
    ) -> Result<OneOffPayment, CapitalError> {
        if notification.reference() != request.reference() {
            return Err(CapitalError::InvalidCharge(
                "the notification names another order".into(),
            ));
        }
        match self.api.payment(request.reference()).await? {
            // `paid` refuses an amount or currency other than the request's.
            Some((amount, currency)) => {
                OneOffPayment::paid(self.name(), request, amount, amount, &currency)
            }
            None => Ok(OneOffPayment::unpaid(self.name(), request)),
        }
    }

    fn handle_webhook(
        &self,
        payload: &[u8],
        headers: &HashMap<String, String>,
    ) -> Result<WebhookEvent, CapitalError> {
        self.webhook_verification_mode()?;
        let invalid = || CapitalError::InvalidSignature("ExamplePay signature".into());
        let signature = headers.get("examplepay-signature").ok_or_else(invalid)?;
        let tag = hex::decode(signature).map_err(|_| invalid())?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.webhook_secret.as_bytes());
        // Constant-time comparison of the whole body's MAC.
        hmac::verify(&key, payload, &tag).map_err(|_| invalid())?;
        let body: Value = serde_json::from_slice(payload)
            .map_err(|_| CapitalError::PayloadParseError("ExamplePay event".into()))?;
        let field = |name: &str| body[name].as_str().unwrap_or_default().to_owned();
        Ok(WebhookEvent {
            subscription_id: field("subscription_id"),
            customer_id: field("customer_id"),
            customer_email: field("customer_email"),
            plan_id: field("plan_id"),
            status: SubscriptionStatus::parse_status(&field("status")),
            ends_at: None,
        })
    }

    async fn create_customer_portal(&self, _: &str, _: &str) -> Result<String, CapitalError> {
        Err(unsupported("customer portal"))
    }

    async fn cancel_subscription(&self, _: &str) -> Result<(), CapitalError> {
        Err(unsupported("subscriptions"))
    }

    async fn pause_subscription(&self, _: &str) -> Result<(), CapitalError> {
        Err(unsupported("subscriptions"))
    }

    async fn report_usage(&self, _: &str, _: &str, _: u64) -> Result<(), CapitalError> {
        Err(unsupported("metered usage"))
    }

    async fn apply_coupon(&self, _: &str, _: &str) -> Result<(), CapitalError> {
        Err(unsupported("coupons"))
    }

    async fn extend_trial(&self, _: &str, _: i64) -> Result<(), CapitalError> {
        Err(unsupported("trials"))
    }
}
// ANCHOR_END: provider

// ANCHOR: harness
/// The local stand-in for ExamplePay's API: deterministic links and a
/// payment the test controls.
#[derive(Clone)]
struct LocalApi {
    payment: Arc<Mutex<SourcePayment>>,
}

impl ExamplePayApi for LocalApi {
    async fn create_link(&self, request: &OneOffCheckoutRequest) -> Result<String, CapitalError> {
        Ok(format!(
            "https://pay.example.invalid/checkout/{}",
            request.reference()
        ))
    }

    async fn payment(&self, _: &str) -> Result<Option<(u64, String)>, CapitalError> {
        let payment = self.payment.lock().unwrap();
        Ok(payment
            .is_paid()
            .then(|| (payment.amount_minor(), payment.currency().to_owned())))
    }
}

const SECRET: &str = "examplepay_webhook_secret";

struct Harness {
    api: LocalApi,
}

impl BillingHarness for Harness {
    fn one_off_request(&self) -> Option<OneOffCheckoutRequest> {
        OneOffCheckoutRequest::new(
            "order-42",
            4_900,
            "usd",
            "Rust workshop seat",
            "https://shop.example/paid",
            "https://shop.example/cart",
        )
        .ok()
    }

    fn authentic_delivery(&self, _: Option<&OneOffCheckoutRequest>) -> Option<WebhookDelivery> {
        let body = r#"{"subscription_id":"sub_1","customer_id":"cus_1","plan_id":"plan_1","status":"active"}"#;
        let key = hmac::Key::new(hmac::HMAC_SHA256, SECRET.as_bytes());
        let signature = hex::encode(hmac::sign(&key, body.as_bytes()));
        Some(WebhookDelivery::new(body).with_header("examplepay-signature", signature))
    }

    fn set_source_payment(&self, _: &OneOffCheckoutRequest, payment: &SourcePayment) -> bool {
        *self.api.payment.lock().unwrap() = payment.clone();
        true
    }
}

#[tokio::test]
async fn examplepay_passes_the_billing_conformance_kit() {
    let api = LocalApi {
        payment: Arc::new(Mutex::new(SourcePayment::unpaid())),
    };
    let provider = ExamplePay::new(SECRET, api.clone());
    let report = check_billing_provider(&provider, &Harness { api }).await;
    println!("{report}");
    report.assert_passed();
}
// ANCHOR_END: harness
