#[tokio::test]
async fn test_kept_payment_providers_use_offline_fixtures() {
    use rullst_capital::providers::*;
    use std::collections::HashMap;

    let headers = HashMap::new();

    // 1. Stripe
    let stripe = StripeProvider::new("mock_stripe_key".to_string(), "mock_whsec".to_string());
    assert_eq!(stripe.name(), "stripe");
    let url = stripe
        .create_checkout_session("alice@stripe.com", "price_pro", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.starts_with("https://mock.stripe.invalid/checkout/mock_session?plan=price_pro"));
    assert!(!url.contains("alice"));
    let portal = stripe
        .create_customer_portal("alice@stripe.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("mock_portal"));
    assert!(stripe.cancel_subscription("sub_123").await.is_ok());
    assert!(stripe.pause_subscription("sub_123").await.is_ok());
    assert!(
        stripe
            .report_usage("sub_123", "api_calls", 500)
            .await
            .is_ok()
    );
    assert!(stripe.apply_coupon("sub_123", "SAVE20").await.is_ok());
    assert!(stripe.extend_trial("sub_123", 1798761600).await.is_ok());

    // 2. InfinitePay (experimental)
    let ip = InfinitePayProvider::new("mock_ip_client".to_string(), "mock_ip_sec".to_string());
    assert_eq!(ip.name(), "infinitepay");
    let url = ip
        .create_checkout_session("pix@empresa.com.br", "plan_pix", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(
        ip.create_customer_portal("pix@empresa.com.br", "https://app.com")
            .await
            .is_ok()
    );
    assert!(ip.cancel_subscription("sub_ip").await.is_ok());
    let ip_payload = br#"{"event":"charge.paid","data":{"id":"sub_ip_1","customer":{"email":"pix@empresa.com.br"},"status":"paid"}}"#;
    assert!(ip.handle_webhook(ip_payload, &headers).is_err());
}

#[test]
fn test_subscription_status_parsing_and_conversion() {
    use rullst_capital::providers::SubscriptionStatus;

    assert_eq!(
        SubscriptionStatus::parse_status("active"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("PAID"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("canceled"),
        SubscriptionStatus::Canceled
    );
    assert_eq!(
        SubscriptionStatus::parse_status("past_due"),
        SubscriptionStatus::PastDue
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
        SubscriptionStatus::parse_status("unknown_val"),
        SubscriptionStatus::Unpaid
    );

    assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
    assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
    assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
    assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");
}

#[test]
fn test_revenue_metrics_and_dashboard() {
    use rullst_capital::dashboard::{RevenueDashboardManager, RevenueMetrics, WebhookEventRecord};

    let mgr = RevenueDashboardManager::new();

    let initial = mgr.get_metrics();
    assert_eq!(initial.mrr_cents, 0);

    let updated = RevenueMetrics {
        mrr_cents: 1_250_000,  // $12,500 MRR
        arr_cents: 15_000_000, // $150,000 ARR
        net_revenue_cents: 120_000_000,
        active_subscriptions: 142,
        churn_rate_percent: 1.8,
    };
    mgr.update_metrics(updated.clone());

    let curr = mgr.get_metrics();
    assert_eq!(curr.mrr_cents, 1_250_000);
    assert_eq!(curr.active_subscriptions, 142);
    assert_eq!(curr.churn_rate_percent, 1.8);

    let record = WebhookEventRecord {
        id: "evt_1001".to_string(),
        provider: "stripe".to_string(),
        event_type: "invoice.payment_succeeded".to_string(),
        status: "processed".to_string(),
        timestamp: 1724170000,
        payload_snippet: "{\"amount\": 9900}".to_string(),
    };
    mgr.record_event(record);

    let events = mgr.get_recent_events(10);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, "evt_1001");
}

#[test]
fn test_invoice_html_generation() {
    use chrono::Utc;
    use rullst_capital::invoice::{Invoice, InvoiceItem};

    let invoice = Invoice {
        invoice_id: "INV-2026-0042".to_string(),
        customer_email: "billing@client.com".to_string(),
        date: Utc::now(),
        items: vec![
            InvoiceItem {
                description: "Rullst Enterprise License (Monthly)".to_string(),
                amount: 499.00,
            },
            InvoiceItem {
                description: "Priority 24/7 SLA Support".to_string(),
                amount: 199.00,
            },
        ],
        total: 698.00,
        currency: "USD".to_string(),
    };

    let html = invoice.generate_html();
    assert!(html.contains("INV-2026-0042"));
    assert!(html.contains("billing@client.com"));
    assert!(html.contains("Rullst Enterprise License"));
    assert!(html.contains("698.00 USD") || html.contains("698.00"));
}

#[tokio::test]
async fn test_billable_trait_facade() {
    use async_trait::async_trait;
    use rullst_capital::billable::Billable;
    use rullst_capital::providers::{StripeProvider, init_provider};

    init_provider(Box::new(StripeProvider::new(
        "mock_stripe_key".to_string(),
        "mock_wh_test".to_string(),
    )));

    struct User {
        email_addr: String,
        sub: Option<String>,
    }

    #[async_trait]
    impl Billable for User {
        fn email(&self) -> String {
            self.email_addr.clone()
        }
        fn subscription_id(&self) -> Option<String> {
            self.sub.clone()
        }
    }

    let user = User {
        email_addr: "subscriber@company.com".to_string(),
        sub: Some("sub_active_888".to_string()),
    };

    let checkout = user
        .subscribe("plan_tier_1", "https://app.com/success")
        .await;
    assert!(checkout.is_ok());

    let portal = user.billing_portal_url("https://app.com/account").await;
    assert!(portal.is_ok());

    let cancel = user.cancel_subscription().await;
    assert!(cancel.is_ok());

    let pause = user.pause_subscription().await;
    assert!(pause.is_ok());

    let usage = user.report_usage("tokens", 1000).await;
    assert!(usage.is_ok());
}
