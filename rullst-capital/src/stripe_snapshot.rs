//! Explicit, ownership-bound subscription reads for caller-owned reconciliation.

use crate::{CapitalError, WebhookEvent, providers::stripe_contract::valid_reference};

/// Expected persisted identity for one single-price Stripe subscription.
#[derive(Clone)]
pub struct StripeSubscriptionLookup {
    subscription_id: String,
    customer_id: String,
    owner_reference: String,
    price_id: String,
    livemode: bool,
}

impl StripeSubscriptionLookup {
    /// Bind these expectations to application state before reading Stripe.
    pub fn new(
        subscription_id: impl Into<String>,
        customer_id: impl Into<String>,
        owner_reference: impl Into<String>,
        price_id: impl Into<String>,
        livemode: bool,
    ) -> Result<Self, CapitalError> {
        let request = Self {
            subscription_id: subscription_id.into(),
            customer_id: customer_id.into(),
            owner_reference: owner_reference.into(),
            price_id: price_id.into(),
            livemode,
        };
        if !valid_reference(&request.subscription_id, "sub_", 200)
            || !valid_reference(&request.customer_id, "cus_", 200)
            || !valid_reference(&request.owner_reference, "", 200)
            || !valid_reference(&request.price_id, "", 200)
        {
            return Err(CapitalError::ConfigurationError(
                "subscription lookup requires bounded subscription/customer/owner/price identities"
                    .into(),
            ));
        }
        Ok(request)
    }

    pub fn subscription_id(&self) -> &str {
        &self.subscription_id
    }
    pub fn customer_id(&self) -> &str {
        &self.customer_id
    }
    pub fn owner_reference(&self) -> &str {
        &self.owner_reference
    }
    pub fn price_id(&self) -> &str {
        &self.price_id
    }
    /// Expected mode, not evidence that a provider object exists.
    pub fn livemode(&self) -> bool {
        self.livemode
    }
}

impl std::fmt::Debug for StripeSubscriptionLookup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeSubscriptionLookup")
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}

/// Origin of a subscription snapshot, not proof of payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripeSubscriptionSource {
    Retrieved,
    Mock,
}

/// Immutable bounded provider state; host serialization/reconciliation is still required.
#[derive(Clone)]
pub struct StripeSubscriptionSnapshot {
    pub(crate) subscription: WebhookEvent,
    pub(crate) provider_status: String,
    pub(crate) owner_reference: String,
    pub(crate) source: StripeSubscriptionSource,
    pub(crate) livemode: Option<bool>,
}

impl StripeSubscriptionSnapshot {
    pub fn subscription(&self) -> &WebhookEvent {
        &self.subscription
    }
    pub fn provider_status(&self) -> &str {
        &self.provider_status
    }
    pub fn owner_reference(&self) -> &str {
        &self.owner_reference
    }
    pub fn source(&self) -> StripeSubscriptionSource {
        self.source
    }
    pub fn livemode(&self) -> Option<bool> {
        self.livemode
    }

    /// Refuses a mock before production reconciliation.
    pub fn require_real(&self) -> Result<(), CapitalError> {
        if self.source != StripeSubscriptionSource::Retrieved {
            return Err(CapitalError::ConfigurationError(
                "mock subscription state cannot establish production billing state".into(),
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for StripeSubscriptionSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StripeSubscriptionSnapshot")
            .field("source", &self.source)
            .field("livemode", &self.livemode)
            .finish_non_exhaustive()
    }
}
