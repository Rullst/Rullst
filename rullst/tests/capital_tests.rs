use rullst::capital::{BillingProvider, StripeProvider, SubscriptionStatus};

#[tokio::test]
async fn test_subscription_status_parsing() {
    assert_eq!(
        SubscriptionStatus::parse_status("active"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("CANCELED"),
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
        SubscriptionStatus::parse_status("trialing"),
        SubscriptionStatus::Trialing
    );
    assert_eq!(
        SubscriptionStatus::parse_status("paused"),
        SubscriptionStatus::Paused
    );
    assert_eq!(
        SubscriptionStatus::parse_status("unknown_random_string"),
        SubscriptionStatus::Unpaid
    );
}

#[tokio::test]
async fn test_subscription_status_as_str() {
    assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
}

#[tokio::test]
async fn test_stripe_provider_mock_checkout() {
    let provider = StripeProvider::new("mock_key".to_string(), "secret".to_string());
    assert_eq!(provider.name(), "stripe");

    let url = provider
        .create_checkout_session(
            "test@test.com",
            "plan_123",
            "https://app.rullst.test/success",
        )
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(url.contains("test%40test.com"));
    assert!(url.contains("plan_123"));
}

#[tokio::test]
async fn test_stripe_provider_webhook_parsing() {
    let provider = StripeProvider::new("mock_key".to_string(), "".to_string());

    let payload = serde_json::json!({
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "id": "sub_123",
                "customer": "cus_123",
                "status": "active",
                "current_period_end": 1700000000,
                "email": "test@test.com",
                "items": {
                    "data": [
                        { "price": { "id": "price_123" } }
                    ]
                }
            }
        }
    })
    .to_string();

    let headers = std::collections::HashMap::new();
    // With empty secret, it skips signature verification
    let event = provider
        .handle_webhook(payload.as_bytes(), &headers)
        .unwrap();
    assert_eq!(event.subscription_id, "sub_123");
    assert_eq!(event.status, SubscriptionStatus::Active);
    assert_eq!(event.customer_email, "test@test.com");
    assert_eq!(event.ends_at, Some(1700000000));
}

#[tokio::test]
async fn test_stripe_provider_webhook_uninteresting() {
    let provider = StripeProvider::new("mock_key".to_string(), "".to_string());
    let payload = serde_json::json!({
        "type": "invoice.paid"
    })
    .to_string();

    let headers = std::collections::HashMap::new();
    let res = provider.handle_webhook(payload.as_bytes(), &headers);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Uninteresting"));
}

#[tokio::test]
async fn test_lemonsqueezy_provider_mock_checkout() {
    use rullst::capital::LemonSqueezyProvider;
    let provider = LemonSqueezyProvider::new("mock_key".to_string(), "secret".to_string());
    assert_eq!(provider.name(), "lemonsqueezy");

    let url = provider
        .create_checkout_session(
            "test@test.com",
            "variant_1",
            "https://app.rullst.test/success",
        )
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(url.contains("test%40test.com"));
    assert!(url.contains("variant_1"));
}

#[tokio::test]
async fn test_lemonsqueezy_provider_webhook_parsing() {
    use rullst::capital::LemonSqueezyProvider;
    let provider = LemonSqueezyProvider::new("mock_key".to_string(), "".to_string());

    let payload = serde_json::json!({
        "meta": {
            "event_name": "subscription_created"
        },
        "data": {
            "id": "sub_456",
            "attributes": {
                "customer_id": 999,
                "user_email": "lemon@test.com",
                "variant_id": 123,
                "status": "past_due",
                "ends_at": "2023-10-01T10:00:00Z"
            }
        }
    })
    .to_string();

    let headers = std::collections::HashMap::new();
    let event = provider
        .handle_webhook(payload.as_bytes(), &headers)
        .unwrap();
    assert_eq!(event.subscription_id, "sub_456");
    assert_eq!(event.customer_id, "999");
    assert_eq!(event.customer_email, "lemon@test.com");
    assert_eq!(event.plan_id, "123");
    assert_eq!(event.status, SubscriptionStatus::PastDue);
}

#[tokio::test]
async fn test_lemonsqueezy_webhook_uninteresting() {
    use rullst::capital::LemonSqueezyProvider;
    let provider = LemonSqueezyProvider::new("mock_key".to_string(), "".to_string());
    let payload = serde_json::json!({
        "meta": {
            "event_name": "order_created"
        }
    })
    .to_string();

    let headers = std::collections::HashMap::new();
    let res = provider.handle_webhook(payload.as_bytes(), &headers);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Uninteresting"));
}

#[tokio::test]
async fn test_stripe_signature_verification_failure() {
    let provider = StripeProvider::new("mock_key".to_string(), "my_secret".to_string());
    let payload = b"dummy payload";
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "stripe-signature".to_string(),
        "t=123,v1=badhex".to_string(),
    );

    let err = provider.handle_webhook(payload, &headers).unwrap_err();
    assert!(err.contains("Invalid hex"));
}

#[tokio::test]
async fn test_lemonsqueezy_signature_verification_failure() {
    use rullst::capital::LemonSqueezyProvider;
    let provider = LemonSqueezyProvider::new("mock_key".to_string(), "my_secret".to_string());
    let payload = b"dummy payload";
    let mut headers = std::collections::HashMap::new();
    headers.insert("x-signature".to_string(), "badhex".to_string());

    let err = provider.handle_webhook(payload, &headers).unwrap_err();
    assert!(err.contains("Invalid hex"));
}
