#![cfg(feature = "capital")]

use rullst::capital::{BillingProvider, StripeProvider, SubscriptionStatus};

#[tokio::test]
async fn facade_exposes_provider_specific_metered_usage() {
    use rullst::capital::{
        MeteredBillingProvider as _, StripeMeterEvent, UsageDeduplication, UsageStatus,
    };

    let provider = StripeProvider::new("mock_usage", "mock_webhook");
    let event = StripeMeterEvent::new("cus_123", "lesson_minutes", 15, "usage-event-123")
        .expect("valid metered event");
    let receipt = provider
        .report_metered_usage(&event)
        .await
        .expect("deterministic mock usage receipt");

    assert_eq!(receipt.status(), UsageStatus::Mock);
    assert_eq!(receipt.deduplication(), UsageDeduplication::Mock);
    assert!(!receipt.is_live_accepted());
}
use std::string::ToString;

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
    assert!(!url.contains("test%40test.com"));
    assert!(url.contains("plan_123"));
}

#[tokio::test]
async fn test_stripe_provider_webhook_parsing() {
    let provider = StripeProvider::new("mock_key".to_string(), "mock_secret".to_string());

    let payload = serde_json::json!({
        "type": "customer.subscription.updated",
        "data": {
            "object": {
                "object": "subscription",
                "id": "sub_123",
                "customer": "cus_123",
                "status": "active",
                "current_period_end": 1700000000,
                "email": "test@test.com",
                "items": {
                    "has_more": false,
                    "data": [
                        { "price": { "id": "price_123" } }
                    ]
                }
            }
        }
    })
    .to_string();

    let mut headers = std::collections::HashMap::new();
    headers.insert("stripe-signature".to_string(), "mock_secret".to_string());
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
    let provider = StripeProvider::new("mock_key".to_string(), "mock_secret".to_string());
    let payload = serde_json::json!({
        "type": "invoice.paid"
    })
    .to_string();

    let mut headers = std::collections::HashMap::new();
    headers.insert("stripe-signature".to_string(), "mock_secret".to_string());
    let res = provider.handle_webhook(payload.as_bytes(), &headers);
    assert!(res.is_err());
    assert!(matches!(
        res,
        Err(rullst::capital::CapitalError::PayloadParseError(reason))
            if reason == "Stripe: unsupported subscription lifecycle event"
    ));
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
    assert!(
        err.to_string().contains("Invalid")
            || matches!(err, rullst::capital::CapitalError::InvalidSignature(_))
    );
}

#[tokio::test]
async fn test_infinitepay_provider_mock_checkout() {
    use rullst::capital::InfinitePayProvider;
    let provider = InfinitePayProvider::new("mock_key".to_string(), "mock_secret".to_string());
    assert_eq!(provider.name(), "infinitepay");

    let url = provider
        .create_checkout_session(
            "test@test.com",
            "plan_pro",
            "https://app.rullst.test/success",
        )
        .await
        .unwrap();
    assert!(url.starts_with("https://mock.infinitepay.invalid/"));
    assert!(!url.contains("test%40test.com"));
    assert!(url.contains("plan_pro"));
}

#[tokio::test]
async fn test_infinitepay_provider_mock_webhook_parsing() {
    use rullst::capital::InfinitePayProvider;
    let provider = InfinitePayProvider::new("mock_key".to_string(), "mock_secret".to_string());
    let payload = serde_json::json!({
        "id": "txn_456",
        "customer": { "id": "cus_999", "email": "cliente@test.com" },
        "plan_id": "plan_pro",
        "status": "past_due"
    })
    .to_string();

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-signature".to_string(), "mock_secret".to_string());
    let event = provider
        .handle_webhook(payload.as_bytes(), &headers)
        .unwrap();
    assert_eq!(event.subscription_id, "txn_456");
    assert_eq!(event.customer_id, "cus_999");
    assert_eq!(event.customer_email, "cliente@test.com");
    assert_eq!(event.plan_id, "plan_pro");
    assert_eq!(event.status, SubscriptionStatus::PastDue);
}

#[tokio::test]
async fn test_infinitepay_live_webhooks_fail_closed() {
    use rullst::capital::InfinitePayProvider;
    let provider = InfinitePayProvider::new("live_key".to_string(), "real_secret".to_string());
    let mut headers = std::collections::HashMap::new();
    headers.insert("x-signature".to_string(), "real_secret".to_string());

    let err = provider
        .handle_webhook(b"{\"status\":\"paid\"}", &headers)
        .unwrap_err();
    assert!(matches!(
        err,
        rullst::capital::CapitalError::UnsupportedOperation(_)
    ));
}
