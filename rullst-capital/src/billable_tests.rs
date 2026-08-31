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
    let user = TestUser;
    assert_eq!(user.email(), "test@example.com");
    assert_eq!(user.subscription_id(), None);
    assert_eq!(user.tier(), None);
    assert_eq!(user.tier_limit("cpu"), None);
    assert!(!user.check_quota("cpu", 10));
    assert!(!user.can_access("pro"));

    let result = user.subscribe("pro", "http://return").await;
    assert_eq!(
        result.expect_err("provider is absent"),
        CapitalError::ConfigurationError("BillingProvider not initialized".to_string())
    );
    assert!(user.billing_portal_url("http://return").await.is_err());
    assert!(matches!(
        user.cancel_subscription().await,
        Err(CapitalError::SubscriptionError(_))
    ));
    assert!(matches!(
        user.pause_subscription().await,
        Err(CapitalError::SubscriptionError(_))
    ));
    assert!(matches!(
        user.report_usage("api_calls", 5).await,
        Err(CapitalError::SubscriptionError(_))
    ));
    assert!(matches!(
        user.apply_coupon("DISCOUNT10").await,
        Err(CapitalError::SubscriptionError(_))
    ));
    assert!(matches!(
        user.extend_trial(15).await,
        Err(CapitalError::SubscriptionError(_))
    ));
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

    let quota = pro
        .quota_request(
            crate::BillingSubject::try_new("workspace", "acme").expect("subject"),
            "api_calls",
            "request-17",
            5,
        )
        .expect("quota request");
    assert_eq!(quota.limit(), 1_000);
    assert_eq!(quota.units(), 5);
    assert!(
        pro.quota_request(
            crate::BillingSubject::try_new("workspace", "acme").expect("subject"),
            "unknown",
            "request-18",
            1,
        )
        .is_err()
    );

    assert!(matches!(
        pro.cancel_subscription().await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        pro.pause_subscription().await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        pro.report_usage("api_calls", 10).await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        pro.apply_coupon("DISCOUNT").await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        pro.extend_trial_days_at(15, 1_800_000_000).await,
        Err(CapitalError::ConfigurationError(_))
    ));
}

#[test]
fn grace_period_is_bounded_and_uses_half_open_clock_semantics() {
    let grace = GracePeriod::new(1_000, 1_100).expect("grace period");
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
async fn explicit_subscription_handle_is_static_bounded_and_redacted() {
    let provider = StripeProvider::new("mock_api", "mock_webhook");
    let pro = ProUser;
    let handle = pro
        .subscription_with(&provider)
        .expect("subscription handle");

    assert_eq!(handle.id(), "sub_12345");
    assert!(handle.grace_period().is_some());
    assert!(handle.cancel().await.is_ok());
    assert!(handle.pause().await.is_ok());
    assert!(handle.apply_coupon("BLACK_FRIDAY").await.is_ok());
    assert!(handle.extend_trial_days_at(15, 1_800_000_000).await.is_ok());
    assert!(handle.extend_trial_days_at(0, 1_800_000_000).await.is_err());
    assert!(handle.set_trial_end(0).await.is_err());
    let debug = format!("{handle:?}");
    assert!(debug.contains("stripe"));
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("sub_12345"));

    assert!(SubscriptionHandle::new(&provider, "").is_err());
    assert!(SubscriptionHandle::new(&provider, "line\nbreak").is_err());
    assert!(SubscriptionHandle::new(&provider, "x".repeat(513)).is_err());
}
