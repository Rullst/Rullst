use super::*;

#[test]
fn test_subscription_status_variants() {
    assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
    assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
    assert_eq!(SubscriptionStatus::Unpaid.as_str(), "unpaid");
    assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
    assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");

    assert_eq!(
        SubscriptionStatus::parse_status("active"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("paid"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("completed"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("approved"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("resolved"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("canceled"),
        SubscriptionStatus::Canceled
    );
    assert_eq!(
        SubscriptionStatus::parse_status("cancelled"),
        SubscriptionStatus::Canceled
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
        SubscriptionStatus::parse_status("failed"),
        SubscriptionStatus::Unpaid
    );
    assert_eq!(
        SubscriptionStatus::parse_status("rejected"),
        SubscriptionStatus::Unpaid
    );
    assert_eq!(
        SubscriptionStatus::parse_status("expired"),
        SubscriptionStatus::Unpaid
    );
    assert_eq!(
        SubscriptionStatus::parse_status("trialing"),
        SubscriptionStatus::Trialing
    );
    assert_eq!(
        SubscriptionStatus::parse_status("paused"),
        SubscriptionStatus::Paused
    );
    assert_eq!(
        SubscriptionStatus::parse_status("unknown_xyz"),
        SubscriptionStatus::Unpaid
    );
}

#[test]
fn test_url_encoding_characters() {
    assert_eq!(url_encode("hello world"), "hello%20world");
    assert_eq!(url_encode("foo/bar?baz=1"), "foo%2Fbar%3Fbaz%3D1");
    assert_eq!(url_encode("user@domain.com"), "user%40domain.com");
    assert_eq!(url_encode("simple_word-123.test~"), "simple_word-123.test~");
}

#[test]
fn explicit_mock_webhook_mode_still_requires_its_secret() {
    let provider = StripeProvider::new("mock_api_key", "mock_webhook_secret");
    assert_eq!(
        provider.webhook_verification_mode().unwrap(),
        WebhookVerificationMode::Mock
    );
    assert!(
        provider
            .verify_signature(b"{}", "mock_webhook_secret")
            .is_ok()
    );
    assert!(provider.verify_signature(b"{}", "wrong").is_err());

    let empty = StripeProvider::new("mock_api_key", "");
    assert!(matches!(
        empty.webhook_verification_mode(),
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        empty.handle_webhook(b"{}", &HashMap::new()),
        Err(CapitalError::ConfigurationError(_))
    ));
}

#[test]
fn test_global_providers_initialization() {
    let stripe = StripeProvider::new("mock_key", "sec_stripe");
    init_provider(Box::new(stripe));
    assert!(provider().is_some());
    assert_eq!(provider().unwrap().name(), "stripe");

    let wise = WiseProvider::new("mock_token", "sec_wise");
    init_payout_provider(Box::new(wise));
    assert!(payout_provider().is_some());
    assert_eq!(payout_provider().unwrap().name(), "wise");
}
