use rullst_capital::dashboard::*;
use rullst_capital::invoice::*;
use rullst_capital::providers::*;
use std::collections::HashMap;

#[test]
fn test_revenue_metrics_and_dashboard() {
    let mgr = RevenueDashboardManager::new();
    let initial = mgr.get_metrics();
    assert_eq!(initial.mrr_cents, 0);
    assert_eq!(initial.arr_cents, 0);

    let event1 = WebhookEventRecord {
        id: "evt_sub_1".to_string(),
        provider: "stripe".to_string(),
        event_type: "customer.subscription.created".to_string(),
        status: "processed".to_string(),
        timestamp: 1700000000,
        payload_snippet: r#"{"type":"subscription_created"}"#.to_string(),
    };

    mgr.record_event(event1);
    mgr.update_metrics(RevenueMetrics {
        mrr_cents: 2900,
        arr_cents: 2900 * 12,
        net_revenue_cents: 2900,
        active_subscriptions: 1,
        churn_rate_percent: 0.0,
    });
    let after_sub = mgr.get_metrics();
    assert_eq!(after_sub.active_subscriptions, 1);
    assert_eq!(after_sub.mrr_cents, 2900);
    assert_eq!(after_sub.arr_cents, 2900 * 12);

    let recent = mgr.get_recent_events(5);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "evt_sub_1");
}

#[test]
fn test_subscription_status_parsing_and_conversion() {
    assert_eq!(
        SubscriptionStatus::parse_status("active"),
        SubscriptionStatus::Active
    );
    assert_eq!(
        SubscriptionStatus::parse_status("paid"),
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
        SubscriptionStatus::parse_status("unknown"),
        SubscriptionStatus::Unpaid
    );

    assert_eq!(SubscriptionStatus::Active.as_str(), "active");
    assert_eq!(SubscriptionStatus::Canceled.as_str(), "canceled");
    assert_eq!(SubscriptionStatus::PastDue.as_str(), "past_due");
    assert_eq!(SubscriptionStatus::Unpaid.as_str(), "unpaid");
    assert_eq!(SubscriptionStatus::Trialing.as_str(), "trialing");
    assert_eq!(SubscriptionStatus::Paused.as_str(), "paused");
}

#[test]
fn test_url_encode_helper() {
    assert_eq!(url_encode("hello world"), "hello%20world");
    assert_eq!(url_encode("foo@example.com"), "foo%40example.com");
    assert_eq!(url_encode("abc-123_.~"), "abc-123_.~");
}

#[test]
fn test_invoice_html_and_dps_generation() {
    let invoice = Invoice {
        invoice_id: "INV-2026-001".to_string(),
        customer_email: "alice@example.com".to_string(),
        date: chrono::Utc::now(),
        items: vec![
            InvoiceItem {
                description: "Pro Subscription".to_string(),
                amount: 29.00,
            },
            InvoiceItem {
                description: "Dedicated IP Addon".to_string(),
                amount: 10.00,
            },
        ],
        total: 39.00,
        currency: "USD".to_string(),
    };

    let html = invoice.generate_html();
    assert!(html.contains("Invoice INV-2026-001"));
    assert!(html.contains("alice@example.com"));
    assert!(html.contains("Pro Subscription"));
    assert!(html.contains("39.00 USD"));

    let dps = invoice.to_dps("01.07.01", "3550308", 0.05);
    assert_eq!(dps.amount, 39.00);
    assert_eq!(dps.service_code, "01.07.01");
}

#[tokio::test]
async fn test_provider_initialization_and_portals() {
    let stripe = StripeProvider::new("sk_test_123".to_string(), "whsec_test".to_string());
    assert_eq!(stripe.name(), "stripe");
    let headers = HashMap::new();
    assert!(stripe.handle_webhook(b"{}", &headers).is_err());

    let mp = MercadoPagoProvider::new("test_token".to_string(), "test_secret".to_string());
    assert_eq!(mp.name(), "mercadopago");
    let portal_mp = mp
        .create_customer_portal("alice@example.com", "http://return")
        .await
        .unwrap();
    assert!(portal_mp.contains("alice%40example.com"));
    assert!(mp.verify_signature(b"payload", "invalid_format").is_err());

    let ls = LemonSqueezyProvider::new("test_key".to_string(), "test_wh".to_string());
    assert_eq!(ls.name(), "lemonsqueezy");
    let portal_ls = ls
        .create_customer_portal("bob@example.com", "http://return")
        .await
        .unwrap();
    assert!(portal_ls.contains("bob%40example.com"));

    let uninteresting = serde_json::json!({
        "meta": { "event_name": "order_created" },
        "data": {}
    });
    let mut ls_headers = HashMap::new();
    ls_headers.insert("x-signature".to_string(), "invalid_hex".to_string());
    assert!(
        ls.handle_webhook(&serde_json::to_vec(&uninteresting).unwrap(), &ls_headers)
            .is_err()
    );

    let ip = InfinitePayProvider::new("test_ip".to_string(), "test_secret".to_string());
    assert_eq!(ip.name(), "infinitepay");
    let portal_ip = ip
        .create_customer_portal("carol@example.com", "http://return")
        .await
        .unwrap();
    assert!(portal_ip.contains("carol%40example.com"));

    let cb = CoinbaseCommerceProvider::new("test_cb".to_string(), "wh_secret".to_string());
    assert_eq!(cb.name(), "coinbase");
    let portal_cb = cb
        .create_customer_portal("dan@example.com", "http://return")
        .await
        .unwrap();
    assert!(portal_cb.contains("dan%40example.com"));
}
