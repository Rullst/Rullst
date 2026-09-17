//! Explicit subscription checkout for an already persisted Stripe customer.

use crate::CapitalError;
use ring::digest::{Context, SHA256};
use std::fmt;

pub(crate) const STRIPE_CHECKOUT_API_VERSION: &str = "2025-03-31.basil";

/// Immutable input for one hosted Stripe subscription checkout attempt.
///
/// The caller must authorize and persist customer/tenant ownership and this
/// attempt before dispatch. IDs and keys must be server-owned; never accept
/// them directly from an untrusted checkout form. Retries reuse the same request.
#[derive(Clone, PartialEq, Eq)]
pub struct StripeCheckoutRequest {
    customer_id: String,
    price_id: String,
    owner_reference: String,
    idempotency_key: String,
    success_url: String,
    cancel_url: String,
}

impl StripeCheckoutRequest {
    /// Creates a bounded subscription request with explicit customer and retry identity.
    ///
    /// The reference is an opaque non-PII application key, not an email or token.
    /// HTTPS redirect URLs are application-owned and must contain no credentials,
    /// whitespace, control characters or fragments. No customer is created implicitly.
    pub fn new(
        customer_id: impl Into<String>,
        price_id: impl Into<String>,
        owner_reference: impl Into<String>,
        idempotency_key: impl Into<String>,
        success_url: impl Into<String>,
        cancel_url: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let request = Self {
            customer_id: customer_id.into(),
            price_id: price_id.into(),
            owner_reference: owner_reference.into(),
            idempotency_key: idempotency_key.into(),
            success_url: success_url.into(),
            cancel_url: cancel_url.into(),
        };
        if !valid_reference(&request.customer_id, "cus_", 200)
            || !valid_reference(&request.price_id, "price_", 200)
            || !valid_reference(&request.owner_reference, "", 200)
            || !valid_reference(&request.idempotency_key, "", 255)
        {
            return Err(CapitalError::ConfigurationError(
                "checkout requires bounded Stripe customer/price IDs and opaque ASCII owner/retry keys".into(),
            ));
        }
        for url in [&request.success_url, &request.cancel_url] {
            crate::providers::validate_checkout_url("checkout-redirect", url).map_err(|_| {
                CapitalError::ConfigurationError("checkout redirects require bounded credential-free HTTPS URLs without fragments".into())
            })?;
        }
        Ok(request)
    }

    /// Existing Stripe customer bound to an authorized application owner.
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }
    /// The server-selected recurring Stripe price, with quantity fixed to one.
    pub fn price_id(&self) -> &str {
        &self.price_id
    }
    /// Opaque local reference forwarded as client_reference_id and subscription metadata.
    pub fn owner_reference(&self) -> &str {
        &self.owner_reference
    }
    /// Application-persisted retry key forwarded to Stripe's Idempotency-Key header.
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
    /// Navigation after checkout; never a payment confirmation.
    pub fn success_url(&self) -> &str {
        &self.success_url
    }
    /// Application navigation when checkout is cancelled.
    pub fn cancel_url(&self) -> &str {
        &self.cancel_url
    }

    /// Versioned SHA-256 digest for binding a persisted attempt to immutable input.
    ///
    /// Scope the attempt by provider account and test/live mode as well as this
    /// digest. This value is not encryption, a signature or durable deduplication.
    pub fn request_digest(&self) -> [u8; 32] {
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.stripe.subscription-checkout.v1\0");
        hash.update(STRIPE_CHECKOUT_API_VERSION.as_bytes());
        hash.update(b"\0");
        for field in [
            self.customer_id(),
            self.price_id(),
            self.owner_reference(),
            self.idempotency_key(),
            self.success_url(),
            self.cancel_url(),
        ] {
            hash.update(&(field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        let digest = hash.finish();
        let mut result = [0; 32];
        result.copy_from_slice(digest.as_ref());
        result
    }
}

impl fmt::Debug for StripeCheckoutRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripeCheckoutRequest")
            .finish_non_exhaustive()
    }
}

pub(crate) fn valid_reference(value: &str, prefix: &str, limit: usize) -> bool {
    value.len() > prefix.len()
        && value.len() <= limit
        && value.starts_with(prefix)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

/// Provenance of a created checkout session; neither variant proves payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripeCheckoutStatus {
    /// Open session returned by Stripe with bound identities and line item.
    Created,
    /// Deterministic local fixture with no provider operation.
    Mock,
}

/// A provider-bound open subscription checkout, never a paid receipt.
#[derive(Clone)]
pub struct StripeCheckoutSession {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) status: StripeCheckoutStatus,
    pub(crate) livemode: Option<bool>,
    pub(crate) expires_at: Option<i64>,
    pub(crate) request_digest: [u8; 32],
}

impl StripeCheckoutSession {
    /// Session ID to persist with the attempt before redirecting.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Hosted URL, preserved exactly, including Stripe's documented fragment.
    pub fn url(&self) -> &str {
        &self.url
    }
    /// Whether Stripe created the session or this is an offline fixture.
    pub fn status(&self) -> StripeCheckoutStatus {
        self.status
    }
    /// Provider test/live mode; mocks have no provider mode.
    pub fn livemode(&self) -> Option<bool> {
        self.livemode
    }
    /// Provider session expiry; mocks have no provider expiration.
    pub fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }
    /// Input digest to compare with the durably persisted attempt.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
}

impl fmt::Debug for StripeCheckoutSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripeCheckoutSession")
            .field("status", &self.status)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
