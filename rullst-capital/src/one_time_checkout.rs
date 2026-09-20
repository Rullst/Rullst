//! Explicit one-time hosted checkout, separate from recurring subscriptions.

use crate::{CapitalError, StripeCheckoutRequest};

/// Server-owned fixed one-time price and the amount/currency shown to the buyer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripeOneTimePrice {
    id: String,
    amount_minor: u64,
    currency: String,
}

impl StripeOneTimePrice {
    pub fn new(
        id: impl Into<String>,
        amount_minor: u64,
        currency: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let id = id.into();
        let currency = currency.into();
        if !crate::providers::stripe_contract::valid_reference(&id, "price_", 200)
            || !(1..=99_999_999).contains(&amount_minor)
            || currency.len() != 3
            || !currency.bytes().all(|b| b.is_ascii_lowercase())
        {
            return Err(CapitalError::ConfigurationError("one-time checkout requires a fixed Stripe price, positive minor-unit amount and lowercase currency".into()));
        }
        Ok(Self {
            id,
            amount_minor,
            currency,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn amount_minor(&self) -> u64 {
        self.amount_minor
    }
    pub fn currency(&self) -> &str {
        &self.currency
    }
}

/// Immutable authorized purchase attempt. Persist it before calling the provider.
/// No tax/discount/shipping/adaptive-pricing variation is enabled by this contract.
#[derive(Clone)]
pub struct StripePaymentCheckoutRequest {
    pub(crate) checkout: StripeCheckoutRequest,
    price: StripeOneTimePrice,
}

impl StripePaymentCheckoutRequest {
    pub fn new(
        customer_id: impl Into<String>,
        price: StripeOneTimePrice,
        owner_reference: impl Into<String>,
        idempotency_key: impl Into<String>,
        success_url: impl Into<String>,
        cancel_url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let checkout = StripeCheckoutRequest::new(
            customer_id,
            price.id(),
            owner_reference,
            idempotency_key,
            success_url,
            cancel_url,
        )?;
        Ok(Self { checkout, price })
    }
    pub fn price(&self) -> &StripeOneTimePrice {
        &self.price
    }
    pub fn customer_id(&self) -> &str {
        self.checkout.customer_id()
    }
    pub fn owner_reference(&self) -> &str {
        self.checkout.owner_reference()
    }
    pub fn idempotency_key(&self) -> &str {
        self.checkout.idempotency_key()
    }
    pub fn success_url(&self) -> &str {
        self.checkout.success_url()
    }
    pub fn cancel_url(&self) -> &str {
        self.checkout.cancel_url()
    }
    /// Domain-separated from recurring checkout and bound to displayed price data.
    pub fn request_digest(&self) -> [u8; 32] {
        let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
        hash.update(b"rullst.stripe.payment-checkout.v1\0");
        hash.update(&self.checkout.request_digest());
        hash.update(&self.price.amount_minor.to_be_bytes());
        hash.update(self.price.currency.as_bytes());
        let mut result = [0; 32];
        result.copy_from_slice(hash.finish().as_ref());
        result
    }
}

impl std::fmt::Debug for StripePaymentCheckoutRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripePaymentCheckoutRequest")
            .finish_non_exhaustive()
    }
}

/// Authoritative provider snapshot, not a permanent entitlement or receipt claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripeOneTimePaymentState {
    Unpaid,
    Paid,
    Refunded,
    Disputed,
    Expired,
    Mock,
}

/// A fresh session/payment/charge read bound to the authorized purchase attempt.
#[derive(Clone)]
pub struct StripeOneTimeReceipt {
    pub(crate) session_id: String,
    pub(crate) payment_intent_id: Option<String>,
    pub(crate) state: StripeOneTimePaymentState,
    pub(crate) livemode: Option<bool>,
    pub(crate) request_digest: [u8; 32],
}

impl StripeOneTimeReceipt {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn payment_intent_id(&self) -> Option<&str> {
        self.payment_intent_id.as_deref()
    }
    pub fn state(&self) -> StripeOneTimePaymentState {
        self.state
    }
    pub fn livemode(&self) -> Option<bool> {
        self.livemode
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
}

impl std::fmt::Debug for StripeOneTimeReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StripeOneTimeReceipt")
            .field("state", &self.state)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
