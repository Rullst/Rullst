//! Payment Gateways Catalog and Configuration Metadata for Rullst Capital.
//! Defines all 11 supported financial providers with environment status detection.

/// Metadata model for a supported Payment / Payout Gateway.
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub archetype: &'static str,
    pub archetype_badge_class: &'static str,
    pub category: &'static str,
    pub region: &'static str,
    pub flag: &'static str,
    pub fees: &'static str,
    pub payout_speed: &'static str,
    pub tax_handling: &'static str,
    pub best_for: &'static str,
    pub env_example: &'static str,
    pub rust_init_code: &'static str,
    pub webhook_code: &'static str,
}

impl GatewayInfo {
    /// Returns true if live API credentials for this gateway are configured in the environment.
    pub fn is_configured(&self) -> bool {
        match self.id {
            "stripe" => std::env::var("STRIPE_SECRET_KEY").is_ok(),
            "lemonsqueezy" => std::env::var("LEMONSQUEEZY_API_KEY").is_ok(),
            "infinitepay" => std::env::var("INFINITEPAY_API_KEY").is_ok(),
            "polar" => std::env::var("POLAR_ACCESS_TOKEN").is_ok(),
            "paddle" => std::env::var("PADDLE_API_KEY").is_ok(),
            "alipay" => std::env::var("ALIPAY_APP_ID").is_ok(),
            "mercadopago" => std::env::var("MERCADOPAGO_ACCESS_TOKEN").is_ok(),
            "razorpay" => std::env::var("RAZORPAY_KEY_ID").is_ok(),
            "coinbase" => std::env::var("COINBASE_COMMERCE_API_KEY").is_ok(),
            "picpay" => std::env::var("PICPAY_TOKEN").is_ok(),
            "wise" => std::env::var("WISE_API_TOKEN").is_ok(),
            _ => false,
        }
    }

    /// Returns the live or mock status label and CSS badge.
    pub fn status_badge(&self) -> (&'static str, &'static str) {
        if self.is_configured() {
            ("🟢 Live Connected", "status-live")
        } else {
            ("🟡 Sandbox Mock Ready", "status-mock")
        }
    }
}

