//! Route Handlers for Pricing, Monetization and Checkout Simulator in Blog Example.

use async_trait::async_trait;
use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use chrono::Utc;
use serde::Deserialize;

use rullst_capital::billable::Billable;
use rullst_capital::fiscal::dps::build_dps_xml;
use rullst_capital::fiscal::models::{FiscalCustomer, FiscalEmitter, NfseDps, TaxRegime};

use super::gateways::simulate_provider_checkout;
use super::views::render_pricing_page;
use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

/// Example SaaS Subscriber implementing the `Billable` trait.
pub struct Subscriber {
    pub email_address: String,
    pub plan_tier: String,
    pub published_posts_count: u32,
}

#[async_trait]
impl Billable for Subscriber {
    fn email(&self) -> String {
        self.email_address.clone()
    }

    fn tier(&self) -> Option<String> {
        Some(self.plan_tier.clone())
    }

    fn tier_limit(&self, _feature: &str) -> Option<usize> {
        match self.plan_tier.as_str() {
            "Enterprise" => Some(10_000),
            "Pro" => Some(50),
            _ => Some(3), // Community Free Tier limit: 3 posts
        }
    }
}

/// Query parameters for testing checkout generation.
#[derive(Debug, Deserialize)]
pub struct CheckoutParams {
    pub provider: Option<String>,
    pub plan: Option<String>,
    pub email: Option<String>,
}

/// Handler for the Pricing & Monetization showcase route (`/pricing` and `/billing`).
pub async fn pricing_page() -> impl IntoResponse {
    let nav = render_showcase_nav("/pricing");
    let styles = render_shared_styles();

    let (free_can_post, xml_snippet) = compute_demo_data();

    let body = render_pricing_page(nav, styles, free_can_post, xml_snippet, None);
    Html(body)
}

/// Handler for interactive Checkout generation (`/checkout` via GET).
pub async fn checkout_handler(Query(params): Query<CheckoutParams>) -> impl IntoResponse {
    handle_checkout_submission(params).await
}

/// Handler for interactive Checkout generation (`/checkout` via GET).
pub async fn checkout_handler_get(Query(params): Query<CheckoutParams>) -> impl IntoResponse {
    handle_checkout_submission(params).await
}

/// Handler for interactive Checkout generation (`/checkout` via POST).
pub async fn checkout_handler_post(
    axum::extract::Form(params): axum::extract::Form<CheckoutParams>,
) -> impl IntoResponse {
    handle_checkout_submission(params).await
}

async fn handle_checkout_submission(params: CheckoutParams) -> impl IntoResponse {
    let nav = render_showcase_nav("/pricing");
    let styles = render_shared_styles();

    let provider = params.provider.unwrap_or_else(|| "infinitepay".to_string());
    let plan = params.plan.unwrap_or_else(|| "pro_plan".to_string());
    let email = params
        .email
        .unwrap_or_else(|| "user@rullst.com".to_string());

    let (free_can_post, xml_snippet) = compute_demo_data();

    let simulation_result = simulate_provider_checkout(
        &provider,
        &email,
        &plan,
        "http://localhost:3000/pricing?status=success",
    )
    .await;

    let simulated = match simulation_result {
        Ok(url) => Some((provider, url)),
        Err(e) => Some((provider, format!("Error generating session: {}", e))),
    };

    let body = render_pricing_page(nav, styles, free_can_post, xml_snippet, simulated);
    Html(body)
}

/// Generates a real quota result and a clearly labelled offline DPS preview.
fn compute_demo_data() -> (bool, String) {
    let free_user = Subscriber {
        email_address: "author@community.dev".to_string(),
        plan_tier: "Community".to_string(),
        published_posts_count: 2,
    };
    let free_can_post = free_user.check_quota("posts", free_user.published_posts_count as usize);

    let emitter = FiscalEmitter {
        cnpj: "12345678000190".to_string(),
        inscricao_municipal: "12345".to_string(),
        legal_name: "Rullst SaaS Publisher Inc".to_string(),
        trade_name: Some("Rullst Publisher".to_string()),
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };

    let customer = FiscalCustomer {
        doc_number: "98765432000188".to_string(),
        name: "Acme Corp Brazil".to_string(),
        email: "billing@acme.com.br".to_string(),
        zip_code: Some("01310-100".to_string()),
        address: Some("Av Paulista, 1000".to_string()),
        ibge_code: Some("3550308".to_string()),
    };

    let dps = NfseDps {
        id: "DPS355030800010000000000000000000000000000001".to_string(),
        series: "1".to_string(),
        number: 1042,
        issued_at: Utc::now(),
        service_code: "1.03.01".to_string(),
        description: "Rullst Sovereign Publisher - Enterprise Plan Subscription".to_string(),
        amount: 499.00,
        iss_rate: 2.0,
        iss_retained: false,
        service_city_ibge: "3550308".to_string(),
    };

    let unsigned_xml = build_dps_xml(&emitter, &customer, &dps);
    let mut preview: String = unsigned_xml.chars().take(250).collect();
    if unsigned_xml.chars().count() > 250 {
        preview.push_str("...");
    }
    preview.push_str(" [OFFLINE DPS PREVIEW — NOT AUTHORIZED OR SIGNED]");

    (free_can_post, preview)
}
