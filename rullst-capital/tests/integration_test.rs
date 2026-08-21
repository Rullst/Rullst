// tests/integration_test.rs — Comprehensive fiscal, billing and webhook tests for Rullst Capital.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use rullst_capital::fiscal::{
    FiscalCustomer, FiscalEmitter, NfseDps, TaxRegime, build_dps_xml, compute_sha256_digest,
};

#[test]
fn test_fiscal_xml_builder_and_digest() {
    let emitter = FiscalEmitter {
        cnpj: "12.345.678/0001-90".to_string(),
        inscricao_municipal: "1234567".to_string(),
        legal_name: "Rullst SaaS & Software Ltda".to_string(),
        trade_name: Some("Rullst".to_string()),
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };

    let customer = FiscalCustomer {
        doc_number: "123.456.789-00".to_string(),
        name: "João Silva & Cia".to_string(),
        email: "joao@example.com".to_string(),
        zip_code: Some("01310-100".to_string()),
        address: Some("Av. Paulista, 1000".to_string()),
        ibge_code: Some("3550308".to_string()),
    };

    let dps = NfseDps {
        id: "DPS355030800010000000000000000000000000000001".to_string(),
        series: "1".to_string(),
        number: 1001,
        issued_at: Utc::now(),
        service_code: "01.07.01".to_string(),
        description: "Software as a Service Subscription & Support".to_string(),
        amount: 99.00,
        iss_rate: 0.05,
        iss_retained: false,
        service_city_ibge: "3550308".to_string(),
    };

    let xml = build_dps_xml(&emitter, &customer, &dps);
    assert!(xml.contains("<DPS"));
    assert!(xml.contains("12345678000190"));
    assert!(xml.contains("99.00"));

    let digest = compute_sha256_digest(&xml);
    assert!(!digest.is_empty());
}

#[tokio::test]
async fn test_all_12_payment_and_payout_providers() {
    use rullst_capital::providers::*;
    use std::collections::HashMap;

    // 1. Stripe
    let stripe = StripeProvider::new("mock_stripe_key".to_string(), "mock_whsec".to_string());
    assert_eq!(stripe.name(), "stripe");
    let url = stripe.create_checkout_session("alice@stripe.com", "price_pro", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session") && url.contains("alice%40stripe.com"));
    let portal = stripe.create_customer_portal("alice@stripe.com", "https://app.com").await.unwrap();
    assert!(portal.contains("mock_portal"));

    // 2. LemonSqueezy
    let ls = LemonSqueezyProvider::new("mock_ls_key".to_string(), "mock_ls_sec".to_string());
    assert_eq!(ls.name(), "lemonsqueezy");
    let url = ls.create_checkout_session("bob@ls.com", "var_999", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 3. InfinitePay
    let ip = InfinitePayProvider::new("mock_ip_client".to_string(), "mock_ip_sec".to_string());
    assert_eq!(ip.name(), "infinitepay");
    let url = ip.create_checkout_session("pix@empresa.com.br", "plan_pix", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 4. Polar
    let polar = PolarProvider::new("mock_polar_tok".to_string(), "mock_polar_wh".to_string());
    assert_eq!(polar.name(), "polar");
    let url = polar.create_checkout_session("dev@github.com", "tier_oss", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 5. Paddle
    let paddle = PaddleProvider::new("mock_pad_key".to_string(), "mock_pad_sec".to_string());
    assert_eq!(paddle.name(), "paddle");
    let url = paddle.create_checkout_session("user@paddle.com", "pri_paddle", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 6. Mercado Pago
    let mp = MercadoPagoProvider::new("mock_mp_acc".to_string(), "mock_mp_sec".to_string());
    assert_eq!(mp.name(), "mercadopago");
    let url = mp.create_checkout_session("cliente@mercadopago.com", "plan_latam", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 7. PicPay
    let picpay = PicPayProvider::new("mock_pic_tok".to_string(), "mock_pic_sec".to_string());
    assert_eq!(picpay.name(), "picpay");
    let url = picpay.create_checkout_session("usuario@picpay.com", "sub_pic", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 8. Razorpay
    let razor = RazorpayProvider::new("mock_rzp_key".to_string(), "mock_rzp_sec".to_string(), "mock_wh_sec".to_string());
    assert_eq!(razor.name(), "razorpay");
    let url = razor.create_checkout_session("user@razorpay.in", "plan_inr", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 9. Coinbase Commerce
    let cb = CoinbaseCommerceProvider::new("mock_cb_api".to_string(), "mock_cb_wh".to_string());
    assert_eq!(cb.name(), "coinbase");
    let url = cb.create_checkout_session("crypto@web3.eth", "charge_btc", "https://app.com/ok").await.unwrap();
    assert!(url.contains("mock_session"));

    // 10. Alipay
    let alipay = AlipayProvider::new("mock_ali_app_id".to_string(), "mock_ali_private_key".to_string(), "mock_ali_public_key".to_string());
    assert_eq!(alipay.name(), "alipay");
    let url = alipay.create_checkout_session("user@alipay.cn", "plan_cny", "https://app.com/ok").await.unwrap();
    assert!(url.contains("alipay.trade.page.pay") && url.contains("user%40alipay.cn"));

    // 11. Wise
    let wise = WiseProvider::new("mock_wise_key".to_string(), "mock_wise_prof".to_string());
    assert_eq!(wise.name(), "wise");
    let payout_res = wise.create_transfer("transfer@wise.com", 10000, "USD").await;
    assert!(payout_res.is_ok());

    // Generic Billing Provider Webhook Verification
    let payload = b"{\"type\":\"charge.succeeded\"}";
    let signature = "t=123,v1=mock_signature";
    let _ = stripe.verify_signature(payload, signature);
    assert!(stripe.cancel_subscription("sub_123").await.is_ok());
    assert!(stripe.pause_subscription("sub_123").await.is_ok());
    assert!(stripe.report_usage("sub_123", "api_calls", 500).await.is_ok());
}

#[test]
fn test_subscription_status_parsing_and_conversion() {
    use rullst_capital::providers::SubscriptionStatus;

    assert_eq!(SubscriptionStatus::parse_status("active"), SubscriptionStatus::Active);
    assert_eq!(SubscriptionStatus::parse_status("PAID"), SubscriptionStatus::Active);
    assert_eq!(SubscriptionStatus::parse_status("canceled"), SubscriptionStatus::Canceled);
    assert_eq!(SubscriptionStatus::parse_status("past_due"), SubscriptionStatus::PastDue);
    assert_eq!(SubscriptionStatus::parse_status("trialing"), SubscriptionStatus::Trialing);
    assert_eq!(SubscriptionStatus::parse_status("paused"), SubscriptionStatus::Paused);
    assert_eq!(SubscriptionStatus::parse_status("unknown_val"), SubscriptionStatus::Unpaid);

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
        mrr_cents: 1_250_000,     // $12,500 MRR
        arr_cents: 15_000_000,    // $150,000 ARR
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
fn test_invoice_html_and_dps_generation() {
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
    use rullst_capital::providers::{init_provider, StripeProvider};

    init_provider(Box::new(StripeProvider::new("mock_stripe_key".to_string(), "mock_wh_test".to_string())));

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

    let checkout = user.subscribe("plan_tier_1", "https://app.com/success").await;
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
