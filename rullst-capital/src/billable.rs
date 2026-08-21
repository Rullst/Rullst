use crate::capital::provider;
use crate::error::CapitalError;
use async_trait::async_trait;

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
        let sub_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .cancel_subscription(&sub_id)
            .await
    }

    /// Pauses the active subscription.
    async fn pause_subscription(&self) -> Result<(), CapitalError> {
        let sub_id = self.subscription_id().ok_or_else(|| {
            CapitalError::SubscriptionError("No subscription ID available".to_string())
        })?;
        provider()
            .ok_or_else(|| {
                CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
            })?
            .pause_subscription(&sub_id)
            .await
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

    struct TestUser;

    #[async_trait]
    impl Billable for TestUser {
        fn email(&self) -> String {
            "test@example.com".to_string()
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

        let res = u.subscribe("pro", "http://return").await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            crate::error::CapitalError::ConfigurationError(
                "BillingProvider not initialized".to_string()
            )
        );
    }
}
