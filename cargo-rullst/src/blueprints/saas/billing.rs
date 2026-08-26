// cargo-rullst/src/blueprints/saas/billing.rs — Stripe/Capital pricing & checkout views for SaaS blueprint.

pub fn get_billing_pages() -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let pages_billing = r##"use rullst::html;
use rullst::response::Html;

fn pricing_head() -> String {
    r#"
        <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
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
                <span style="font-size: 0.7rem; color: #94a3b8;">"(local in debug; credentials in release)"</span>
            </div>
            <a href="http://127.0.0.1:5555" target="_blank" style="background: #1e293b; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; border: 1px solid #374151; font-size: 0.85rem; transition: all 0.2s;">"📊 Studio (local)"</a>
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
