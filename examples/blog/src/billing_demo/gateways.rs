//! Payment Gateways Catalog and Configuration Metadata for Rullst Capital.
//! Defines payment-adapter metadata with environment credential detection.

/// Metadata model for a supported Payment / Payout Gateway.
#[derive(Debug, Clone)]
pub struct GatewayInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub archetype: &'static str,
    pub archetype_badge_class: &'static str,
    pub flag: &'static str,
    pub current_boundary: &'static str,
    pub env_example: &'static str,
    pub rust_init_code: &'static str,
}

impl GatewayInfo {
    /// Returns true when an expected credential variable exists.
    ///
    /// Presence does not validate the credential or prove that every adapter capability is live.
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

    /// Returns a credential-presence or offline-demo status label and CSS badge.
    pub fn status_badge(&self) -> (&'static str, &'static str) {
        if self.is_configured() {
            ("🔐 Credentials Detected (Unverified)", "status-live")
        } else {
            ("🟡 Offline Demo", "status-mock")
        }
    }
}

/// Returns the 11 payment/payout adapters represented by offline fixtures.
pub fn all_gateways() -> Vec<GatewayInfo> {
    vec![
        GatewayInfo {
            id: "infinitepay",
            name: "InfinitePay",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-emerald",
            flag: "🇧🇷",
            current_boundary: "Deterministic offline checkout fixture. Live plan-only checkout is unsupported until an authoritative pricing contract is implemented.",
            env_example: "INFINITEPAY_API_KEY=\"inf_live_sec_...\"\nINFINITEPAY_WEBHOOK_SECRET=\"whsec_inf_...\"",
            rust_init_code: "use rullst_capital::{init_provider, InfinitePayProvider};\n\ninit_provider(Box::new(InfinitePayProvider::new(\n    std::env::var(\"INFINITEPAY_API_KEY\")?,\n    std::env::var(\"INFINITEPAY_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "stripe",
            name: "Stripe",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-blue",
            flag: "🌐",
            current_boundary: "Reviewed plan-based checkout adapter plus bounded immediate Payment Intent charge and documented webhook foundations; validate your live account and products.",
            env_example: "STRIPE_SECRET_KEY=\"sk_live_51...\"\nSTRIPE_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, StripeProvider};\n\ninit_provider(Box::new(StripeProvider::new(\n    std::env::var(\"STRIPE_SECRET_KEY\")?,\n    std::env::var(\"STRIPE_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "lemonsqueezy",
            name: "Lemon Squeezy",
            archetype: "Merchant-of-record adapter",
            archetype_badge_class: "badge-amber",
            flag: "🍋",
            current_boundary: "Plan-based checkout adapter and provider-specific usage-record contract with an explicit offline mock path; validate live account behavior and terms.",
            env_example: "LEMONSQUEEZY_API_KEY=\"lmsq_live_...\"\nLEMONSQUEEZY_STORE_ID=\"12345\"\nLEMONSQUEEZY_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, LemonSqueezyProvider};\n\ninit_provider(Box::new(LemonSqueezyProvider::new(\n    std::env::var(\"LEMONSQUEEZY_API_KEY\")?,\n    std::env::var(\"LEMONSQUEEZY_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "polar",
            name: "Polar.sh",
            archetype: "Merchant-of-record adapter",
            archetype_badge_class: "badge-cyan",
            flag: "⚡",
            current_boundary: "Plan-based checkout adapter and Standard Webhooks envelope verification foundation; validate live account behavior and provider coverage.",
            env_example: "POLAR_ACCESS_TOKEN=\"polar_at_...\"\nPOLAR_WEBHOOK_SECRET=\"polar_wh_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PolarProvider};\n\ninit_provider(Box::new(PolarProvider::new(\n    std::env::var(\"POLAR_ACCESS_TOKEN\")?,\n    std::env::var(\"POLAR_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "paddle",
            name: "Paddle",
            archetype: "Merchant-of-record adapter",
            archetype_badge_class: "badge-purple",
            flag: "🛡️",
            current_boundary: "Plan-based checkout adapter and signed-webhook foundation; validate the required live operations and provider contract.",
            env_example: "PADDLE_API_KEY=\"pdl_live_...\"\nPADDLE_WEBHOOK_SECRET=\"pdl_wh_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PaddleProvider};\n\ninit_provider(Box::new(PaddleProvider::new(\n    std::env::var(\"PADDLE_API_KEY\")?,\n    std::env::var(\"PADDLE_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "alipay",
            name: "Alipay (支付宝 / Alipay+)",
            archetype: "Cross-border adapter",
            archetype_badge_class: "badge-blue",
            flag: "🇨🇳",
            current_boundary: "Offline fixture only. Live RSA2 checkout and webhook verification fail closed until the asymmetric protocol is implemented and independently verified.",
            env_example: "ALIPAY_APP_ID=\"2021000123456789\"\nALIPAY_PRIVATE_KEY=\"MIIEvgIBADANBgkqhki...\"\nALIPAY_PUBLIC_KEY=\"MIIBIjANBgkqhki...\"",
            rust_init_code: "use rullst_capital::{init_provider, AlipayProvider};\n\ninit_provider(Box::new(AlipayProvider::new(\n    std::env::var(\"ALIPAY_APP_ID\")?,\n    std::env::var(\"ALIPAY_PRIVATE_KEY\")?,\n    std::env::var(\"ALIPAY_PUBLIC_KEY\")?,\n)));",
        },
        GatewayInfo {
            id: "mercadopago",
            name: "Mercado Pago",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-indigo",
            flag: "🌎",
            current_boundary: "Deterministic offline checkout fixture. Live plan-only checkout and body-only webhook verification are unavailable and fail closed.",
            env_example: "MERCADOPAGO_ACCESS_TOKEN=\"APP_USR-...\"\nMERCADOPAGO_WEBHOOK_SECRET=\"sec_mp_...\"",
            rust_init_code: "use rullst_capital::{init_provider, MercadoPagoProvider};\n\ninit_provider(Box::new(MercadoPagoProvider::new(\n    std::env::var(\"MERCADOPAGO_ACCESS_TOKEN\")?,\n    std::env::var(\"MERCADOPAGO_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "razorpay",
            name: "Razorpay",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-blue",
            flag: "🇮🇳",
            current_boundary: "Plan-based checkout adapter and signed-webhook foundation; unsupported event kinds fail closed. Validate required live account behavior.",
            env_example: "RAZORPAY_KEY_ID=\"rzp_live_...\"\nRAZORPAY_KEY_SECRET=\"sec_rzp_...\"\nRAZORPAY_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, RazorpayProvider};\n\ninit_provider(Box::new(RazorpayProvider::new(\n    std::env::var(\"RAZORPAY_KEY_ID\")?,\n    std::env::var(\"RAZORPAY_KEY_SECRET\")?,\n    std::env::var(\"RAZORPAY_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "coinbase",
            name: "Coinbase Commerce",
            archetype: "Commerce adapter",
            archetype_badge_class: "badge-amber",
            flag: "₿",
            current_boundary: "Deterministic offline checkout fixture plus a signed-webhook foundation. Live plan-only checkout and unsupported events fail closed.",
            env_example: "COINBASE_COMMERCE_API_KEY=\"cb_live_...\"\nCOINBASE_COMMERCE_WEBHOOK_SECRET=\"whsec_...\"",
            rust_init_code: "use rullst_capital::{init_provider, CoinbaseCommerceProvider};\n\ninit_provider(Box::new(CoinbaseCommerceProvider::new(\n    std::env::var(\"COINBASE_COMMERCE_API_KEY\")?,\n    std::env::var(\"COINBASE_COMMERCE_WEBHOOK_SECRET\")?,\n)));",
        },
        GatewayInfo {
            id: "picpay",
            name: "PicPay",
            archetype: "Billing adapter",
            archetype_badge_class: "badge-emerald",
            flag: "📱",
            current_boundary: "Deterministic offline checkout fixture. Live plan-only checkout is unsupported until an authoritative pricing contract is implemented.",
            env_example: "PICPAY_TOKEN=\"picpay_token_...\"\nPICPAY_SELLER_TOKEN=\"seller_token_...\"",
            rust_init_code: "use rullst_capital::{init_provider, PicPayProvider};\n\ninit_provider(Box::new(PicPayProvider::new(\n    std::env::var(\"PICPAY_TOKEN\")?,\n    std::env::var(\"PICPAY_SELLER_TOKEN\")?,\n)));",
        },
        GatewayInfo {
            id: "wise",
            name: "Wise (Transfers)",
            archetype: "Payout adapter",
            archetype_badge_class: "badge-cyan",
            flag: "💸",
            current_boundary: "Payout-adapter foundation, not a subscription checkout provider. Identity, compliance, currency, availability, and reconciliation remain external responsibilities.",
            env_example: "WISE_API_TOKEN=\"wise_api_tok_...\"\nWISE_PROFILE_ID=\"12345678\"",
            rust_init_code: "use rullst_capital::{init_payout_provider, WiseProvider};\n\ninit_payout_provider(Box::new(WiseProvider::new(\n    std::env::var(\"WISE_API_TOKEN\")?,\n    std::env::var(\"WISE_PROFILE_ID\")?,\n)));",
        },
    ]
}

