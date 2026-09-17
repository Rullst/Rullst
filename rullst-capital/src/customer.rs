//! Explicit Stripe customer provisioning before subscription checkout.

use crate::CapitalError;
use crate::providers::stripe_contract::{API_VERSION, valid_reference};
use ring::digest::{Context, SHA256};

/// Immutable provisioning input; persist it under an authorized account/owner before HTTP.
#[derive(Clone, PartialEq, Eq)]
pub struct StripeCustomerRequest {
    owner_reference: String,
    idempotency_key: String,
    email: Option<String>,
}

impl StripeCustomerRequest {
    /// Uses an opaque non-PII owner reference and a server-owned persisted retry key.
    pub fn new(
        owner_reference: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let request = Self {
            owner_reference: owner_reference.into(),
            idempotency_key: idempotency_key.into(),
            email: None,
        };
        if !valid_reference(&request.owner_reference, "", 200)
            || !valid_reference(&request.idempotency_key, "", 255)
        {
            return Err(CapitalError::ConfigurationError(
                "customer provisioning requires bounded opaque ASCII owner/retry keys".into(),
            ));
        }
        Ok(request)
    }

    /// Adds optional contact data; never used to look up or establish customer ownership.
    ///
    /// Performs bounded syntax checks, not mailbox verification. Changing this
    /// value changes the immutable request digest and must not reuse an old attempt.
    pub fn with_email(mut self, email: impl Into<String>) -> Result<Self, CapitalError> {
        let email = email.into();
        if email.len() > 254
            || email
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
            || !email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty() && !domain.is_empty() && !domain.contains('@')
            })
        {
            return Err(CapitalError::ConfigurationError(
                "customer email must be a bounded contact address without whitespace or controls"
                    .into(),
            ));
        }
        self.email = Some(email);
        Ok(self)
    }

    /// Opaque application-owned reference sent in customer metadata.
    pub fn owner_reference(&self) -> &str {
        &self.owner_reference
    }
    /// Persisted key sent in Stripe's Idempotency-Key header.
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
    /// Optional contact data, not an identity proof.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Versioned request digest; scope persisted attempts by provider account and mode as well.
    ///
    /// Hashing is not encryption of optional contact data or durable deduplication.
    pub fn request_digest(&self) -> [u8; 32] {
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.stripe.customer-create.v1\0");
        hash.update(API_VERSION.as_bytes());
        hash.update(&[0, u8::from(self.email.is_some())]);
        for field in [
            self.owner_reference(),
            self.idempotency_key(),
            self.email().unwrap_or(""),
        ] {
            hash.update(&(field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        let mut digest = [0; 32];
        digest.copy_from_slice(hash.finish().as_ref());
        digest
    }
}

impl std::fmt::Debug for StripeCustomerRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeCustomerRequest")
            .finish_non_exhaustive()
    }
}

/// Origin of a provisioning result; customer creation never proves payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripeCustomerStatus {
    /// Stripe returned a customer bound to the requested metadata.
    Created,
    /// Deterministic offline fixture; no provider customer was created.
    Mock,
}

/// Provider-customer result to persist before dispatching any checkout.
#[derive(Clone)]
pub struct StripeCustomerReceipt {
    pub(crate) id: String,
    pub(crate) status: StripeCustomerStatus,
    pub(crate) livemode: Option<bool>,
    pub(crate) created_at: Option<i64>,
    pub(crate) request_digest: [u8; 32],
}

impl StripeCustomerReceipt {
    /// Stripe customer identity, not an email or local owner ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Provider creation or an explicit local fixture.
    pub fn status(&self) -> StripeCustomerStatus {
        self.status
    }
    /// Provider test/live mode; mocks have no provider mode.
    pub fn livemode(&self) -> Option<bool> {
        self.livemode
    }
    /// Provider object creation time; mocks have no provider timestamp.
    pub fn created_at(&self) -> Option<i64> {
        self.created_at
    }
    /// Binding to the immutable provisioning request.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
}

impl std::fmt::Debug for StripeCustomerReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeCustomerReceipt")
            .field("status", &self.status)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