/// Returns the comprehensive list of all 11 supported gateways in Rullst Capital.
pub fn all_gateways() -> Vec<GatewayInfo> {
    vec![
        GatewayInfo {
            id: "infinitepay",
            name: "InfinitePay",
            symbol: "⚡",
            archetype: "Domestic Fee Leader (CloudWalk)",
            archetype_badge_class: "badge-emerald",
            category: "brazil",
            region: "Brazil (Domestic)",
            flag: "🇧🇷",
            fees: "Pix: 0.00% (Zero) | Card: 0.75% to 1.44%",
            payout_speed: "Instant (D+0 / D+1)",
            tax_handling: "Direct SPED NFS-e Integration",
            best_for: "Brazilian SaaS, optimized margins, zero-fee Pix, and credit card installments up to 12x.",
            env_example: "INFINITEPAY_API_KEY=\"inf_live_sec_...\"\nINFINITEPAY_WEBHOOK_SECRET=\"whsec_inf_...\"",
            rust_init_code: "use rullst_capital::{init_provider, InfinitePayProvider};\n\ninit_provider(Box::new(InfinitePayProvider::new(\n    std::env::var(\"INFINITEPAY_API_KEY\")?,\n    std::env::var(\"INFINITEPAY_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: x-infinitepay-signature\nVerification: HMAC-SHA256 (Constant-time comparison)",
        },
        GatewayInfo {
            id: "stripe",
            name: "Stripe",
            symbol: "🌐",
            archetype: "Global Direct Merchant",
            archetype_badge_class: "badge-blue",
            category: "direct",
            region: "Global (45+ countries)",
            flag: "🌐",
            fees: "~2.9% + $0.30 per transaction",
            payout_speed: "2 business days (D+2)",
            tax_handling: "Automatic Calculation (Stripe Tax)",
            best_for: "High-volume global SaaS, international cards, Apple Pay, Google Pay, and customer portals.",
            env_example: "STRIPE_SECRET_KEY=\"sk_live_51...\"\nSTRIPE_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, StripeProvider};\n\ninit_provider(Box::new(StripeProvider::new(\n    std::env::var(\"STRIPE_SECRET_KEY\")?,\n    std::env::var(\"STRIPE_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: Stripe-Signature (t=...,v1=...)\nVerification: HMAC-SHA256 with timestamp replay protection",
        },
        GatewayInfo {
            id: "lemonsqueezy",
            name: "Lemon Squeezy",
            symbol: "🍋",
            archetype: "Merchant of Record (MoR)",
            archetype_badge_class: "badge-amber",
            category: "mor",
            region: "Global (100+ countries)",
            flag: "🍋",
            fees: "~5.0% + $0.50 per transaction",
            payout_speed: "Weekly / Bi-weekly",
            tax_handling: "100% Automatic (MoR handles VAT & Sales Tax)",
            best_for: "Solo developers and startups selling globally without forming foreign legal entities.",
            env_example: "LEMONSQUEEZY_API_KEY=\"lmsq_live_...\"\nLEMONSQUEEZY_STORE_ID=\"12345\"\nLEMONSQUEEZY_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, LemonSqueezyProvider};\n\ninit_provider(Box::new(LemonSqueezyProvider::new(\n    std::env::var(\"LEMONSQUEEZY_API_KEY\")?,\n    std::env::var(\"LEMONSQUEEZY_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: X-Signature\nVerification: HMAC-SHA256 hex digest validation",
        },
        GatewayInfo {
            id: "polar",
            name: "Polar.sh",
            symbol: "⚡",
            archetype: "Developer-First MoR & Open Source",
            archetype_badge_class: "badge-cyan",
            category: "mor",
            region: "Global (Engineers & GitHub)",
            flag: "⚡",
            fees: "~4.0% + $0.40 per transaction",
            payout_speed: "Monthly / On-demand",
            tax_handling: "100% Automatic (Global MoR)",
            best_for: "Monetizing open-source repositories, software licenses, GitHub backers, and micro-SaaS.",
            env_example: "POLAR_ACCESS_TOKEN=\"polar_at_...\"\nPOLAR_WEBHOOK_SECRET=\"polar_wh_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PolarProvider};\n\ninit_provider(Box::new(PolarProvider::new(\n    std::env::var(\"POLAR_ACCESS_TOKEN\")?,\n    std::env::var(\"POLAR_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: webhook-signature\nVerification: Standard Webhook signature verification",
        },
        GatewayInfo {
            id: "paddle",
            name: "Paddle",
            symbol: "🛡️",
            archetype: "Enterprise Global MoR",
            archetype_badge_class: "badge-purple",
            category: "mor",
            region: "Global B2B & Enterprise",
            flag: "🛡️",
            fees: "~5.0% + $0.50 per transaction",
            payout_speed: "Monthly",
            tax_handling: "100% Automatic (Enterprise MoR)",
            best_for: "Enterprise B2B SaaS in the US and Europe with quote-to-cash invoicing workflows.",
            env_example: "PADDLE_API_KEY=\"pdl_live_...\"\nPADDLE_WEBHOOK_SECRET=\"pdl_wh_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PaddleProvider};\n\ninit_provider(Box::new(PaddleProvider::new(\n    std::env::var(\"PADDLE_API_KEY\")?,\n    std::env::var(\"PADDLE_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: Paddle-Signature (ts=...,h1=...)\nVerification: HMAC-SHA256 signature verification",
        },
        GatewayInfo {
            id: "alipay",
            name: "Alipay (支付宝 / Alipay+)",
            symbol: "🇨🇳",
            archetype: "China & APAC Cross-Border Leader",
            archetype_badge_class: "badge-blue",
            category: "apac",
            region: "China, HK & Asia-Pacific (> 1.3B users)",
            flag: "🇨🇳",
            fees: "~1.5% to 2.8% (domestic / cross-border)",
            payout_speed: "Instant / T+1",
            tax_handling: "Cross-Border Customs & VAT Compliance",
            best_for: "Selling to the Chinese consumer market, cross-border e-commerce, and Alipay+ Global wallets (GCash, Kakao Pay, DANA).",
            env_example: "ALIPAY_APP_ID=\"2021000123456789\"\nALIPAY_PRIVATE_KEY=\"MIIEvgIBADANBgkqhki...\"\nALIPAY_PUBLIC_KEY=\"MIIBIjANBgkqhki...\"",
            rust_init_code: "use rullst_capital::{init_provider, AlipayProvider};\n\ninit_provider(Box::new(AlipayProvider::new(\n    std::env::var(\"ALIPAY_APP_ID\")?,\n    std::env::var(\"ALIPAY_PRIVATE_KEY\")?,\n    std::env::var(\"ALIPAY_PUBLIC_KEY\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: sign / alipay-signature\nVerification: RSA2 (SHA256withRSA) / HMAC constant-time tag matching",
        },
        GatewayInfo {
            id: "mercadopago",
            name: "Mercado Pago",
            symbol: "🌎",
            archetype: "Latin America Regional Leader",
            archetype_badge_class: "badge-indigo",
            category: "latam",
            region: "Latin America (BR, AR, MX, CL, CO, UY)",
            flag: "🌎",
            fees: "~3.99% to 4.98% per transaction",
            payout_speed: "Instant",
            tax_handling: "Local Fiscal Compliance (NFS-e / SAT / AFIP)",
            best_for: "Complete regional coverage across Latin America (Pix, Boleto, local credit/debit cards).",
            env_example: "MERCADOPAGO_ACCESS_TOKEN=\"APP_USR-...\"\nMERCADOPAGO_WEBHOOK_SECRET=\"sec_mp_...\"",
            rust_init_code: "use rullst_capital::{init_provider, MercadoPagoProvider};\n\ninit_provider(Box::new(MercadoPagoProvider::new(\n    std::env::var(\"MERCADOPAGO_ACCESS_TOKEN\")?,\n    std::env::var(\"MERCADOPAGO_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: x-signature (ts=...,v1=...)\nVerification: HMAC-SHA256 signature matching",
        },
        GatewayInfo {
            id: "razorpay",
            name: "Razorpay",
            symbol: "🇮🇳",
            archetype: "India & Southeast Asia Regional Leader",
            archetype_badge_class: "badge-blue",
            category: "apac",
            region: "India & Southeast Asia",
            flag: "🇮🇳",
            fees: "~2.0% + GST per transaction",
            payout_speed: "2 to 3 business days (D+2)",
            tax_handling: "GST Compliance & Tax Invoicing",
            best_for: "Recurring UPI Autopay, Indian cards (RuPay/Visa/Mastercard), and NetBanking mandates.",
            env_example: "RAZORPAY_KEY_ID=\"rzp_live_...\"\nRAZORPAY_KEY_SECRET=\"sec_rzp_...\"\nRAZORPAY_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, RazorpayProvider};\n\ninit_provider(Box::new(RazorpayProvider::new(\n    std::env::var(\"RAZORPAY_KEY_ID\")?,\n    std::env::var(\"RAZORPAY_KEY_SECRET\")?,\n    std::env::var(\"RAZORPAY_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: X-Razorpay-Signature\nVerification: HMAC-SHA256 signature verification",
        },
        GatewayInfo {
            id: "coinbase",
            name: "Coinbase Commerce",
            symbol: "₿",
            archetype: "Global Web3 & Crypto",
            archetype_badge_class: "badge-amber",
            category: "crypto",
            region: "Global Decentralized (On-Chain)",
            flag: "₿",
            fees: "~1.0% per transaction",
            payout_speed: "Instant On-Chain",
            tax_handling: "Crypto Self-Custody / N/A",
            best_for: "Borderless global subscriptions in Bitcoin, Ethereum, Solana, USDC, and USDT with zero chargebacks.",
            env_example: "COINBASE_COMMERCE_API_KEY=\"cb_live_...\"\nCOINBASE_COMMERCE_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, CoinbaseCommerceProvider};\n\ninit_provider(Box::new(CoinbaseCommerceProvider::new(\n    std::env::var(\"COINBASE_COMMERCE_API_KEY\")?,\n    std::env::var(\"COINBASE_COMMERCE_WEBHOOK_SECRET\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: X-CC-Webhook-Signature\nVerification: HMAC-SHA256 signature verification",
        },
        GatewayInfo {
            id: "picpay",
            name: "PicPay",
            symbol: "📱",
            archetype: "Brazil Digital Wallet & QR Code",
            archetype_badge_class: "badge-emerald",
            category: "brazil",
            region: "Brazil (30M+ Users)",
            flag: "📱",
            fees: "~2.99% to 3.99% per transaction",
            payout_speed: "Instant",
            tax_handling: "Brazilian Domestic Fiscal Compliance",
            best_for: "Consumer mobile apps, direct PicPay digital wallet balance, and native QR Code checkout.",
            env_example: "PICPAY_TOKEN=\"picpay_token_...\"\nPICPAY_SELLER_TOKEN=\"seller_token_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PicPayProvider};\n\ninit_provider(Box::new(PicPayProvider::new(\n    std::env::var(\"PICPAY_TOKEN\")?,\n    std::env::var(\"PICPAY_SELLER_TOKEN\")?,\n)));",
            webhook_code: "POST /webhooks/capital\nHeader: x-seller-token\nVerification: Constant-time token matching",
        },
        GatewayInfo {
            id: "wise",
            name: "Wise (Transfers)",
            symbol: "💸",
            archetype: "Global Multi-Currency Payouts",
            archetype_badge_class: "badge-cyan",
            category: "payouts",
            region: "Global (40+ currencies in 160+ countries)",
            flag: "💸",
            fees: "Mid-market exchange rate + low fee (~0.4%)",
            payout_speed: "Instant / Same-day",
            tax_handling: "B2B Payouts & Disbursements",
            best_for: "Automated payouts and disbursements to international creators, affiliates, and remote contractors.",
            env_example: "WISE_API_TOKEN=\"wise_api_tok_...\"\nWISE_PROFILE_ID=\"12345678\"",
            rust_init_code: "use rullst_capital::{init_payout_provider, WiseProvider};\n\ninit_payout_provider(Box::new(WiseProvider::new(\n    std::env::var(\"WISE_API_TOKEN\")?,\n    std::env::var(\"WISE_PROFILE_ID\")?,\n)));",
            webhook_code: "POST /webhooks/payouts\nHeader: X-Signature-SHA256\nVerification: Payout event dispatch and ledger tracking",
        },
    ]
}

