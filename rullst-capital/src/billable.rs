use crate::capital::{BillingProvider, provider};
use crate::error::CapitalError;
use crate::{ChargeReceipt, ChargeRequest, TrialExtension};
use async_trait::async_trait;

const MAX_GRACE_PERIOD_SECONDS: i64 = 366 * 24 * 60 * 60;

/// Bounded application access window after a billing transition.
///
/// This value does not schedule a provider cancellation, persist itself, or grant
/// access automatically. Applications store it with authoritative subscription
/// state and evaluate it in their own entitlement boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GracePeriod {
    starts_at: i64,
    ends_at: i64,
}

impl GracePeriod {
    /// Creates a positive, at-most-366-day half-open Unix timestamp window.
    pub fn new(starts_at: i64, ends_at: i64) -> Result<Self, CapitalError> {
        let duration = ends_at.checked_sub(starts_at).ok_or_else(|| {
            CapitalError::SubscriptionError("grace-period timestamps overflowed".to_string())
        })?;
        if starts_at < 0 || duration <= 0 || duration > MAX_GRACE_PERIOD_SECONDS {
            return Err(CapitalError::SubscriptionError(
                "grace period must be a positive window of at most 366 days".to_string(),
            ));
        }
        Ok(Self { starts_at, ends_at })
    }

    /// Inclusive beginning of the window as Unix seconds.
    pub fn starts_at(self) -> i64 {
        self.starts_at
    }

    /// Exclusive end of the window as Unix seconds.
    pub fn ends_at(self) -> i64 {
        self.ends_at
    }

    /// Returns whether the supplied trusted clock value is inside the window.
    pub fn contains(self, now: i64) -> bool {
        now >= self.starts_at && now < self.ends_at
    }

    /// Returns remaining whole seconds, or zero before/after the active window.
    pub fn remaining_seconds(self, now: i64) -> u64 {
        if !self.contains(now) {
            return 0;
        }
        u64::try_from(self.ends_at - now).unwrap_or(0)
    }
}

/// Provider-bound operations for one validated subscription identifier.
///
/// Constructing a handle does not load or authorize a subscription. The caller
/// must derive the identifier and optional grace period from authenticated,
/// authoritative application state.
pub struct SubscriptionHandle<'provider, Provider>
where
    Provider: BillingProvider + ?Sized,
{
    provider: &'provider Provider,
    subscription_id: String,
    grace_period: Option<GracePeriod>,
}

impl<'provider, Provider> SubscriptionHandle<'provider, Provider>
where
    Provider: BillingProvider + ?Sized,
{
    /// Binds a provider to one non-empty, bounded subscription ID.
    pub fn new(
        provider: &'provider Provider,
        subscription_id: impl Into<String>,
    ) -> Result<Self, CapitalError> {
        let subscription_id = subscription_id.into();
        if subscription_id.is_empty()
            || subscription_id.len() > 512
            || subscription_id.trim() != subscription_id
            || subscription_id.chars().any(char::is_control)
        {
            return Err(CapitalError::SubscriptionError(
                "subscription ID must contain 1 to 512 non-control characters without surrounding whitespace"
                    .to_string(),
            ));
        }
        Ok(Self {
            provider,
            subscription_id,
            grace_period: None,
        })
    }

    /// Attaches application-owned grace-period metadata to this handle.
    pub fn with_grace_period(mut self, grace_period: GracePeriod) -> Self {
        self.grace_period = Some(grace_period);
        self
    }

    /// Returns the provider subscription identifier.
    pub fn id(&self) -> &str {
        &self.subscription_id
    }

    /// Returns the optional application-owned grace-period policy value.
    pub fn grace_period(&self) -> Option<GracePeriod> {
        self.grace_period
    }

    /// Delegates cancellation to the selected provider adapter.
    pub async fn cancel(&self) -> Result<(), CapitalError> {
        self.provider
            .cancel_subscription(&self.subscription_id)
            .await
    }

    /// Delegates pausing to the selected provider adapter.
    pub async fn pause(&self) -> Result<(), CapitalError> {
        self.provider
            .pause_subscription(&self.subscription_id)
            .await
    }

    /// Validates and applies a provider coupon to this subscription.
    pub async fn apply_coupon(&self, coupon_code: &str) -> Result<(), CapitalError> {
        let coupon = crate::CouponCode::try_new(coupon_code)?;
        self.provider
            .apply_coupon(&self.subscription_id, coupon.as_str())
            .await
    }

    /// Extends the trial by whole days resolved against the current UTC clock.
    pub async fn extend_trial(&self, days: u16) -> Result<(), CapitalError> {
        let extension = TrialExtension::from_days(days)?;
        self.set_trial_end(extension.ends_at()).await
    }

    /// Extends the trial against an explicit trusted clock for workers and tests.
    pub async fn extend_trial_days_at(&self, days: u16, now: i64) -> Result<(), CapitalError> {
        let extension = TrialExtension::from_days_at(days, now)?;
        self.set_trial_end(extension.ends_at()).await
    }

    /// Sets an explicit provider trial end after local timestamp validation.
    pub async fn set_trial_end(&self, trial_ends_at: i64) -> Result<(), CapitalError> {
        crate::subscription::validate_trial_end(trial_ends_at)?;
        self.provider
            .extend_trial(&self.subscription_id, trial_ends_at)
            .await
    }
}

