// cargo-rullst/src/blueprints/saas/billing.rs — Stripe/Capital pricing & checkout views for SaaS blueprint.

const SAAS_STYLES: &str = include_str!("styles.css");

pub fn get_billing_pages() -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let pages_billing = r##"use rullst::html;
use rullst::response::Html;

fn pricing_navbar() -> String {
    html! {
        <nav class="pricing-nav" aria-label="Application links">
            <a href="/login" class="pricing-nav__link">"🔑 Login"</a>
            <div class="pricing-nav__stack">
                <a href="/nexus" class="pricing-nav__link pricing-nav__link--solid">"⚙️ Nexus CMS"</a>
                <span class="pricing-nav__note">"(local in debug; credentials in release)"</span>
            </div>
            <a href="http://127.0.0.1:5555" target="_blank" class="pricing-nav__link pricing-nav__link--solid">"📊 Studio (local)"</a>
        </nav>
    }
}

fn pricing_setup_banner() -> String {
    html! {
        <div class="setup-banner">
            <div class="setup-banner-icon">"🚀"</div>
            <div class="setup-banner-content">
                <h4>"Stripe Setup Required"</h4>
                <p>"To enable real checkouts, create a " <code>".env"</code> " file in your project root with your API keys:"</p>
                <pre><code>"BILLING_PROVIDER=stripe\nBILLING_API_KEY=sk_test_...\nBILLING_WEBHOOK_SECRET=whsec_..."</code></pre>
            </div>
        </div>
    }
}

fn pricing_header() -> String {
    html! {
        <div class="header">
            <span class="badge">"Rullst Capital"</span>
            <h1>"Simple, Transparent Pricing"</h1>
            <p class="subtitle">"Choose the perfect plan to boost your application with next-gen fullstack performance."</p>
        </div>
    }
}

fn pricing_plans() -> String {
    html! {
        <div class="pricing-grid">
            <div class="pricing-card">
                <h2 class="plan-name">"Starter"</h2>
                <p class="plan-desc">"For hobbyists and early-stage startup prototypes."</p>
                <div class="price-container">
                    <span class="currency">"$"</span>
                    <span class="price">"9"</span>
                    <span class="period">"/mo"</span>
                </div>
                <ul class="features-list">
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Up to 5 Projects"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Standard SQLite Database"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Email Support"</li>
                </ul>
                <a href="/billing/checkout?plan=price_starter" class="btn-checkout secondary">"Get Started"</a>
            </div>
            
            <div class="pricing-card premium">
                <h2 class="plan-name">"Pro"</h2>
                <p class="plan-desc">"For growing apps needing production scaling and support."</p>
                <div class="price-container">
                    <span class="currency">"$"</span>
                    <span class="price">"29"</span>
                    <span class="period">"/mo"</span>
                </div>
                <ul class="features-list">
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Unlimited Projects"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"PostgreSQL / Turso Sync"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Priority 24/7 Support"</li>
                </ul>
                <a href="/billing/checkout?plan=price_pro" class="btn-checkout primary">"Upgrade to Pro"</a>
            </div>
        </div>
    }
}

pub fn pricing_page() -> Html<String> {
    let has_keys = std::env::var("BILLING_API_KEY").map(|k| !k.is_empty() && k != "mock_key").unwrap_or(false);
    let banner_code = if !has_keys { pricing_setup_banner() } else { String::new() };

    let document = html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Select a Plan - Rullst Billing"</title>
                <link rel="icon" type="image/png" href="/static/rullst.png" />
                <link rel="stylesheet" href="/static/rullst.css" />
            </head>
            <body>
                <div class="glow-bg"></div>
                <div class="glow-bg-right"></div>
                <div class="container">
                    { rullst::html::RawHtml(pricing_navbar()) }
                    { rullst::html::RawHtml(banner_code) }
                    { rullst::html::RawHtml(pricing_header()) }
                    { rullst::html::RawHtml(pricing_plans()) }
                </div>
            </body>
        </html>
    };
    Html(format!("<!DOCTYPE html>{document}"))
}
"##;
    manifest.push(("src/pages/billing.rs", pages_billing.to_string()));
    manifest.push(("static/rullst.css", SAAS_STYLES.to_string()));

    let pages_mod = r##"pub mod auth;
pub mod billing;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
