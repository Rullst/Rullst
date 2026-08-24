use chrono::Utc;
use rullst_capital::fiscal::{
    FiscalCustomer, FiscalEmitter, NfseDps, TaxRegime, build_dps_xml, compute_sha256_digest,
};

#[test]
fn test_dps_xml_generation_and_hashing() {
    let emitter = FiscalEmitter {
        cnpj: "12345678000190".to_string(),
        legal_name: "Empresa Teste Ltda".to_string(),
        trade_name: Some("Teste Corp".to_string()),
        inscricao_municipal: "123456".to_string(),
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };

    let customer = FiscalCustomer {
        doc_number: "98765432000109".to_string(),
        name: "Tomador Servicos SA".to_string(),
        email: "tomador@teste.com".to_string(),
        zip_code: Some("01001000".to_string()),
        address: Some("Av Paulista, 1000".to_string()),
        ibge_code: Some("3550308".to_string()),
    };

    let dps = NfseDps {
        id: "DPS355030800010000000000000000000000000000001".to_string(),
        series: "1".to_string(),
        number: 1,
        issued_at: Utc::now(),
        service_code: "1.03.01".to_string(),
        description: "Desenvolvimento de software customizado".to_string(),
        amount: 99.00,
        iss_rate: 2.0,
        iss_retained: false,
        service_city_ibge: "3550308".to_string(),
    };

    let xml = build_dps_xml(&emitter, &customer, &dps);
    assert!(xml.contains("12345678000190"));
    assert!(xml.contains("98765432000109"));
    assert!(xml.contains("99.00"));

    let digest = compute_sha256_digest(&xml);
    assert!(!digest.is_empty());
}