impl<Provider> std::fmt::Debug for SubscriptionHandle<'_, Provider>
where
    Provider: BillingProvider + ?Sized,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionHandle")
            .field("provider", &self.provider.name())
            .field("subscription_id", &"[REDACTED]")
            .field("grace_period", &self.grace_period)
            .finish()
    }
}

/// A trait applied to a domain struct (usually User) to grant it billing capabilities.
#[async_trait]
pub trait Billable {
    /// Returns the email associated with the billable entity.
    fn email(&self) -> String;

    /// Generates a checkout session URL for the user to subscribe to a specific plan.
    async fn subscribe(&self, plan_id: &str, redirect_url: &str) -> Result<String, CapitalError> {
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .create_checkout_session(&self.email(), plan_id, redirect_url)
            .await
    }

    /// Performs one validated immediate charge through an explicit provider.
    ///
    /// `amount_minor` is expressed in the currency's smallest unit. The caller
    /// must supply provider-tokenized customer/payment-method IDs and a unique
    /// application retry key; raw payment credentials are outside this API.
    async fn charge_with<Provider>(
        &self,
        selected_provider: &Provider,
        amount_minor: u64,
        currency: &str,
        customer_id: &str,
        payment_method_id: &str,
        idempotency_key: &str,
    ) -> Result<ChargeReceipt, CapitalError>
    where
        Provider: BillingProvider + ?Sized,
    {
        let request = ChargeRequest::new(
            amount_minor,
            currency,
            customer_id,
            self.email(),
            payment_method_id,
            idempotency_key,
        )?;
        selected_provider.charge(&request).await
    }

    /// Performs one validated immediate charge through the global provider.
    async fn charge(
        &self,
        amount_minor: u64,
        currency: &str,
        customer_id: &str,
        payment_method_id: &str,
        idempotency_key: &str,
    ) -> Result<ChargeReceipt, CapitalError> {
        let selected_provider = provider().ok_or_else(|| {
            CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
        })?;
        self.charge_with(
            selected_provider,
            amount_minor,
            currency,
            customer_id,
            payment_method_id,
            idempotency_key,
        )
        .await
    }

    /// Retrieves the current subscription ID from the entity if available.
    fn subscription_id(&self) -> Option<String> {
        None
    }

    /// Retrieves the current tier or plan ID from the entity if available.
    fn tier(&self) -> Option<String> {
        None
    }

    /// Returns application-owned grace-period state, when persisted by the model.
    fn grace_period(&self) -> Result<Option<GracePeriod>, CapitalError> {
        Ok(None)
    }

