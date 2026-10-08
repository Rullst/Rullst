//! Route Handlers for Pricing, Monetization and Checkout Simulator in Blog Example.

use async_trait::async_trait;
use axum::extract::{Extension, Query};
use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use rullst_capital::billable::Billable;

use super::gateways::simulate_provider_checkout;
use super::views::render_pricing_page;
use crate::showcase_nav::{render_head_assets, render_showcase_nav};

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
pub async fn pricing_page(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
) -> impl IntoResponse {
    let nav = render_showcase_nav("/pricing");
    let head_assets = render_head_assets();

    let free_can_post = compute_demo_data();

    let body = render_pricing_page(nav, head_assets, free_can_post, csrf_token.as_str(), None);
    Html(body)
}

/// Handler for interactive Checkout generation (`/checkout` via GET).
pub async fn checkout_handler(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
    Query(params): Query<CheckoutParams>,
) -> impl IntoResponse {
    handle_checkout_submission(params, csrf_token.as_str().to_owned()).await
}

/// Handler for interactive Checkout generation (`/checkout` via GET).
pub async fn checkout_handler_get(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
    Query(params): Query<CheckoutParams>,
) -> impl IntoResponse {
    handle_checkout_submission(params, csrf_token.as_str().to_owned()).await
}

/// Handler for interactive Checkout generation (`/checkout` via POST).
pub async fn checkout_handler_post(
    Extension(csrf_token): Extension<rullst::security::CsrfToken>,
    axum::extract::Form(params): axum::extract::Form<CheckoutParams>,
) -> impl IntoResponse {
    handle_checkout_submission(params, csrf_token.as_str().to_owned()).await
}

async fn handle_checkout_submission(params: CheckoutParams, csrf_token: String) -> Html<String> {
    let nav = render_showcase_nav("/pricing");
    let head_assets = render_head_assets();

    let provider = params.provider.unwrap_or_else(|| "infinitepay".to_string());
    let plan = params.plan.unwrap_or_else(|| "pro_plan".to_string());
    let email = params
        .email
        .unwrap_or_else(|| "user@rullst.com".to_string());

    let free_can_post = compute_demo_data();

    let return_url = format!(
        "{}/pricing?status=success",
        crate::public_origin::fixture_origin()
    );
    let simulation_result = simulate_provider_checkout(&provider, &email, &plan, &return_url).await;

    let simulated = match simulation_result {
        Ok(url) => Some((provider, url)),
        Err(e) => Some((provider, format!("Error generating session: {}", e))),
    };

    let body = render_pricing_page(nav, head_assets, free_can_post, &csrf_token, simulated);
    Html(body)
}

/// Generates a real quota result for the community tier.
fn compute_demo_data() -> bool {
    let free_user = Subscriber {
        email_address: "author@community.dev".to_string(),
        plan_tier: "Community".to_string(),
        published_posts_count: 2,
    };
    free_user.check_quota("posts", free_user.published_posts_count as usize)
}
