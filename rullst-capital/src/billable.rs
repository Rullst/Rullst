use crate::capital::{BillingProvider, provider};
use crate::error::CapitalError;
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

    /// Applies a coupon code to the active subscription.
    async fn apply_coupon(&self, coupon_code: &str) -> Result<(), CapitalError> {
        let sub_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .apply_coupon(&sub_id, coupon_code)
            .await
    }

    /// Extends a trial by setting a new expiration timestamp.
    async fn extend_trial(&self, trial_ends_at: i64) -> Result<(), CapitalError> {
        let sub_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .extend_trial(&sub_id, trial_ends_at)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::StripeProvider;

    struct TestUser;

    #[async_trait]
    impl Billable for TestUser {
        fn email(&self) -> String {
            "test@example.com".to_string()
        }
    }

    struct ProUser;

    #[async_trait]
    impl Billable for ProUser {
        fn email(&self) -> String {
            "pro@example.com".to_string()
        }
        fn subscription_id(&self) -> Option<String> {
            Some("sub_12345".to_string())
        }
        fn tier(&self) -> Option<String> {
            Some("pro".to_string())
        }
        fn grace_period(&self) -> Result<Option<GracePeriod>, CapitalError> {
            GracePeriod::new(1_700_000_000, 1_700_086_400).map(Some)
        }
        fn tier_limit(&self, feature: &str) -> Option<usize> {
            match feature {
                "api_calls" => Some(1000),
                _ => None,
            }
        }
    }

    #[tokio::test]
    async fn test_billable_defaults() {
        let u = TestUser;
        assert_eq!(u.email(), "test@example.com");
        assert_eq!(u.subscription_id(), None);
        assert_eq!(u.tier(), None);
        assert_eq!(u.tier_limit("cpu"), None);
        assert!(!u.check_quota("cpu", 10));
        assert!(!u.can_access("pro"));

        let res = u.subscribe("pro", "http://return").await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            crate::error::CapitalError::ConfigurationError(
                "BillingProvider not initialized".to_string()
            )
        );

        let res = u.billing_portal_url("http://return").await;
        assert!(res.is_err());

        let res = u.cancel_subscription().await;
        assert!(matches!(res, Err(CapitalError::SubscriptionError(_))));

        let res = u.pause_subscription().await;
        assert!(matches!(res, Err(CapitalError::SubscriptionError(_))));

        let res = u.report_usage("api_calls", 5).await;
        assert!(matches!(res, Err(CapitalError::SubscriptionError(_))));

        let res = u.apply_coupon("DISCOUNT10").await;
        assert!(matches!(res, Err(CapitalError::SubscriptionError(_))));

        let res = u.extend_trial(1700000000).await;
        assert!(matches!(res, Err(CapitalError::SubscriptionError(_))));
    }

    #[tokio::test]
    async fn test_billable_custom_implementation() {
        let pro = ProUser;
        assert_eq!(pro.subscription_id().as_deref(), Some("sub_12345"));
        assert_eq!(pro.tier().as_deref(), Some("pro"));
        assert!(pro.can_access("pro"));
        assert!(!pro.can_access("enterprise"));

        assert_eq!(pro.tier_limit("api_calls"), Some(1000));
        assert_eq!(pro.tier_limit("unknown"), None);
        assert!(pro.check_quota("api_calls", 500));
        assert!(!pro.check_quota("api_calls", 1500));
        assert!(!pro.check_quota("unknown", 0));

        let res = pro.cancel_subscription().await;
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));

        let res = pro.pause_subscription().await;
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));

        let res = pro.report_usage("api_calls", 10).await;
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));

        let res = pro.apply_coupon("DISCOUNT").await;
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));

        let res = pro.extend_trial(1700000000).await;
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));
    }

    #[test]
    fn grace_period_is_bounded_and_uses_half_open_clock_semantics() {
        let grace = GracePeriod::new(1_000, 1_100).unwrap();
        assert_eq!(grace.starts_at(), 1_000);
        assert_eq!(grace.ends_at(), 1_100);
        assert!(!grace.contains(999));
        assert!(grace.contains(1_000));
        assert!(grace.contains(1_099));
        assert!(!grace.contains(1_100));
        assert_eq!(grace.remaining_seconds(1_050), 50);
        assert_eq!(grace.remaining_seconds(999), 0);
        assert_eq!(grace.remaining_seconds(1_100), 0);

        assert!(GracePeriod::new(-1, 1).is_err());
        assert!(GracePeriod::new(1_000, 1_000).is_err());
        assert!(GracePeriod::new(1_000, 999).is_err());
        assert!(GracePeriod::new(1_000, 1_000 + MAX_GRACE_PERIOD_SECONDS + 1).is_err());
    }

    #[tokio::test]
    async fn explicit_subscription_handle_uses_static_provider_and_redacts_its_id() {
        let provider = StripeProvider::new("mock_api", "mock_webhook");
        let pro = ProUser;
        let handle = pro.subscription_with(&provider).unwrap();

        assert_eq!(handle.id(), "sub_12345");
        assert!(handle.grace_period().is_some());
        assert!(handle.cancel().await.is_ok());
        assert!(handle.pause().await.is_ok());
        let debug = format!("{handle:?}");
        assert!(debug.contains("stripe"));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sub_12345"));

        assert!(SubscriptionHandle::new(&provider, "").is_err());
        assert!(SubscriptionHandle::new(&provider, "line\nbreak").is_err());
        assert!(SubscriptionHandle::new(&provider, "x".repeat(513)).is_err());
    }
}
