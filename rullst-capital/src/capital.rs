//! # Rullst Capital
//!
//! Billing and payout adapter foundations for Rullst applications.
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
        assert!(url.contains("test%40user.com"));
    }

    #[tokio::test]
    async fn test_mock_lemonsqueezy_provider() {
        let provider = LemonSqueezyProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "lemonsqueezy");

        let url = provider
            .create_checkout_session("test@user.com", "456", "https://app.com/success")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
        assert!(url.contains("test%40user.com"));
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
        assert!(url.contains("user%40empresa.com.br"));
    }

    #[tokio::test]
    async fn test_mock_polar_provider() {
        let provider = PolarProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "polar");

        let url = provider
            .create_checkout_session("dev@github.com", "tier_backer", "https://open.source/done")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_paddle_provider() {
        let provider = PaddleProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "paddle");

        let url = provider
            .create_checkout_session(
                "corp@enterprise.com",
                "pri_enterprise",
                "https://corp.com/cb",
            )
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_mercadopago_provider() {
        let provider = MercadoPagoProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "mercadopago");

        let url = provider
            .create_checkout_session("cliente@latam.com", "plan_latam", "https://saas.lat/ok")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_coinbase_provider() {
        let provider =
            CoinbaseCommerceProvider::new("mock_key".to_string(), "mock_secret".to_string());
        assert_eq!(provider.name(), "coinbase");

        let url = provider
            .create_checkout_session("satoshi@web3.org", "tier_crypto", "https://web3.app/verify")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_picpay_provider() {
        let provider = PicPayProvider::new("mock_picpay".to_string(), "mock_seller".to_string());
        assert_eq!(provider.name(), "picpay");

        let url = provider
            .create_checkout_session("cliente@picpay.com", "sub_basic", "https://app.com/done")
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_razorpay_provider() {
        let provider = RazorpayProvider::new(
            "mock_key_id".to_string(),
            "mock_key_secret".to_string(),
            "mock_webhook_secret".to_string(),
        );
        assert_eq!(provider.name(), "razorpay");

        let url = provider
            .create_checkout_session(
                "dev@bangalore.in",
                "plan_in_sub",
                "https://app.in/checkout/done",
            )
            .await
            .unwrap();
        assert!(url.contains("mock_session"));
    }

    #[tokio::test]
    async fn test_mock_wise_provider() {
        let provider = WiseProvider::new("mock_token".to_string(), "profile_123".to_string());
        assert_eq!(provider.name(), "wise");

        let transfer_id = provider
            .create_transfer("contractor@global.com", 250000, "USD")
            .await
            .unwrap();
        assert_eq!(transfer_id, "wise_tr_mock_contractor_global.com");

        let status = provider.get_transfer_status(&transfer_id).await.unwrap();
        assert_eq!(status, PayoutStatus::OutgoingPaymentSent);
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
    fn test_lemonsqueezy_signature_verification() {
        let provider = LemonSqueezyProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing X-Signature header".to_string())
        );

        headers.insert("x-signature".to_string(), "invalid".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());

        headers.insert("x-signature".to_string(), "deadbeef".to_string());
        let res3 = provider.handle_webhook(b"{}", &headers);
        assert!(res3.is_err());
        assert_eq!(
            res3.unwrap_err(),
            CapitalError::InvalidSignature(
                "LemonSqueezy signature verification failed".to_string()
            )
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_infinitepay_signature_verification() {
        let provider = InfinitePayProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing X-Signature header".to_string())
        );

        headers.insert("x-signature".to_string(), "deadbeef".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature("InfinitePay signature verification failed".to_string())
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_polar_signature_verification() {
        let provider = PolarProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing Polar-Signature header".to_string())
        );

        headers.insert("polar-signature".to_string(), "deadbeef".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature("Polar.sh signature verification failed".to_string())
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_coinbase_signature_verification() {
        let provider = CoinbaseCommerceProvider::new("mock".to_string(), "secret".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing X-CC-Webhook-Signature header".to_string())
        );

        headers.insert("x-cc-webhook-signature".to_string(), "deadbeef".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature(
                "Coinbase Commerce signature verification failed".to_string()
            )
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_picpay_token_verification() {
        let provider = PicPayProvider::new("tok".to_string(), "my_secret_token".to_string());

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing x-seller-token header".to_string())
        );

        headers.insert("x-seller-token".to_string(), "wrong_token".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature("PicPay seller token verification failed".to_string())
        );
    }

    #[test]
    #[cfg(not(miri))]
    fn test_razorpay_signature_verification() {
        let provider = RazorpayProvider::new(
            "key_id".to_string(),
            "key_secret".to_string(),
            "secret".to_string(),
        );

        let mut headers = HashMap::new();
        let res = provider.handle_webhook(b"{}", &headers);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            CapitalError::InvalidSignature("Missing X-Razorpay-Signature header".to_string())
        );

        headers.insert("x-razorpay-signature".to_string(), "deadbeef".to_string());
        let res2 = provider.handle_webhook(b"{}", &headers);
        assert!(res2.is_err());
        assert_eq!(
            res2.unwrap_err(),
            CapitalError::InvalidSignature("Razorpay signature verification failed".to_string())
        );
    }
}
