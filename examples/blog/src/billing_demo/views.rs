//! HTML UI and Glassmorphic Components for Capital Pricing & Gateway Showcase.

use super::gateways::{GatewayInfo, all_gateways};
use rullst::html;

/// Renders the complete HTML Pricing and Gateway Showcase page.
pub fn render_pricing_page(
    nav: String,
    head_assets: String,
    free_can_post: bool,
    csrf_token: &str,
    simulated_checkout_url: Option<(String, String)>, // (provider_id, url)
) -> String {
    let gateways = all_gateways();
    let configured_count = gateways.iter().filter(|g| g.is_configured()).count();
    let total_count = gateways.len();

    let gateways_cards_html = render_gateway_cards(&gateways);
    let config_accordions_html = render_config_guide(&gateways);
    let checkout_result_html = render_checkout_result(simulated_checkout_url);

    html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst Capital — Offline Adapter Capability Showcase"</title>
                { rullst::html::RawHtml(head_assets) }
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">

                    { rullst::html::RawHtml(checkout_result_html) }

                    <div class="card pricing-hero">
                        <div class="card-header wrap">
                            <div>
                                <h1 class="card-title compact">
                                    "SaaS Pricing & Quota Governance"
                                    <span class="feature-tag tag-cap">"rullst-capital"</span>
                                </h1>
                                <p class="lead flush hero-copy">
                                    "Billing demonstrations in Rust: tier quotas with the " <code>"Billable"</code> " trait, verified webhook primitives, and an offline catalogue of payment-provider adapters. Capabilities vary by adapter and live credentials are never used by this page."
                                </p>
                            </div>
                        </div>

                        <div class="hero-stats">
                            <span class="stat-badge live">
                                "🔐 " <strong>{format!("{} / {} Credential Sets Detected", configured_count, total_count)}</strong>
                            </span>
                            <span class="stat-badge">
                                "🧪 Every action on this page stays offline"
                            </span>
                            <span class="stat-badge">
                                "🇧🇷 InfinitePay is experimental until validated live"
                            </span>
                            <span class="stat-badge">
                                "🧾 Provider tax features vary by contract"
                            </span>
                        </div>

                        <div class="tier-grid">
                            <div class="tier">
                                <div>
                                    <h3 class="tier-name">"Community Free"</h3>
                                    <div class="tier-price">"$0" <span class="tier-period">"/mo"</span></div>
                                    <ul class="tier-features">
                                        <li>"Up to 3 Published Stories"</li>
                                        <li>"Typed SSR UI with optional HTMX behavior"</li>
                                        <li>"Community Support & Forum"</li>
                                    </ul>
                                </div>
                                <div class="quota-check">
                                    {if free_can_post { "✅ Quota Check: Allowed (2/3)" } else { "❌ Quota Reached" }}
                                </div>
                            </div>

                            <div class="tier tier-featured">
                                <div class="tier-ribbon">"MOST POPULAR"</div>
                                <div>
                                    <h3 class="tier-name pro">"Pro Author"</h3>
                                    <div class="tier-price">"$29" <span class="tier-period">"/mo"</span></div>
                                    <ul class="tier-features">
                                        <li>"Up to 50 Published Stories"</li>
                                        <li>"LiveView Real-time Comments"</li>
                                        <li>"AI Assistant & Semantic RAG"</li>
                                    </ul>
                                </div>
                                <a href="#checkout-simulator" class="btn btn-block">"Run an Offline Fixture (2 Adapters)"</a>
                            </div>

                            <div class="tier">
                                <div>
                                    <h3 class="tier-name enterprise">"Enterprise"</h3>
                                    <div class="tier-price">"$99" <span class="tier-period">"/mo"</span></div>
                                    <ul class="tier-features">
                                        <li>"Unlimited Stories & Multi-tenant"</li>
                                        <li>"Full Studio & Nexus CMS Control Room"</li>
                                        <li>"Bring-your-own gateway adapter contracts"</li>
                                    </ul>
                                </div>
                                <a href="#checkout-simulator" class="btn btn-emerald btn-block">"Inspect Adapter Boundaries"</a>
                            </div>
                        </div>
                    </div>

                    <div id="checkout-simulator" class="card checkout-box">
                        <h2 class="card-title compact accent-sky">
                            "🧪 Offline Checkout Fixture Explorer"
                        </h2>
                        <p class="checkout-intro">
                            "Select an adapter to exercise deterministic mock behavior through " <code>"rullst-capital"</code> ". This page does not contact a live payment service:"
                        </p>

                        <form method="POST" action="/checkout" class="checkout-form">
                            <input type="hidden" name="_token" value={csrf_token} />
                            <div>
                                <label for="checkout-provider" class="field-label">"Payment Adapter Fixture:"</label>
                                <select id="checkout-provider" name="provider" class="field-control">
                                    <option value="infinitepay">"🇧🇷 InfinitePay (experimental offline fixture)"</option>
                                    <option value="stripe">"🌐 Stripe (offline billing fixture)"</option>
                                </select>
                            </div>

                            <div>
                                <label for="checkout-plan" class="field-label">"SaaS Plan:"</label>
                                <select id="checkout-plan" name="plan" class="field-control">
                                    <option value="pro_plan">"Pro Author ($29/mo)"</option>
                                    <option value="enterprise_plan">"Enterprise ($99/mo)"</option>
                                </select>
                            </div>

                            <div>
                                <label for="checkout-email" class="field-label">"Subscriber Email:"</label>
                                <input id="checkout-email" type="email" name="email" value="customer@rullst.com" class="field-control" required="true" />
                            </div>

                            <div>
                                <button type="submit" class="btn">
                                    "Generate Offline Fixture ➔"
                                </button>
                            </div>
                        </form>

                    </div>

                    <div class="card card-spaced">
                        <div class="card-header wrap centered">
                            <div>
                                <h2 class="card-title flush">
                                    "💳 Payment Adapter Catalogue in Rullst Capital"
                                </h2>
                                <p class="catalogue-copy">
                                    "Strongly typed adapters with deterministic offline credentials. Each card states the reviewed v12 boundary; a fixture is not evidence of live provider acceptance."
                                </p>
                            </div>
                            <div>
                                <a href="https://github.com/Rullst/Rullst/blob/main/docs/src/payment-gateways-guide.md" target="_blank" rel="noopener noreferrer" class="btn btn-small">
                                    "📖 Open Full Architecture Guide"
                                </a>
                            </div>
                        </div>

                        <div class="gateway-grid">
                            { rullst::html::RawHtml(gateways_cards_html) }
                        </div>
                    </div>

                    <div class="card card-spaced">
                        <h2 class="card-title compact accent-violet">
                            "⚙️ Adapter Initialization Reference"
                        </h2>
                        <p class="muted small-text">
                            "Open a provider to inspect illustrative environment names, Rust initialization, and the reviewed v12 capability boundary. Consult the provider and Rullst documentation before enabling a live account:"
                        </p>

                        <div class="config-accordion">
                            { rullst::html::RawHtml(config_accordions_html) }
                        </div>
                    </div>

                </div>
            </body>
        </html>
    }
}

