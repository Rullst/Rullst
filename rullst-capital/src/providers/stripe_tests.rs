use super::*;

#[tokio::test]
async fn test_stripe_provider_methods() {
    let provider = StripeProvider::new("mock_key", "whsec_stripe123");
    assert_eq!(provider.name(), "stripe");

    let url = provider
        .create_checkout_session(
            "user@stripe.com",
            "price_pro_month",
            "https://app.com/success",
        )
        .await
        .unwrap();
    assert!(url.contains("checkout.stripe.com"));
    assert!(url.contains("price_pro_month"));
    assert!(
        provider
            .create_checkout_session("", "plan", "url")
            .await
            .is_err()
    );
    assert!(
        provider
            .create_checkout_session("email", "", "url")
            .await
            .is_err()
    );

    let portal = provider
        .create_customer_portal("user@stripe.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("billing.stripe.com/p/session/mock_portal"));
    assert!(provider.create_customer_portal("", "url").await.is_err());

    assert!(provider.cancel_subscription("sub_str").await.is_ok());
    assert!(provider.cancel_subscription("").await.is_err());
    assert!(provider.pause_subscription("sub_str").await.is_ok());
    assert!(provider.pause_subscription("").await.is_err());
    assert!(provider.report_usage("sub_str", "api", 10).await.is_ok());
    assert!(provider.report_usage("", "api", 10).await.is_err());
    assert!(provider.apply_coupon("sub_str", "PROMO10").await.is_ok());
    assert!(provider.apply_coupon("", "PROMO10").await.is_err());
    assert!(provider.apply_coupon("sub_str", "").await.is_err());
    assert!(provider.extend_trial("sub_str", 1800000000).await.is_ok());
    assert!(provider.extend_trial("", 1800000000).await.is_err());
    assert!(provider.extend_trial("sub_str", -1).await.is_err());

    let secret = "whsec_stripe123";
    let now = chrono::Utc::now().timestamp();
    let timestamp = now.to_string();
    let payload = br#"{"data":{"object":{"id":"sub_str_100","customer":"cus_123","customer_email":"user@stripe.com","status":"active","items":{"data":[{"price":{"id":"price_pro"}}]}}}}"#;

    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let mut ctx = hmac::Context::with_key(&key);
    ctx.update(timestamp.as_bytes());
    ctx.update(b".");
    ctx.update(payload);
    let sig_hex = hex::encode(ctx.sign().as_ref());
    let valid_header = format!("t={timestamp},v1={sig_hex}");

    assert!(
        provider
            .verify_signature_at(payload, &valid_header, now)
            .is_ok()
    );
    assert!(matches!(
        provider.verify_signature_at(payload, &valid_header, now + 301),
        Err(CapitalError::StaleWebhook(_))
    ));

    let no_secret = StripeProvider::new("k", "");
    assert!(matches!(
        no_secret.verify_signature(payload, ""),
        Err(CapitalError::ConfigurationError(_))
    ));
    assert!(
        provider
            .verify_signature(payload, "invalid_header")
            .is_err()
    );
    assert!(
        provider
            .verify_signature(payload, "t=123,v1=invalid_hex_!")
            .is_err()
    );
    assert!(
        provider
            .verify_signature(
                payload,
                "t=123,v1=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
            )
            .is_err()
    );

    let mut headers = HashMap::new();
    headers.insert("stripe-signature".to_string(), valid_header);
    let event = provider.handle_webhook(payload, &headers).unwrap();
    assert_eq!(event.subscription_id, "sub_str_100");
    assert_eq!(event.customer_id, "cus_123");
    assert_eq!(event.customer_email, "user@stripe.com");
    assert_eq!(event.status, SubscriptionStatus::Active);

    let empty_headers = HashMap::new();
    assert!(provider.handle_webhook(payload, &empty_headers).is_err());
    assert!(provider.handle_webhook(b"invalid json", &headers).is_err());
}