/// Generates deterministic offline adapter output for the showcase.
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
                "mock_public".to_string(),
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
            "offline-fixture:wise?recipient={}&plan={}&amount=2900&currency=USD",
            rullst_capital::url_encode(customer_email),
            rullst_capital::url_encode(plan_id)
        )),
        _ => Err(rullst_capital::CapitalError::ConfigurationError(format!(
            "Unknown provider: {}",
            provider_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{all_gateways, simulate_provider_checkout};
    use std::collections::HashSet;

    #[tokio::test]
    async fn every_catalogue_action_returns_bounded_offline_output() {
        let gateways = all_gateways();
        assert_eq!(gateways.len(), 11);

        let unique_ids = gateways
            .iter()
            .map(|gateway| gateway.id)
            .collect::<HashSet<_>>();
        assert_eq!(unique_ids.len(), gateways.len());

        for gateway in gateways {
            assert!(!gateway.current_boundary.trim().is_empty());
            let output = simulate_provider_checkout(
                gateway.id,
                "fixture@example.invalid",
                "pro_plan",
                "https://showcase.example.invalid/pricing?status=success",
            )
            .await
            .unwrap_or_else(|error| panic!("{} offline fixture failed: {error}", gateway.id));
            assert!(
                !output.trim().is_empty(),
                "{} fixture was empty",
                gateway.id
            );
            assert!(
                output.len() <= 2_048,
                "{} fixture was unbounded",
                gateway.id
            );
        }
    }

    #[tokio::test]
    async fn unknown_catalogue_action_fails_explicitly() {
        let error = simulate_provider_checkout(
            "unknown",
            "fixture@example.invalid",
            "pro_plan",
            "https://showcase.example.invalid/pricing",
        )
        .await
        .expect_err("unknown provider must fail");
        assert!(error.to_string().contains("Unknown provider"));
    }
}
