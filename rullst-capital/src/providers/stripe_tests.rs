use super::*;
use crate::SubscriptionStatus;

#[test]
fn stripe_rotation_accepts_any_bounded_v1_and_rejects_ambiguous_timestamps() {
    let provider = StripeProvider::new("mock_key", "rotation-secret");
    let payload = b"exact raw webhook bytes";
    let now = 1_800_000_000;
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"rotation-secret");
    let mut context = hmac::Context::with_key(&key);
    context.update(format!("{now}.").as_bytes());
    context.update(payload);
    let signature = hex::encode(context.sign().as_ref());
    let invalid = "00".repeat(32);
    for candidates in [
        format!("v1={signature},v1={invalid},v1=malformed"),
        format!("v1={invalid},v1={signature},v1=malformed"),
        format!("v1=malformed,v1={invalid},v1={signature}"),
    ] {
        let header = format!("t={now},{candidates}");
        assert!(provider.verify_signature_at(payload, &header, now).is_ok());
        assert!(matches!(
            provider.verify_signature_at(payload, &header, now + 301),
            Err(CapitalError::StaleWebhook(_))
        ));
        assert!(
            provider
                .verify_signature_at(b"changed body", &header, now)
                .is_err()
        );
    }
    for header in [
        format!("t={now},v1={invalid},v1=malformed"),
        format!("t={now},t={now},v1={signature}"),
        format!("t=wrong,t={now},v1={signature}"),
        format!("t={now},v1={signature},{}", "v1=bad,".repeat(16)),
        format!("t={now},v1={signature},v0={}", "x".repeat(4096)),
    ] {
        assert!(provider.verify_signature_at(payload, &header, now).is_err());
    }
}

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
    assert!(url.starts_with("https://mock.stripe.invalid/checkout/mock_session?plan="));
    assert!(!url.contains("%40") && !url.contains("app.com"));
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
    assert_eq!(portal, "https://mock.stripe.invalid/portal/mock_portal");
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
    let payload = br#"{"type":"customer.subscription.updated","data":{"object":{"object":"subscription","id":"sub_str_100","customer":"cus_123","customer_email":"user@stripe.com","status":"active","items":{"has_more":false,"data":[{"price":{"id":"price_pro"}}]}}}}"#;

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

fn mock_signed() -> HashMap<String, String> {
    HashMap::from([("stripe-signature".to_string(), "mock_bounds".to_string())])
}

fn subscription_payload(email: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "customer.subscription.updated",
        "data": {"object": {
            "object": "subscription", "id": "sub_bounds", "customer": "cus_bounds",
            "customer_email": email, "status": "active",
            "items": {"has_more": false, "data": [{"price": {"id": "price_bounds"}}]}
        }}
    }))
    .unwrap()
}

#[test]
fn subscription_webhook_payload_bounds_are_inclusive() {
    let provider = StripeProvider::new("mock_key", "mock_bounds");
    let too_large = CapitalError::PayloadParseError(
        "Stripe: subscription payload exceeds the supported bound".into(),
    );
    let not_json = CapitalError::PayloadParseError("Stripe: invalid subscription JSON".into());
    assert_eq!(
        provider.handle_webhook(b"", &mock_signed()).unwrap_err(),
        too_large
    );
    assert_eq!(
        provider
            .handle_webhook(&vec![b'x'; 2_097_152], &mock_signed())
            .unwrap_err(),
        not_json
    );
    assert_eq!(
        provider
            .handle_webhook(&vec![b'x'; 2_097_153], &mock_signed())
            .unwrap_err(),
        too_large
    );
    // The envelope verifier applies the same 2 MiB bound before delegating.
    assert_eq!(
        provider
            .verify_subscription_event(&vec![b'x'; 2_097_152], &mock_signed())
            .unwrap_err(),
        not_json
    );
}

#[test]
fn subscription_contact_email_is_bounded_and_printable() {
    let provider = StripeProvider::new("mock_key", "mock_bounds");
    let longest = format!("{}@example.io", "a".repeat(243));
    assert_eq!(longest.len(), 254);
    let event = provider
        .handle_webhook(&subscription_payload(&longest), &mock_signed())
        .unwrap();
    assert_eq!(event.customer_email, longest);
    for invalid in [
        format!("{}@example.io", "a".repeat(244)),
        "a\u{7}@x.io".to_string(),
    ] {
        assert!(matches!(
            provider.handle_webhook(&subscription_payload(&invalid), &mock_signed()),
            Err(CapitalError::PayloadParseError(_))
        ));
    }
}

#[test]
fn signature_header_of_exactly_4096_bytes_is_parsed() {
    let provider = StripeProvider::new("mock_key", "header-secret");
    let payload = b"bounded header";
    let now = 1_800_000_000;
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"header-secret");
    let mut context = hmac::Context::with_key(&key);
    context.update(format!("{now}.").as_bytes());
    context.update(payload);
    let prefix = format!("t={now},v1={},v0=", hex::encode(context.sign().as_ref()));
    let header = format!("{prefix}{}", "x".repeat(4096 - prefix.len()));
    assert_eq!(header.len(), 4096);
    provider.verify_signature_at(payload, &header, now).unwrap();
    let longer = format!("{header}x");
    assert!(matches!(
        provider.verify_signature_at(payload, &longer, now),
        Err(CapitalError::InvalidSignature(_))
    ));
}

#[tokio::test]
async fn offline_keys_skip_id_validation_and_live_keys_enforce_it() {
    // An empty or mock key never reaches the provider, so the ID is not checked;
    // a live key validates the URL segment before any request is built.
    for key in ["", "mock_key"] {
        let provider = StripeProvider::new(key, "whsec_unused");
        provider
            .cancel_subscription("not a path segment")
            .await
            .unwrap();
        provider
            .pause_subscription("not a path segment")
            .await
            .unwrap();
    }
    let live = StripeProvider::new("sk_live_unused", "whsec_unused");
    assert!(matches!(
        live.cancel_subscription("not a path segment").await,
        Err(CapitalError::SubscriptionError(_))
    ));
    assert!(matches!(
        live.pause_subscription("..").await,
        Err(CapitalError::SubscriptionError(_))
    ));
}