#[tokio::test]
async fn test_all_12_payment_and_payout_providers() {
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
    assert!(url.contains("mock_session") && url.contains("alice%40stripe.com"));
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

    // 2. LemonSqueezy
    let ls = LemonSqueezyProvider::new("mock_ls_key".to_string(), "mock_ls_sec".to_string());
    assert_eq!(ls.name(), "lemonsqueezy");
    let url = ls
        .create_checkout_session("bob@ls.com", "var_999", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    let portal = ls
        .create_customer_portal("bob@ls.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("lemonsqueezy.com/my-orders"));
    assert!(ls.cancel_subscription("sub_ls").await.is_ok());
    assert!(ls.pause_subscription("sub_ls").await.is_ok());
    assert!(ls.report_usage("sub_ls", "seats", 5).await.is_ok());
    assert!(ls.apply_coupon("sub_ls", "PROMO").await.is_ok());
    assert!(ls.extend_trial("sub_ls", 1798761600).await.is_ok());
    let ls_payload = br#"{"meta":{"event_name":"subscription_created"},"data":{"id":"sub_ls_1","attributes":{"user_email":"bob@ls.com","variant_id":999,"status":"active","renews_at":"2026-12-31T00:00:00Z"}}}"#;
    assert!(ls.handle_webhook(ls_payload, &headers).is_err());

    // 3. InfinitePay
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

    // 4. Polar
    let polar = PolarProvider::new("mock_polar_tok".to_string(), "mock_polar_wh".to_string());
    assert_eq!(polar.name(), "polar");
    let url = polar
        .create_checkout_session("dev@github.com", "tier_oss", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    let portal = polar
        .create_customer_portal("dev@github.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("polar.sh/purchases"));
    assert!(polar.cancel_subscription("sub_pol").await.is_ok());
    assert!(polar.pause_subscription("sub_pol").await.is_ok());
    assert!(polar.report_usage("sub_pol", "events", 100).await.is_ok());
    assert!(polar.apply_coupon("sub_pol", "POLAR10").await.is_ok());
    assert!(polar.extend_trial("sub_pol", 1798761600).await.is_ok());
    let pol_payload = br#"{"type":"subscription.created","data":{"id":"sub_pol_1","user_id":"usr_1","user":{"email":"dev@github.com"},"product_id":"prod_1","status":"active","current_period_end":"2026-12-31T00:00:00Z"}}"#;
    assert!(polar.handle_webhook(pol_payload, &headers).is_err());

    // 5. Paddle
    let paddle = PaddleProvider::new("mock_pad_key".to_string(), "mock_pad_sec".to_string());
    assert_eq!(paddle.name(), "paddle");
    let url = paddle
        .create_checkout_session("user@paddle.com", "pri_paddle", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    let portal = paddle
        .create_customer_portal("user@paddle.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("paddle.com"));
    assert!(paddle.cancel_subscription("sub_pad").await.is_ok());
    assert!(paddle.pause_subscription("sub_pad").await.is_ok());
    assert!(paddle.report_usage("sub_pad", "gb", 10).await.is_ok());
    assert!(paddle.apply_coupon("sub_pad", "PADDLE10").await.is_ok());
    assert!(paddle.extend_trial("sub_pad", 1798761600).await.is_ok());
    let pad_payload = br#"{"event_type":"subscription.created","data":{"id":"sub_pad_1","customer_id":"ct_1","items":[{"price":{"id":"pri_1"}}],"status":"active","current_billing_period":{"ends_at":"2026-12-31T00:00:00Z"}}}"#;
    assert!(paddle.handle_webhook(pad_payload, &headers).is_err());

    // 6. Mercado Pago
    let mp = MercadoPagoProvider::new("mock_mp_acc".to_string(), "mock_mp_sec".to_string());
    assert_eq!(mp.name(), "mercadopago");
    let url = mp
        .create_checkout_session(
            "cliente@mercadopago.com",
            "plan_latam",
            "https://app.com/ok",
        )
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    let portal = mp
        .create_customer_portal("cliente@mercadopago.com", "https://app.com")
        .await
        .unwrap();
    assert!(portal.contains("mercadopago.com/subscriptions"));
    assert!(mp.cancel_subscription("sub_mp").await.is_ok());
    assert!(mp.pause_subscription("sub_mp").await.is_ok());
    assert!(mp.report_usage("sub_mp", "vendas", 10).await.is_ok());
    assert!(mp.apply_coupon("sub_mp", "DESCONTO").await.is_ok());
    assert!(mp.extend_trial("sub_mp", 1798761600).await.is_ok());
    let mp_payload = br#"{"data":{"id":"sub_mp_1","payer_id":"pay_1","email":"cliente@mercadopago.com","plan_id":"plan_latam","status":"approved","next_payment_date":"2026-12-31T00:00:00Z"}}"#;
    assert!(mp.handle_webhook(mp_payload, &headers).is_err());

    // 7. PicPay
    let picpay = PicPayProvider::new("mock_pic_tok".to_string(), "mock_pic_sec".to_string());
    assert_eq!(picpay.name(), "picpay");
    let url = picpay
        .create_checkout_session("usuario@picpay.com", "sub_pic", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(
        picpay
            .create_customer_portal("usuario@picpay.com", "https://app.com")
            .await
            .is_ok()
    );
    assert!(picpay.cancel_subscription("sub_pic").await.is_ok());
    assert!(picpay.pause_subscription("sub_pic").await.is_err());
    assert!(
        picpay
            .report_usage("sub_pic", "transacoes", 1)
            .await
            .is_err()
    );
    assert!(picpay.apply_coupon("sub_pic", "PICPAY5").await.is_err());
    assert!(picpay.extend_trial("sub_pic", 1798761600).await.is_err());
    let pic_payload = br#"{"referenceId":"sub_pic_1","status":"paid","authorizationId":"auth_1"}"#;
    assert!(picpay.handle_webhook(pic_payload, &headers).is_err());

    // 8. Razorpay
    let razor = RazorpayProvider::new(
        "mock_rzp_key".to_string(),
        "mock_rzp_sec".to_string(),
        "mock_wh_sec".to_string(),
    );
    assert_eq!(razor.name(), "razorpay");
    let url = razor
        .create_checkout_session("user@razorpay.in", "plan_inr", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(
        razor
            .create_customer_portal("user@razorpay.in", "https://app.com")
            .await
            .is_ok()
    );
    assert!(razor.cancel_subscription("sub_rzp").await.is_ok());
    assert!(razor.pause_subscription("sub_rzp").await.is_ok());
    assert!(razor.report_usage("sub_rzp", "api", 100).await.is_ok());
    assert!(razor.apply_coupon("sub_rzp", "RZP10").await.is_ok());
    assert!(razor.extend_trial("sub_rzp", 1798761600).await.is_ok());
    let rzp_payload = br#"{"event":"subscription.charged","payload":{"subscription":{"entity":{"id":"sub_rzp_1","plan_id":"plan_inr","status":"active","current_end":1798761600}},"payment":{"entity":{"customer_id":"cust_1","email":"user@razorpay.in"}}}}"#;
    assert!(razor.handle_webhook(rzp_payload, &headers).is_err());

    // 9. Coinbase Commerce
    let cb = CoinbaseCommerceProvider::new("mock_cb_api".to_string(), "mock_cb_wh".to_string());
    assert_eq!(cb.name(), "coinbase");
    let url = cb
        .create_checkout_session("crypto@web3.eth", "charge_btc", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock_session"));
    assert!(
        cb.create_customer_portal("crypto@web3.eth", "https://app.com")
            .await
            .is_ok()
    );
    assert!(cb.cancel_subscription("charge_btc").await.is_ok());
    assert!(cb.pause_subscription("charge_btc").await.is_err());
    let cb_payload = br#"{"event":{"id":"evt_1","type":"charge:confirmed","data":{"id":"ch_1","pricing":{"local":{"amount":"10.00"}}}}}"#;
    assert!(cb.handle_webhook(cb_payload, &headers).is_err());

    // 10. Alipay
    let alipay = AlipayProvider::new(
        "mock_ali_app_id".to_string(),
        "mock_ali_private_key".to_string(),
        "mock_ali_public_key".to_string(),
    );
    assert_eq!(alipay.name(), "alipay");
    let url = alipay
        .create_checkout_session("user@alipay.cn", "plan_cny", "https://app.com/ok")
        .await
        .unwrap();
    assert!(url.contains("mock.alipay.invalid") && url.contains("user%40alipay.cn"));
    assert!(
        alipay
            .create_customer_portal("user@alipay.cn", "https://app.com")
            .await
            .is_ok()
    );
    assert!(alipay.cancel_subscription("sub_ali").await.is_ok());
    assert!(alipay.pause_subscription("sub_ali").await.is_err());
    let ali_payload = br#"{"trade_status":"TRADE_SUCCESS","out_trade_no":"order_123"}"#;
    assert!(alipay.handle_webhook(ali_payload, &headers).is_err());

    // 11. Wise (Payout Provider)
    let wise = WiseProvider::new("mock_wise_key".to_string(), "mock_wise_prof".to_string());
    assert_eq!(wise.name(), "wise");
    let payout_res = wise
        .create_transfer("transfer@wise.com", 10000, "USD")
        .await;
    assert!(payout_res.is_ok());
    let status = wise.get_transfer_status("transfer_123").await;
    assert!(status.is_ok());
    let wise_payload = br#"{"data":{"resource":{"id":12345,"recipient_email":"transfer@wise.com","amount":100.0,"currency":"USD"},"current_state":"outgoing_payment_sent"}}"#;
    let _ = wise.parse_webhook_payload(wise_payload);
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