/// Renders every gateway card.
fn render_gateway_cards(gateways: &[GatewayInfo]) -> String {
    gateways
        .iter()
        .map(|g| {
            let (status_text, status_class) = g.status_badge();
            let config_id = format!("config-{}", g.id);

            html! {
                <div class="gateway-card">
                    <div>
                        <div class="gateway-header">
                            <div>
                                <h3 class="gateway-title">
                                    <span>{g.flag}</span>
                                    <span>{g.name}</span>
                                </h3>
                                <span class={format!("gateway-archetype {}", g.archetype_badge_class)}>
                                    {g.archetype}
                                </span>
                            </div>
                            <span class={format!("stat-badge compact {}", status_class)}>
                                {status_text}
                            </span>
                        </div>

                        <p class="gateway-boundary">
                            {g.current_boundary}
                        </p>

                        <div class="gateway-specs">
                            <div class="spec-item">
                                <span class="spec-label">"External terms:"</span>
                                <span class="spec-val">"Consult the provider's current pricing, availability, settlement, tax, and account documentation."</span>
                            </div>
                        </div>
                    </div>

                    <div class="gateway-actions">
                        <a href={format!("/checkout?provider={}&plan=pro_plan", g.id)} class="btn">
                            "Run Offline Fixture"
                        </a>
                        <a href={format!("#{}", config_id)} class="btn">
                            "View Setup Guide"
                        </a>
                    </div>
                </div>
            }
        })
        .collect()
}

/// Renders the accordion configuration guides for each gateway.
fn render_config_guide(gateways: &[GatewayInfo]) -> String {
    gateways
        .iter()
        .map(|g| {
            let config_id = format!("config-{}", g.id);
            html! {
                <details id={config_id} class="config-details">
                    <summary>
                        <div class="config-summary">
                            <span class="config-flag">{g.flag}</span>
                            <span class="config-name">{g.name}</span>
                            <span class={format!("gateway-archetype {}", g.archetype_badge_class)}>
                                {g.archetype}
                            </span>
                        </div>
                        <span class="config-hint">"View Instructions ▾"</span>
                    </summary>
                    <div class="config-body">
                        <div class="config-grid">
                            <div>
                                <h4 class="config-step-title">
                                    "1. Environment Variables (" <code>".env"</code> "):"
                                </h4>
                                <div class="code-box">
                                    {g.env_example}
                                </div>
                            </div>

                            <div>
                                <h4 class="config-step-title rust">
                                    "2. Rust Server Initialization (" <code>"main.rs"</code> "):"
                                </h4>
                                <div class="code-box">
                                    {g.rust_init_code}
                                </div>
                            </div>
                        </div>

                        <div class="config-step">
                            <h4 class="config-step-title boundary">
                                "3. Current v12 capability boundary:"
                            </h4>
                            <div class="code-box">
                                {g.current_boundary}
                            </div>
                        </div>
                    </div>
                </details>
            }
        })
        .collect()
}

/// Renders the result of a simulated checkout creation if triggered.
fn render_checkout_result(simulated: Option<(String, String)>) -> String {
    if let Some((provider, url)) = simulated {
        html! {
            <div id="offline-fixture-result" class="fixture-result">
                <div class="fixture-result-header">
                    <span class="fixture-result-title">
                        "🧪 Offline Adapter Result for: " <strong>{provider.to_uppercase()}</strong>
                    </span>
                    <span class="stat-badge live">"OFFLINE FIXTURE"</span>
                </div>
                <p class="fixture-result-label">"Adapter output (no live request was made):"</p>
                <div class="code-box">
                    {&url}
                </div>
                <div class="fixture-result-actions">
                    <a href="/pricing" class="btn btn-small">
                        "Clear Simulation"
                    </a>
                </div>
            </div>
        }
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::render_checkout_result;

    #[test]
    fn offline_fixture_url_is_escaped_exactly_once() {
        let html = render_checkout_result(Some((
            "stripe".to_string(),
            "https://example.invalid/pay?recipient=a&plan=pro_plan".to_string(),
        )));
        assert!(html.contains("recipient=a&amp;plan=pro_plan"));
        assert!(!html.contains("&amp;amp;"));
    }
}
