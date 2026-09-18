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
                <h4>"Billing demonstration"</h4>
                <p>"These are example plans. Configure your products and prices before accepting payments."</p>
            </div>
        </div>
    }
}

fn pricing_header() -> String {
    html! {
        <div class="header">
            <span class="badge">"Rullst Capital"</span>
            <h1>"Example Billing Plans"</h1>
            <p class="subtitle">"Replace these sample products, prices, plan IDs and entitlements with values owned by your application and provider account."</p>
        </div>
    }
}

fn pricing_plans(csrf_token: &str) -> String {
    html! {
        <div class="pricing-grid">
            <div class="pricing-card">
                <h2 class="plan-name">"Example Starter"</h2>
                <p class="plan-desc">"Demonstrates a server-owned plan allowlist and provider checkout redirect."</p>
                <div class="price-container">
                    <span class="currency">"$"</span>
                    <span class="price">"9"</span>
                    <span class="period">"/mo"</span>
                </div>
                <ul class="features-list">
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Authenticated customer binding"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"CSRF-protected checkout command"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Deterministic offline provider fallback"</li>
                </ul>
                <form method="post" action="/billing/checkout">
                    <input type="hidden" name="_token" value={csrf_token} />
                    <input type="hidden" name="plan" value="price_starter" />
                    <button type="submit" class="btn-checkout secondary">"Test Starter Checkout"</button>
                </form>
            </div>
            
            <div class="pricing-card premium">
                <h2 class="plan-name">"Example Pro"</h2>
                <p class="plan-desc">"Demonstrates a second allowlisted provider product without promising application features."</p>
                <div class="price-container">
                    <span class="currency">"$"</span>
                    <span class="price">"29"</span>
                    <span class="period">"/mo"</span>
                </div>
                <ul class="features-list">
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Exact signed-webhook middleware"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Replay-aware subscription persistence"</li>
                    <li><svg aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>"Application-owned entitlement boundary"</li>
                </ul>
                <form method="post" action="/billing/checkout">
                    <input type="hidden" name="_token" value={csrf_token} />
                    <input type="hidden" name="plan" value="price_pro" />
                    <button type="submit" class="btn-checkout primary">"Test Pro Checkout"</button>
                </form>
            </div>
        </div>
    }
}

pub fn pricing_page(csrf_token: &str, _csp_nonce: &str) -> Html<String> {
    let banner_code = pricing_setup_banner();

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
                    { rullst::html::RawHtml(pricing_plans(csrf_token)) }
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
