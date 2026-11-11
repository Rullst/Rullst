//! The 13.0.0-alpha.4 constructor, and the operations InfinitePay lacks.

use super::*;

#[tokio::test]
async fn the_legacy_constructor_has_no_handle_for_live_operations() {
    let legacy = InfinitePayProvider::new("inchk_live", "live-secret");
    assert!(!legacy.is_mock());
    assert!(matches!(
        legacy.create_one_off_checkout(&request()).await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        legacy
            .confirm_one_off_payment(&request(), &notification())
            .await,
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(matches!(
        legacy
            .create_checkout_session("a@example.com", "plan", "https://app.example")
            .await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
    assert_eq!(
        legacy.webhook_verification_mode().unwrap(),
        WebhookVerificationMode::ConfirmedAtSource
    );
    assert!(matches!(
        legacy.handle_webhook(b"{}", &HashMap::new()),
        Err(CapitalError::UnsupportedOperation(_))
    ));
    assert!(matches!(
        legacy.verify_signature(b"{}", "live-secret"),
        Err(CapitalError::UnsupportedOperation(_))
    ));
}

#[tokio::test]
async fn subscription_portal_and_plan_operations_are_unsupported() {
    for provider in [
        InfinitePayProvider::checkout("loja").unwrap(),
        InfinitePayProvider::checkout("mock_loja").unwrap(),
        InfinitePayProvider::new("mock_key", "mock_webhook"),
    ] {
        let unsupported = |result: Result<(), CapitalError>| {
            assert!(matches!(result, Err(CapitalError::UnsupportedOperation(_))))
        };
        unsupported(
            provider
                .create_customer_portal("a@example.com", "https://app.example")
                .await
                .map(|_| ()),
        );
        unsupported(provider.cancel_subscription("sub").await);
        unsupported(provider.pause_subscription("sub").await);
        unsupported(provider.report_usage("sub", "api", 1).await);
        unsupported(provider.apply_coupon("sub", "CODE").await);
        unsupported(provider.extend_trial("sub", 1_800_000_000).await);
    }
    let live = InfinitePayProvider::checkout("loja").unwrap();
    assert!(matches!(
        live.create_checkout_session("a@example.com", "plan", "https://app.example")
            .await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
    let mock = InfinitePayProvider::checkout("mock_loja").unwrap();
    assert!(
        mock.create_checkout_session("a@example.com", "plan", "https://app.example")
            .await
            .unwrap()
            .starts_with("https://mock.infinitepay.invalid/checkout/")
    );
    assert!(mock.create_checkout_session("", "plan", "u").await.is_err());
    assert!(
        mock.create_checkout_session("a@example.com", " ", "u")
            .await
            .is_err()
    );
}

#[test]
fn the_legacy_mock_fixture_still_reads_a_mock_signed_body() {
    let provider = InfinitePayProvider::new("mock_key", "mock_webhook");
    assert_eq!(
        provider.webhook_verification_mode().unwrap(),
        WebhookVerificationMode::Mock
    );
    let payload = br#"{"id":"tx_inf_100","customer":{"id":"c1","email":"user@infinite.com"},"plan_id":"plan_starter","status":"paid"}"#;
    let headers = HashMap::from([("x-signature".to_string(), "mock_webhook".to_string())]);
    let event = provider.handle_webhook(payload, &headers).unwrap();
    assert_eq!(event.subscription_id, "tx_inf_100");
    assert_eq!(event.customer_id, "c1");
    assert_eq!(event.customer_email, "user@infinite.com");
    assert_eq!(event.plan_id, "plan_starter");
    assert_eq!(event.status, SubscriptionStatus::Active);
    let fallback = provider
        .handle_webhook(br#"{"transaction_id":"t","event":"failed"}"#, &headers)
        .unwrap();
    assert_eq!(fallback.subscription_id, "t");
    assert_eq!(fallback.plan_id, "default");
    assert_eq!(fallback.status, SubscriptionStatus::Unpaid);
    assert!(provider.handle_webhook(payload, &HashMap::new()).is_err());
    assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
    assert!(
        provider
            .handle_webhook(br#"{"status":" "}"#, &headers)
            .is_err()
    );
    let wrong = HashMap::from([("x-signature".to_string(), "other".to_string())]);
    assert!(provider.handle_webhook(payload, &wrong).is_err());
    assert!(matches!(
        InfinitePayProvider::new("k", "").verify_signature(payload, ""),
        Err(CapitalError::ConfigurationError(_))
    ));
    // Stripe is untouched by the unsigned mode.
    assert_eq!(
        StripeProvider::new("mock_key", "whsec_live")
            .webhook_verification_mode()
            .unwrap(),
        WebhookVerificationMode::Real
    );
}