/// Generates a test checkout URL for any of the 11 supported providers.
pub async fn simulate_provider_checkout(
    provider_id: &str,
    customer_email: &str,
    plan_id: &str,
    redirect_url: &str,
) -> Result<String, rullst_capital::CapitalError> {
    use rullst_capital::providers::*;

    match provider_id {
        "stripe" => {
            let p = StripeProvider::new("mock_stripe_key".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "lemonsqueezy" => {
            let p = LemonSqueezyProvider::new("mock_lmsq".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "infinitepay" => {
            let p = InfinitePayProvider::new("mock_inf".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "polar" => {
            let p = PolarProvider::new("mock_polar".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "paddle" => {
            let p = PaddleProvider::new("mock_paddle".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "alipay" => {
            let p = AlipayProvider::new(
                "mock_alipay".to_string(),
                "mock_priv".to_string(),
                String::new(),
            );
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "mercadopago" => {
            let p = MercadoPagoProvider::new("mock_mp".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "razorpay" => {
            let p = RazorpayProvider::new(
                "mock_rzp".to_string(),
                "mock_sec".to_string(),
                String::new(),
            );
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "coinbase" => {
            let p = CoinbaseCommerceProvider::new("mock_cb".to_string(), String::new());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "picpay" => {
            let p = PicPayProvider::new("mock_pic".to_string(), "mock_seller".to_string());
            p.create_checkout_session(customer_email, plan_id, redirect_url)
                .await
        }
        "wise" => Ok(format!(
            "https://wise.com/pay/mock_transfer?recipient={}&plan={}&amount=2900&currency=USD",
            rullst_capital::url_encode(customer_email),
            rullst_capital::url_encode(plan_id)
        )),
        _ => Err(rullst_capital::CapitalError::ConfigurationError(format!(
            "Unknown provider: {}",
            provider_id
        ))),
    }
}
