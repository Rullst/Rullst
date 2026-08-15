//! HTML UI and Glassmorphic Components for Capital Pricing & Gateway Showcase.

use super::gateways::{GatewayInfo, all_gateways};
use rullst::html;

/// Renders the complete HTML Pricing and Gateway Showcase page.
pub fn render_pricing_page(
    nav: String,
    styles: String,
    free_can_post: bool,
    xml_snippet: String,
    simulated_checkout_url: Option<(String, String)>, // (provider_id, url)
) -> String {
    let gateways = all_gateways();
    let configured_count = gateways.iter().filter(|g| g.is_configured()).count();
    let total_count = gateways.len();

    let extra_styles = r#"
        .pricing-hero { margin-bottom: 2rem; }
        .hero-stats { display: flex; gap: 1rem; flex-wrap: wrap; margin-top: 1rem; }
        .stat-badge { display: inline-flex; align-items: center; gap: 0.5rem; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 9999px; padding: 0.35rem 0.85rem; font-size: 0.8rem; color: #cbd5e1; }
        .stat-badge.live { border-color: rgba(16, 185, 129, 0.4); background: rgba(16, 185, 129, 0.1); color: #34d399; }
        
        .badge-emerald { background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3); }
        .badge-blue { background: rgba(59, 130, 246, 0.15); color: #60a5fa; border: 1px solid rgba(59, 130, 246, 0.3); }
        .badge-amber { background: rgba(245, 158, 11, 0.15); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.3); }
        .badge-purple { background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.3); }
        .badge-cyan { background: rgba(6, 182, 212, 0.15); color: #22d3ee; border: 1px solid rgba(6, 182, 212, 0.3); }
        .badge-indigo { background: rgba(99, 102, 241, 0.15); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.3); }

        .gateway-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1.25rem; margin-top: 1.5rem; }
        .gateway-card { background: #070a12; border: 1px solid #1e293b; border-radius: 0.75rem; padding: 1.25rem; display: flex; flex-direction: column; justify-content: space-between; transition: all 0.2s ease; }
        .gateway-card:hover { border-color: #3b82f6; transform: translateY(-2px); box-shadow: 0 8px 24px rgba(0,0,0,0.5); }
        .gateway-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.75rem; }
        .gateway-title { display: flex; align-items: center; gap: 0.6rem; font-size: 1.15rem; font-weight: 700; color: #f8fafc; margin: 0; }
        .gateway-archetype { display: inline-block; font-size: 0.7rem; font-weight: 600; padding: 0.2rem 0.6rem; border-radius: 9999px; margin-top: 0.25rem; }
        
        .gateway-specs { font-size: 0.82rem; color: #94a3b8; line-height: 1.6; margin: 0.85rem 0; border-top: 1px solid #1e293b; border-bottom: 1px solid #1e293b; padding: 0.75rem 0; }
        .spec-item { display: flex; justify-content: space-between; margin-bottom: 0.35rem; }
        .spec-item:last-child { margin-bottom: 0; }
        .spec-label { color: #64748b; }
        .spec-val { color: #e2e8f0; font-weight: 500; text-align: right; }

        .config-accordion { margin-top: 2rem; background: #070a12; border: 1px solid #1e293b; border-radius: 0.75rem; overflow: hidden; }
        .config-details summary { padding: 1.25rem; font-weight: 700; color: #f8fafc; cursor: pointer; display: flex; justify-content: space-between; align-items: center; user-select: none; background: rgba(255,255,255,0.02); }
        .config-details summary:hover { background: rgba(255,255,255,0.05); }
        .config-body { padding: 1.25rem; border-top: 1px solid #1e293b; }

        .tab-btn { background: #0f172a; border: 1px solid #334155; color: #94a3b8; padding: 0.4rem 0.85rem; border-radius: 0.375rem; font-size: 0.8rem; cursor: pointer; transition: all 0.15s; }
        .tab-btn.active, .tab-btn:hover { background: #1e293b; color: #f8fafc; border-color: #3b82f6; }
        
        .code-box { background: #030712; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1rem; font-family: monospace; font-size: 0.82rem; color: #38bdf8; overflow-x: auto; white-space: pre-wrap; word-break: break-all; }
        .checkout-box { background: rgba(59, 130, 246, 0.08); border: 1px solid rgba(59, 130, 246, 0.3); border-radius: 0.75rem; padding: 1.5rem; margin-top: 1.5rem; }
    "#;

    let gateways_cards_html = render_gateway_cards(&gateways);
    let config_accordions_html = render_config_guide(&gateways);
    let checkout_result_html = render_checkout_result(simulated_checkout_url);

    html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst Capital — Connected Payment Gateways & Configuration Guide"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <style>{ rullst::html::RawHtml(styles) }</style>
                <style>{ rullst::html::RawHtml(extra_styles.to_string()) }</style>
            </head>
            <body>
                { rullst::html::RawHtml(nav) }
                <div class="container">

                    <div class="card pricing-hero">
                        <div style="display: flex; justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 1rem;">
                            <div>
                                <h1 class="card-title" style="margin-bottom: 0.5rem;">
                                    "SaaS Pricing & Quota Governance"
                                    <span class="feature-tag tag-cap">"rullst-capital"</span>
                                </h1>
                                <p style="color: var(--text-muted); max-width: 800px; margin: 0;">
                                    "Sovereign monetization in pure Rust: tier quotas with " <code>"Billable"</code> " trait, cryptographic HMAC webhook auditing, and built-in integration across 11 global payment providers (Brazil Pix, Stripe, Lemon Squeezy, Alipay China, Polar, Paddle, Razorpay, Web3 Crypto, and International Payouts)."
                                </p>
                            </div>
                        </div>

                        <div class="hero-stats">
                            <span class="stat-badge live">
                                "🟢 " <strong>{format!("{} / {} Gateways Ready", configured_count, total_count)}</strong>
                            </span>
                            <span class="stat-badge">
                                "⚡ Pix 0.00% Fee (InfinitePay Brazil)"
                            </span>
                            <span class="stat-badge">
                                "🇨🇳 Alipay Cross-Border (> 1.3B Users)"
                            </span>
                            <span class="stat-badge">
                                "🧾 100% Tax Auto (Global MoR)"
                            </span>
                        </div>

                        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; margin-top: 1.5rem;">
                            <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1.5rem; display: flex; flex-direction: column; justify-content: space-between;">
                                <div>
                                    <h3 style="color: #94a3b8; margin: 0 0 0.5rem 0;">"Community Free"</h3>
                                    <div style="font-size: 2rem; font-weight: 800; color: #fff; margin-bottom: 1rem;">"$0" <span style="font-size: 1rem; color: #64748b;">"/mo"</span></div>
                                    <ul style="color: var(--text-muted); font-size: 0.9rem; padding-left: 1.25rem; line-height: 1.7;">
                                        <li>"Up to 3 Published Stories"</li>
                                        <li>"Zero-bundle HTMX SSR UI"</li>
                                        <li>"Community Support & Forum"</li>
                                    </ul>
                                </div>
                                <div style="margin-top: 1.5rem; padding: 0.5rem; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.3); border-radius: 0.375rem; font-size: 0.8rem; color: #34d399; text-align: center;">
                                    {if free_can_post { "✅ Quota Check: Allowed (2/3)" } else { "❌ Quota Reached" }}
                                </div>
                            </div>

                            <div style="background: #05070c; border: 2px solid #3b82f6; border-radius: 0.5rem; padding: 1.5rem; position: relative; display: flex; flex-direction: column; justify-content: space-between;">
                                <div style="position: absolute; top: -10px; right: 15px; background: #3b82f6; color: #fff; font-size: 0.65rem; font-weight: 800; padding: 0.2rem 0.5rem; border-radius: 9999px;">"MOST POPULAR"</div>
                                <div>
                                    <h3 style="color: #38bdf8; margin: 0 0 0.5rem 0;">"Pro Author"</h3>
                                    <div style="font-size: 2rem; font-weight: 800; color: #fff; margin-bottom: 1rem;">"$29" <span style="font-size: 1rem; color: #64748b;">"/mo"</span></div>
                                    <ul style="color: var(--text-muted); font-size: 0.9rem; padding-left: 1.25rem; line-height: 1.7;">
                                        <li>"Up to 50 Published Stories"</li>
                                        <li>"LiveView Real-time Comments"</li>
                                        <li>"AI Assistant & Semantic RAG"</li>
                                    </ul>
                                </div>
                                <a href="#checkout-simulator" class="btn" style="width: 100%; text-align: center; margin-top: 1rem;">"Simulate Checkout (11 Gateways)"</a>
                            </div>

                            <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 1.5rem; display: flex; flex-direction: column; justify-content: space-between;">
                                <div>
                                    <h3 style="color: #c084fc; margin: 0 0 0.5rem 0;">"Enterprise"</h3>
                                    <div style="font-size: 2rem; font-weight: 800; color: #fff; margin-bottom: 1rem;">"$99" <span style="font-size: 1rem; color: #64748b;">"/mo"</span></div>
                                    <ul style="color: var(--text-muted); font-size: 0.9rem; padding-left: 1.25rem; line-height: 1.7;">
                                        <li>"Unlimited Stories & Multi-tenant"</li>
                                        <li>"Full Studio & Nexus CMS Control Room"</li>
                                        <li>"Automated SPED NFS-e / DPS Invoicing"</li>
                                    </ul>
                                </div>
                                <a href="#checkout-simulator" class="btn btn-emerald" style="width: 100%; text-align: center; margin-top: 1rem;">"Enterprise Contract"</a>
                            </div>
                        </div>
                    </div>

                    <div id="checkout-simulator" class="card checkout-box">
                        <h2 class="card-title" style="margin-bottom: 0.5rem; color: #38bdf8;">
                            "🚀 Real-Time Checkout Session Simulator"
                        </h2>
                        <p style="color: var(--text-muted); font-size: 0.9rem; margin-bottom: 1.25rem;">
                            "Select any of the 11 supported providers to trigger compiled session generation via " <code>"rullst-capital"</code> ":"
                        </p>

                        <form method="GET" action="/checkout" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)) auto; gap: 1rem; align-items: flex-end;">
                            <div>
                                <label style="display: block; font-size: 0.8rem; color: #94a3b8; margin-bottom: 0.35rem;">"Payment Gateway:"</label>
                                <select name="provider" style="width: 100%; padding: 0.6rem; background: #070a12; border: 1px solid #334155; border-radius: 0.375rem; color: #fff; font-size: 0.9rem;">
                                    <option value="infinitepay">"🇧🇷 InfinitePay (Pix 0% & Brazil Domestic Cards)"</option>
                                    <option value="alipay">"🇨🇳 Alipay (支付宝 / Alipay+ China & Asia)"</option>
                                    <option value="stripe">"🌐 Stripe (Global Cards, Apple/Google Pay)"</option>
                                    <option value="lemonsqueezy">"🍋 Lemon Squeezy (MoR Global Tax 100%)"</option>
                                    <option value="polar">"⚡ Polar.sh (Open Source & Developer MoR)"</option>
                                    <option value="paddle">"🛡️ Paddle (Enterprise Global B2B MoR)"</option>
                                    <option value="mercadopago">"🌎 Mercado Pago (Latin America Regional)"</option>
                                    <option value="razorpay">"🇮🇳 Razorpay (India UPI & Subscriptions)"</option>
                                    <option value="coinbase">"₿ Coinbase Commerce (Web3 Crypto BTC/SOL)"</option>
                                    <option value="picpay">"📱 PicPay (Mobile Digital Wallet & QR)"</option>
                                    <option value="wise">"💸 Wise (International Payouts & Transfers)"</option>
                                </select>
                            </div>

                            <div>
                                <label style="display: block; font-size: 0.8rem; color: #94a3b8; margin-bottom: 0.35rem;">"SaaS Plan:"</label>
                                <select name="plan" style="width: 100%; padding: 0.6rem; background: #070a12; border: 1px solid #334155; border-radius: 0.375rem; color: #fff; font-size: 0.9rem;">
                                    <option value="pro_plan">"Pro Author ($29/mo)"</option>
                                    <option value="enterprise_plan">"Enterprise ($99/mo)"</option>
                                </select>
                            </div>

                            <div>
                                <label style="display: block; font-size: 0.8rem; color: #94a3b8; margin-bottom: 0.35rem;">"Subscriber Email:"</label>
                                <input type="email" name="email" value="customer@rullst.com" style="width: 100%; padding: 0.6rem; background: #070a12; border: 1px solid #334155; border-radius: 0.375rem; color: #fff; font-size: 0.9rem;" required="true" />
                            </div>

                            <div>
                                <button type="submit" class="btn btn-primary" style="padding: 0.65rem 1.25rem; font-weight: 700; width: 100%;">
                                    "Generate Checkout Session ➔"
                                </button>
                            </div>
                        </form>

                        { rullst::html::RawHtml(checkout_result_html) }
                    </div>

                    <div class="card" style="margin-top: 2rem;">
                        <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 1rem;">
                            <div>
                                <h2 class="card-title" style="margin: 0;">
                                    "💳 11 Connected Gateways in Rullst Capital"
                                </h2>
                                <p style="color: var(--text-muted); font-size: 0.9rem; margin-top: 0.35rem;">
                                    "Zero-panic architecture with strong typing, constant-time HMAC signature verification, and automated sandbox fallbacks."
                                </p>
                            </div>
                            <div>
                                <a href="https://github.com/venelouis/Rullst/blob/main/docs/src/payment-gateways-guide.md" target="_blank" class="btn" style="font-size: 0.85rem; padding: 0.4rem 0.85rem;">
                                    "📖 Open Full Architecture Guide"
                                </a>
                            </div>
                        </div>

                        <div class="gateway-grid">
                            { rullst::html::RawHtml(gateways_cards_html) }
                        </div>
                    </div>

                    <div class="card" style="margin-top: 2rem;">
                        <h2 class="card-title" style="margin-bottom: 0.5rem; color: #a78bfa;">
                            "⚙️ Step-by-Step Guide: How to Configure Each Gateway"
                        </h2>
                        <p style="color: var(--text-muted); font-size: 0.9rem;">
                            "Click any provider below to view required environment variables (" <code>".env"</code> "), Rust initialization code (" <code>"init_provider"</code> "), and webhook endpoints:"
                        </p>

                        <div class="config-accordion">
                            { rullst::html::RawHtml(config_accordions_html) }
                        </div>
                    </div>

                    <div class="card" style="margin-top: 2rem;">
                        <h2 class="card-title">"🇧🇷 Fiscal Engine: Real SPED DPS XML & XMLDSig Signature"</h2>
                        <p style="color: var(--text-muted); font-size: 0.9rem;">
                            "Rullst Capital generates standardized Declarações de Prestação de Serviços (DPS v1.0.0) XML according to Brazilian Receita Federal standards with W3C XMLDSig digital signatures in pure Rust."
                        </p>
                        <div class="code-box" style="margin-top: 1rem;">
                            { xml_snippet }
                        </div>
                    </div>

                </div>
            </body>
        </html>
    }
}

/// Renders all 11 gateway cards.
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
                            <span class={format!("stat-badge {}", status_class)} style="font-size: 0.72rem; padding: 0.2rem 0.5rem;">
                                {status_text}
                            </span>
                        </div>

                        <p style="color: #cbd5e1; font-size: 0.82rem; margin: 0.5rem 0 0.75rem 0; line-height: 1.4;">
                            {g.best_for}
                        </p>

                        <div class="gateway-specs">
                            <div class="spec-item">
                                <span class="spec-label">"Region:"</span>
                                <span class="spec-val">{g.region}</span>
                            </div>
                            <div class="spec-item">
                                <span class="spec-label">"Fees:"</span>
                                <span class="spec-val">{g.fees}</span>
                            </div>
                            <div class="spec-item">
                                <span class="spec-label">"Payout Speed:"</span>
                                <span class="spec-val">{g.payout_speed}</span>
                            </div>
                            <div class="spec-item">
                                <span class="spec-label">"Tax / Fiscal:"</span>
                                <span class="spec-val">{g.tax_handling}</span>
                            </div>
                        </div>
                    </div>

                    <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem;">
                        <a href={format!("/checkout?provider={}&plan=pro_plan&email=demo@rullst.com", g.id)} class="btn btn-primary" style="flex: 1; text-align: center; font-size: 0.8rem; padding: 0.4rem;">
                            "Test Checkout"
                        </a>
                        <a href={format!("#{}", config_id)} class="btn" style="flex: 1; text-align: center; font-size: 0.8rem; padding: 0.4rem;">
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
                <details id={config_id} class="config-details" style="border-bottom: 1px solid #1e293b;">
                    <summary>
                        <div style="display: flex; align-items: center; gap: 0.75rem;">
                            <span style="font-size: 1.25rem;">{g.flag}</span>
                            <span style="font-size: 1.05rem;">{g.name}</span>
                            <span class={format!("gateway-archetype {}", g.archetype_badge_class)} style="font-size: 0.68rem;">
                                {g.archetype}
                            </span>
                        </div>
                        <span style="color: #38bdf8; font-size: 0.85rem;">"View Instructions ▾"</span>
                    </summary>
                    <div class="config-body">
                        <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 1.25rem;">
                            <div>
                                <h4 style="color: #38bdf8; margin: 0 0 0.5rem 0; font-size: 0.9rem;">
                                    "1. Environment Variables (" <code>".env"</code> "):"
                                </h4>
                                <div class="code-box">
                                    {g.env_example}
                                </div>
                            </div>

                            <div>
                                <h4 style="color: #34d399; margin: 0 0 0.5rem 0; font-size: 0.9rem;">
                                    "2. Rust Server Initialization (" <code>"main.rs"</code> "):"
                                </h4>
                                <div class="code-box">
                                    {g.rust_init_code}
                                </div>
                            </div>
                        </div>

                        <div style="margin-top: 1rem;">
                            <h4 style="color: #c084fc; margin: 0 0 0.5rem 0; font-size: 0.9rem;">
                                "3. Cryptographic HMAC Webhook (" <code>"/webhooks/capital"</code> "):"
                            </h4>
                            <div class="code-box">
                                {g.webhook_code}
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
            <div style="margin-top: 1.5rem; padding: 1.25rem; background: rgba(16, 185, 129, 0.1); border: 1px solid rgba(16, 185, 129, 0.4); border-radius: 0.5rem;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem;">
                    <span style="font-weight: 700; color: #34d399; font-size: 0.95rem;">
                        "✅ Checkout Session Generated Successfully for: " <strong>{provider.to_uppercase()}</strong>
                    </span>
                    <span class="stat-badge live">"Status: 200 OK"</span>
                </div>
                <p style="color: #cbd5e1; font-size: 0.85rem; margin: 0.25rem 0;">"Session Redirect URL:"</p>
                <div class="code-box" style="margin-top: 0.35rem; color: #a7f3d0;">
                    {url.clone()}
                </div>
                <div style="margin-top: 0.75rem; display: flex; gap: 0.75rem;">
                    <a href={url} target="_blank" class="btn btn-emerald" style="font-size: 0.85rem; padding: 0.4rem 1rem;">
                        "Open Checkout at Provider ↗"
                    </a>
                    <a href="/pricing" class="btn" style="font-size: 0.85rem; padding: 0.4rem 1rem;">
                        "Clear Simulation"
                    </a>
                </div>
            </div>
        }
    } else {
        String::new()
    }
}
