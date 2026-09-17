//! Immutable signed Stripe event metadata for caller-owned transactional processing.

use crate::{CapitalError, WebhookEvent, WebhookVerificationMode};
use ring::digest::{Context, SHA256};

/// A bounded Stripe subscription event returned only by the provider verifier.
///
/// It is not a persisted inbox claim, an authorization grant or a payment
/// receipt. Bind account/test-live/customer/owner to application state and
/// commit deduplication with domain changes. Creation time alone cannot order
/// concurrent snapshots; reconciliation remains necessary.
#[derive(Clone)]
pub struct StripeSubscriptionEvent {
    pub(crate) event_id: String,
    pub(crate) event_type: String,
    pub(crate) api_version: String,
    pub(crate) created_at: i64,
    pub(crate) livemode: bool,
    pub(crate) connected_account: Option<String>,
    pub(crate) owner_reference: Option<String>,
    pub(crate) provider_status: String,
    pub(crate) subscription: WebhookEvent,
    pub(crate) verification_mode: WebhookVerificationMode,
    pub(crate) payload_digest: [u8; 32],
}

impl StripeSubscriptionEvent {
    /// Stable Stripe event identity; namespace it by configured account and mode.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    /// Supported subscription lifecycle event name.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }
    /// Provider API version used to render the signed snapshot.
    pub fn api_version(&self) -> &str {
        &self.api_version
    }
    /// Provider event creation time, not a complete causal ordering key.
    pub fn created_at(&self) -> i64 {
        self.created_at
    }
    /// Signed event mode, checked against the nested subscription's mode.
    pub fn livemode(&self) -> bool {
        self.livemode
    }
    /// Signed connected account, when present. Absence does not identify the platform account.
    pub fn connected_account(&self) -> Option<&str> {
        self.connected_account.as_deref()
    }
    /// Opaque subscription metadata reference; compare with a persisted authorized owner.
    pub fn owner_reference(&self) -> Option<&str> {
        self.owner_reference.as_deref()
    }
    /// Exact Stripe state, preserving incomplete/incomplete_expired distinctions.
    pub fn provider_status(&self) -> &str {
        &self.provider_status
    }
    /// Read-only legacy normalization, including optional contact data.
    pub fn subscription(&self) -> &WebhookEvent {
        &self.subscription
    }
    /// Real cryptographic verification or an explicitly configured local mock.
    pub fn verification_mode(&self) -> WebhookVerificationMode {
        self.verification_mode
    }

    /// Rejects mock verification before production event processing.
    ///
    /// This does not check application account/mode/owner bindings or persist anything.
    pub fn require_real(&self) -> Result<(), CapitalError> {
        if self.verification_mode != WebhookVerificationMode::Real {
            return Err(CapitalError::MockWebhookNotAllowed("stripe".into()));
        }
        Ok(())
    }

    /// SHA-256 of the exact signed bytes, never encryption or a semantic replay key.
    pub fn payload_digest(&self) -> [u8; 32] {
        self.payload_digest
    }

    /// Versioned digest of the event metadata and subscription mutation.
    ///
    /// Excludes contact email, signature timestamp and unrelated delivery/JSON
    /// fields. The same ID with a changed mutation must be reconciled, not
    /// silently applied as another event. Include the configured account and
    /// mode in the durable namespace; no platform account is inferred here.
    pub fn mutation_digest(&self) -> [u8; 32] {
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.stripe.subscription-event-mutation.v1\0");
        hash.update(&[
            u8::from(self.livemode),
            u8::from(self.verification_mode == WebhookVerificationMode::Real),
        ]);
        hash.update(&self.created_at.to_be_bytes());
        hash.update(&self.subscription.ends_at.unwrap_or(0).to_be_bytes());
        for field in [
            self.event_id(),
            self.event_type(),
            self.api_version(),
            self.connected_account().unwrap_or(""),
            self.owner_reference().unwrap_or(""),
            self.provider_status(),
            &self.subscription.subscription_id,
            &self.subscription.customer_id,
            &self.subscription.plan_id,
            self.subscription.status.as_str(),
        ] {
            hash.update(&(field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        let mut digest = [0; 32];
        digest.copy_from_slice(hash.finish().as_ref());
        digest
    }
}

impl std::fmt::Debug for StripeSubscriptionEvent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeSubscriptionEvent")
            .field("verification_mode", &self.verification_mode)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