    /// Creates a statically dispatched handle using an explicit provider.
    fn subscription_with<'provider, Provider>(
        &self,
        selected_provider: &'provider Provider,
    ) -> Result<SubscriptionHandle<'provider, Provider>, CapitalError>
    where
        Provider: BillingProvider + ?Sized,
    {
        let subscription_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        let handle = SubscriptionHandle::new(selected_provider, subscription_id)?;
        Ok(match self.grace_period()? {
            Some(grace_period) => handle.with_grace_period(grace_period),
            None => handle,
        })
    }

    /// Creates a handle using the globally selected compatibility provider.
    fn subscription(
        &self,
    ) -> Result<SubscriptionHandle<'static, dyn BillingProvider>, CapitalError> {
        let subscription_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        let selected_provider = provider().ok_or_else(|| {
            CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
        })?;
        let handle = SubscriptionHandle::new(selected_provider, subscription_id)?;
        Ok(match self.grace_period()? {
            Some(grace_period) => handle.with_grace_period(grace_period),
            None => handle,
        })
    }

    /// Generate a customer billing portal URL.
    async fn billing_portal_url(&self, return_url: &str) -> Result<String, CapitalError> {
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .create_customer_portal(&self.email(), return_url)
            .await
    }

    /// Cancels the active subscription.
    async fn cancel_subscription(&self) -> Result<(), CapitalError> {
        self.subscription()?.cancel().await
    }

    /// Pauses the active subscription.
    async fn pause_subscription(&self) -> Result<(), CapitalError> {
        self.subscription()?.pause().await
    }

    /// Reports usage for metered billing.
    async fn report_usage(&self, metric: &str, quantity: u64) -> Result<(), CapitalError> {
        let sub_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .report_usage(&sub_id, metric, quantity)
            .await
    }

    /// Checks if the user is subscribed to the required tier.
    fn can_access(&self, required_tier: &str) -> bool {
        self.tier().map(|t| t == required_tier).unwrap_or(false)
    }

    /// Defines limits for a specific feature based on the current tier.
    /// Can be overridden by the implementor to provide dynamic tier-based limits.
    fn tier_limit(&self, _feature: &str) -> Option<usize> {
        None
    }

    /// Checks if a quota for a specific feature has been reached.
    fn check_quota(&self, feature: &str, current_usage: usize) -> bool {
        if let Some(limit) = self.tier_limit(feature) {
            current_usage < limit
        } else {
            false
        }
    }

    /// Builds a shared, idempotent quota request from this billable model's tier.
    ///
    /// Implement `Billable` on the subscription owner (for example a
    /// Workspace), derive `subject` from trusted tenant membership, and pass
    /// the result to a [`crate::QuotaStore`] before creating the resource.
    fn quota_request(
        &self,
        subject: crate::BillingSubject,
        feature: &str,
        event_key: impl Into<String>,
        units: u64,
    ) -> Result<crate::QuotaRequest, crate::QuotaError> {
        let limit = self.tier_limit(feature).ok_or_else(|| {
            crate::QuotaError::InvalidRequest(
                "billable tier does not define the requested feature".to_string(),
            )
        })?;
        let limit = u64::try_from(limit).map_err(|_| {
            crate::QuotaError::InvalidRequest("billable tier limit overflow".to_string())
        })?;
        crate::QuotaRequest::try_new(subject, feature, event_key, units, limit)
    }

    /// Applies a coupon code to the active subscription.
    async fn apply_coupon(&self, coupon_code: &str) -> Result<(), CapitalError> {
        self.subscription()?.apply_coupon(coupon_code).await
    }

    /// Extends a trial by 1 to 730 whole days from the current UTC clock.
    async fn extend_trial(&self, days: u16) -> Result<(), CapitalError> {
        self.subscription()?.extend_trial(days).await
    }

    /// Extends a trial by whole days from an explicit trusted clock.
    async fn extend_trial_days_at(&self, days: u16, now: i64) -> Result<(), CapitalError> {
        self.subscription()?.extend_trial_days_at(days, now).await
    }

    /// Sets an explicit provider timestamp for compatibility and reconciliation.
    async fn set_trial_end(&self, trial_ends_at: i64) -> Result<(), CapitalError> {
        self.subscription()?.set_trial_end(trial_ends_at).await
    }
}

#[cfg(test)]
#[path = "billable_tests.rs"]
mod tests;
