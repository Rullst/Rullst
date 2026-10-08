//! # Rullst Capital
//!
//! Provider-neutral billing foundations with Stripe and InfinitePay adapters.
//! Provider capabilities and webhook protocols vary; applications must verify
//! the exact live methods they use and reconcile durable state themselves.

pub use crate::providers::*;

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::error::CapitalError;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_mock_stripe_provider() {
        let provider = StripeProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "stripe");

        let url = provider
            .create_checkout_session("test@user.com", "price_123", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
        assert!(!url.contains("user.com"));
    }

    #[tokio::test]
    async fn test_mock_infinitepay_provider() {
        let provider = InfinitePayProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "infinitepay");

        let url = provider
            .create_checkout_session(
                "user@empresa.com.br",
                "plan_pro",
                "https://meusaas.com.br/ok",
            )
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
        assert!(url.starts_with("https://mock.infinitepay.invalid/"));
        assert!(!url.contains("empresa") && !url.contains("meusaas"));
    }

    #[test]
    fn test_subscription_status_parsing() {
        assert_eq!(
            SubscriptionStatus::parse_status("active"),
            SubscriptionStatus::Active
        );
        assert_eq!(
            SubscriptionStatus::parse_status("Canceled"),
            SubscriptionStatus::Canceled
        );
        assert_eq!(
            SubscriptionStatus::parse_status("trialing"),
            SubscriptionStatus::Trialing
        );
        assert_eq!(
            SubscriptionStatus::parse_status("past_due"),
            SubscriptionStatus::PastDue
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unpaid"),
            SubscriptionStatus::Unpaid
        );
        assert_eq!(
            SubscriptionStatus::parse_status("paused"),
            SubscriptionStatus::Paused
        );
        assert_eq!(
            SubscriptionStatus::parse_status("unknown_garbage"),
            SubscriptionStatus::Unpaid
        );
    }

    #[test]
    fn test_subscription_status_as_str() {
        assert_eq!(SubscriptionStatus::Active.as_str(), "active");
        assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
        assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
        assert_eq!(SubscriptionStatus::Unpaid.as_str(), "unpaid");
        assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
        assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");
    }

    #[test]
    #[cfg(not(miri))]
    // TM-PAY-01: forged, malformed and stale Stripe signatures are rejected.
    fn test_stripe_signature_verification() {
        let provider = StripeProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing stripe-signature header".to_string())
        );

        headers.insert("stripe-signature".to_string(), "invalid_format".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature("Invalid Stripe-Signature header format".to_string())
        );

        headers.insert(
            "stripe-signature".to_string(),
            "t=123,v1=not_hex!!".to_string(),
        );
        let res3 = provider.handle_webhook(b"{}", &headers);
        assert!(res3.is_err());

        headers.insert(
            "stripe-signature".to_string(),
            "t=123,v1=deadbeef".to_string(),
        );
        let res4 = provider.handle_webhook(b"{}", &headers);
        assert!(res4.is_err());
        assert_eq!(
            res4.unwrap_err(),
            CapitalError::InvalidSignature("Stripe signature verification failed".to_string())
        );
    }

    #[test]
    fn test_stripe_signature_empty_secret() {
        let provider = StripeProvider::new("mock".to_string(), "".to_string());
        let res = provider.verify_signature(b"{}", "invalid_signature");
        assert!(matches!(res, Err(CapitalError::ConfigurationError(_))));
    }

    #[test]
    #[cfg(not(miri))]
    fn test_infinitepay_signature_verification() {
        let provider = InfinitePayProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert!(matches!(res, Err(CapitalError::UnsupportedOperation(_))));

        headers.insert("x-signature".to_string(), "deadbeef".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert!(matches!(res2, Err(CapitalError::UnsupportedOperation(_))));
    }
}
