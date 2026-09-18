//! Explicit Paddle Billing customer and transaction contracts.
use crate::{CapitalError, WebhookEvent};

/// Immutable customer provisioning intent. Persist before sending; Paddle does
/// not guarantee deduplication of client-supplied retry keys. Email is contact
/// data, never permission to claim an existing provider customer.
#[derive(Clone)]
pub struct PaddleCustomerRequest {
    pub(crate) owner: String,
    pub(crate) attempt: String,
    pub(crate) email: String,
}
impl PaddleCustomerRequest {
    pub fn new(
        owner_reference: impl Into<String>,
        attempt_reference: impl Into<String>,
        email: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let owner = owner_reference.into();
        let attempt = attempt_reference.into();
        let validated = crate::StripeCustomerRequest::new(&owner, &attempt)?.with_email(email)?;
        Ok(Self {
            owner,
            attempt,
            email: validated.email().ok_or_else(invalid)?.into(),
        })
    }
    pub fn owner_reference(&self) -> &str {
        &self.owner
    }
    pub fn attempt_reference(&self) -> &str {
        &self.attempt
    }
    pub fn request_digest(&self) -> [u8; 32] {
        digest(
            b"rullst.paddle.customer.v1",
            &[&self.owner, &self.attempt, &self.email],
        )
    }
}

/// One existing customer, one catalog recurring price, and an application-owned
/// Paddle.js payment page. The page must be configured/approved in Paddle.
/// This URL is the checkout launcher, not an after-payment redirect.
#[derive(Clone)]
pub struct PaddleCheckoutRequest {
    pub(crate) customer: String,
    pub(crate) price: String,
    pub(crate) owner: String,
    pub(crate) attempt: String,
    pub(crate) payment_link: String,
}
impl PaddleCheckoutRequest {
    pub fn new(
        customer_id: impl Into<String>,
        price_id: impl Into<String>,
        owner_reference: impl Into<String>,
        attempt_reference: impl Into<String>,
        payment_link: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let mut request = Self {
            customer: customer_id.into(),
            price: price_id.into(),
            owner: owner_reference.into(),
            attempt: attempt_reference.into(),
            payment_link: payment_link.into(),
        };
        if !id(&request.customer, "ctm_")
            || !id(&request.price, "pri_")
            || !opaque(&request.owner)
            || !opaque(&request.attempt)
        {
            return Err(invalid());
        }
        crate::providers::validate_checkout_url("paddle", &request.payment_link)?;
        let url = reqwest::Url::parse(&request.payment_link).map_err(|_| invalid())?;
        if url.query().is_some() {
            return Err(invalid());
        }
        request.payment_link = url.into();
        Ok(request)
    }
    pub fn customer_id(&self) -> &str {
        &self.customer
    }
    pub fn price_id(&self) -> &str {
        &self.price
    }
    pub fn owner_reference(&self) -> &str {
        &self.owner
    }
    /// Correlation metadata only; not a provider idempotency key.
    pub fn attempt_reference(&self) -> &str {
        &self.attempt
    }
    pub fn payment_link(&self) -> &str {
        &self.payment_link
    }
    /// Scope persistent attempts by provider account and sandbox/live mode too.
    pub fn request_digest(&self) -> [u8; 32] {
        digest(
            b"rullst.paddle.checkout.v1",
            &[
                &self.customer,
                &self.price,
                &self.owner,
                &self.attempt,
                &self.payment_link,
            ],
        )
    }
}

/// Bound customer ID to persist before creating a transaction.
#[derive(Clone)]
pub struct PaddleCustomerReceipt {
    pub(crate) id: String,
    pub(crate) digest: [u8; 32],
    pub(crate) sandbox: Option<bool>,
}
impl PaddleCustomerReceipt {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.digest
    }
    /// None identifies deterministic offline evidence.
    pub fn sandbox(&self) -> Option<bool> {
        self.sandbox
    }
    pub fn is_mock(&self) -> bool {
        self.sandbox.is_none()
    }
}

