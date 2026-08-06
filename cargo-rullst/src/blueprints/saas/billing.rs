// cargo-rullst/src/blueprints/saas/billing.rs — Stripe/Capital pricing & checkout views for SaaS blueprint.

pub fn get_billing_pages() -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let pages_billing = r##"use rullst::html;
use axum::response::Html;

fn pricing_head() -> String {
    r#"
        <link rel="icon" type="image/png" href="data:image/png;base64,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" />
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Select a Plan - Rullst Billing</title>
        <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
        <style>
            * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
            body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow-x: hidden; position: relative; }
            .glow-bg { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(5, 150, 105, 0.15) 0%, rgba(249, 115, 22, 0.05) 50%, transparent 100%); top: -10%; left: -10%; z-index: -1; }
            .glow-bg-right { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(249, 115, 22, 0.1) 0%, rgba(5, 150, 105, 0.05) 50%, transparent 100%); bottom: -10%; right: -10%; z-index: -1; }
            .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; text-align: center; z-index: 1; }
            .header { margin-bottom: 3.5rem; }
            .badge { background: linear-gradient(135deg, #10b981 0%, #f97316 100%); color: white; padding: 0.35rem 1rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 1rem; }
            h1 { font-size: 3rem; font-weight: 700; background: linear-gradient(to right, #ffffff, #9ca3af); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 1rem; }
            .subtitle { color: #9ca3af; font-size: 1.15rem; max-width: 600px; margin: 0 auto; }
            
            .setup-banner { background: rgba(5, 150, 105, 0.1); backdrop-filter: blur(12px); border: 1px solid rgba(5, 150, 105, 0.2); border-radius: 1rem; padding: 1.5rem; margin-bottom: 3rem; max-width: 800px; margin-left: auto; margin-right: auto; display: flex; gap: 1.5rem; align-items: flex-start; text-align: left; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2); animation: fade-in 1s ease-out; }
            @keyframes fade-in { from { opacity: 0; transform: translateY(-10px); } to { opacity: 1; transform: translateY(0); } }
            .setup-banner-icon { font-size: 2rem; }
            .setup-banner-content h4 { font-size: 1.2rem; margin-bottom: 0.5rem; color: #e0e7ff; }
            .setup-banner-content p { color: #9ca3af; line-height: 1.5; margin-bottom: 1rem; }
            
            .pricing-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 2rem; max-width: 1000px; margin: 0 auto; }
            .pricing-card { background: rgba(15, 23, 42, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.05); border-radius: 1.5rem; padding: 2.5rem; text-align: left; display: flex; flex-direction: column; transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1); position: relative; }
            .pricing-card:hover { transform: translateY(-8px); border-color: rgba(5, 150, 105, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
            .pricing-card.premium { border: 2px solid #10b981; }
            .pricing-card.premium::after { content: 'Best Value'; position: absolute; top: -14px; right: 24px; background: #10b981; color: white; font-size: 0.75rem; font-weight: 700; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; }
            .plan-name { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.5rem; }
            .plan-desc { color: #9ca3af; font-size: 0.95rem; margin-bottom: 2rem; min-height: 40px; }
            .price-container { display: flex; align-items: baseline; margin-bottom: 2.5rem; }
            .currency { font-size: 1.75rem; font-weight: 600; color: #ffffff; }
            .price { font-size: 3.5rem; font-weight: 700; color: #ffffff; letter-spacing: -0.02em; }
            .period { color: #9ca3af; font-size: 1rem; margin-left: 0.5rem; }
            .features-list { list-style: none; margin-bottom: 3rem; flex-grow: 1; }
            .features-list li { display: flex; align-items: center; color: #d1d5db; font-size: 0.95rem; margin-bottom: 1rem; }
            .features-list svg { width: 20px; height: 20px; margin-right: 0.75rem; color: #10b981; flex-shrink: 0; }
            .btn-checkout { display: block; width: 100%; text-align: center; padding: 1rem; border-radius: 12px; font-weight: 600; text-decoration: none; font-size: 1rem; transition: all 0.3s; cursor: pointer; border: none; }
            .btn-checkout.primary { background: linear-gradient(135deg, #10b981 0%, #059669 100%); color: white; box-shadow: 0 4px 14px rgba(5, 150, 105, 0.4); }
            .btn-checkout.primary:hover { background: linear-gradient(135deg, #059669 0%, #047857 100%); box-shadow: 0 6px 20px rgba(5, 150, 105, 0.6); }
            .btn-checkout.secondary { background: rgba(255, 255, 255, 0.08); color: white; border: 1px solid rgba(255, 255, 255, 0.1); }
            .btn-checkout.secondary:hover { background: rgba(255, 255, 255, 0.15); border-color: rgba(255, 255, 255, 0.25); }
        </style>
    "#.to_string()
}

fn pricing_navbar() -> String {
    html! {
        <div style="display: flex; justify-content: flex-end; align-items: flex-start; gap: 1rem; margin-bottom: 2rem;">
            <a href="/login" style="background: rgba(255,255,255,0.05); color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; border: 1px solid rgba(255,255,255,0.1); font-size: 0.85rem; transition: all 0.2s;">"🔑 Login"</a>
            <div style="display: flex; flex-direction: column; align-items: center; gap: 0.25rem;">
                <a href="/nexus" style="background: #1e293b; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; border: 1px solid #374151; font-size: 0.85rem; transition: all 0.2s;">"⚙️ Nexus CMS"</a>
                <span style="font-size: 0.7rem; color: #94a3b8;">"(login: admin / password)"</span>
            </div>
            <a href="http://localhost:5555" target="_blank" style="background: #1e293b; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; border: 1px solid #374151; font-size: 0.85rem; transition: all 0.2s;">"📊 Studio"</a>
        </div>
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

    Html(html! {
        <html lang="en">
            <head>
                { rullst::html::RawHtml(pricing_head()) }
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
    })
}
"##;
    manifest.push(("src/pages/billing.rs", pages_billing.to_string()));

    let pages_mod = r##"pub mod auth;
pub mod billing;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