/// Current state of the exact checkout transaction. Neither an open checkout
/// nor a successful navigation proves payment or subscription entitlement.
#[derive(Clone)]
pub struct PaddleCheckoutSession {
    pub(crate) id: String,
    pub(crate) url: Option<String>,
    pub(crate) status: String,
    pub(crate) subscription: Option<String>,
    pub(crate) digest: [u8; 32],
    pub(crate) sandbox: Option<bool>,
}
impl PaddleCheckoutSession {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn subscription_id(&self) -> Option<&str> {
        self.subscription.as_deref()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn sandbox(&self) -> Option<bool> {
        self.sandbox
    }
    pub fn is_mock(&self) -> bool {
        self.sandbox.is_none()
    }
}

/// Signed, owner/attempt-bound subscription event. Commit the event receipt and
/// domain mutation in one account/environment-scoped transaction. Reconcile
/// current provider state before applying delayed or reordered deliveries.
#[derive(Clone)]
pub struct PaddleSubscriptionEvent {
    pub(crate) owner: String,
    pub(crate) attempt: String,
    pub(crate) event_id: String,
    pub(crate) event_type: String,
    pub(crate) occurred_at: i64,
    pub(crate) transaction: Option<String>,
    pub(crate) status: String,
    pub(crate) subscription: WebhookEvent,
    pub(crate) mock: bool,
}
impl PaddleSubscriptionEvent {
    pub fn owner_reference(&self) -> &str {
        &self.owner
    }
    pub fn attempt_reference(&self) -> &str {
        &self.attempt
    }
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
    pub fn occurred_at(&self) -> i64 {
        self.occurred_at
    }
    pub fn transaction_id(&self) -> Option<&str> {
        self.transaction.as_deref()
    }
    pub fn provider_status(&self) -> &str {
        &self.status
    }
    pub fn subscription(&self) -> &WebhookEvent {
        &self.subscription
    }
    pub fn is_mock(&self) -> bool {
        self.mock
    }
    pub fn require_real(&self) -> Result<(), CapitalError> {
        if self.mock { Err(invalid()) } else { Ok(()) }
    }
    /// Stable exposed mutation fields; excludes mutable contact data and delivery
    /// signatures. This digest is neither encryption nor an event claim.
    pub fn mutation_digest(&self) -> [u8; 32] {
        digest(
            b"rullst.paddle.subscription-event.v1",
            &[
                &self.owner,
                &self.attempt,
                &self.event_id,
                &self.event_type,
                &self.occurred_at.to_string(),
                self.transaction_id().unwrap_or(""),
                &self.status,
                &self.subscription.subscription_id,
                &self.subscription.customer_id,
                &self.subscription.plan_id,
                &self
                    .subscription
                    .ends_at
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ],
        )
    }
}

/// Read-only current subscription state bound to the persisted checkout intent.
#[derive(Clone)]
pub struct PaddleSubscriptionSnapshot {
    pub(crate) subscription: WebhookEvent,
    pub(crate) status: String,
    pub(crate) sandbox: Option<bool>,
}
impl PaddleSubscriptionSnapshot {
    pub fn subscription(&self) -> &WebhookEvent {
        &self.subscription
    }
    pub fn provider_status(&self) -> &str {
        &self.status
    }
    pub fn sandbox(&self) -> Option<bool> {
        self.sandbox
    }
    pub fn require_real(&self) -> Result<(), CapitalError> {
        if self.sandbox.is_none() {
            Err(invalid())
        } else {
            Ok(())
        }
    }
}

macro_rules! redacted_debug {
    ($($name:ident),+ $(,)?) => { $(impl std::fmt::Debug for $name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct(stringify!($name)).finish_non_exhaustive() }
    })+ };
}
redacted_debug!(
    PaddleCustomerRequest,
    PaddleCheckoutRequest,
    PaddleCustomerReceipt,
    PaddleCheckoutSession,
    PaddleSubscriptionEvent,
    PaddleSubscriptionSnapshot
);

pub(crate) fn id(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|suffix| {
        suffix.len() == 26
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}
pub(crate) fn opaque(value: &str) -> bool {
    crate::providers::stripe_contract::valid_reference(value, "", 200)
}
pub(crate) fn invalid() -> CapitalError {
    CapitalError::ConfigurationError("Paddle requires bound provider IDs, opaque references and an approved HTTPS payment page without query or fragment".into())
}
pub(crate) fn digest(domain: &[u8], fields: &[&str]) -> [u8; 32] {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(domain);
    for field in fields {
        hash.update(&(field.len() as u64).to_be_bytes());
        hash.update(field.as_bytes());
    }
    let mut result = [0; 32];
    result.copy_from_slice(hash.finish().as_ref());
    result
}
