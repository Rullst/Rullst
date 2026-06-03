use rullst::html;
use axum::response::Html;

pub fn pricing_page() -> Html<String> {
    Html(html! {
        <html lang="en" class="dark">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>"Select a Plan - Rullst Billing"</title>
            <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
            <style>
                "* { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow-x: hidden; position: relative; }
                .glow-bg { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, rgba(139, 92, 246, 0.05) 50%, transparent 100%); top: -10%; left: -10%; z-index: -1; }
                .glow-bg-right { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, rgba(99, 102, 241, 0.05) 50%, transparent 100%); bottom: -10%; right: -10%; z-index: -1; }
                .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; text-align: center; z-index: 1; }
                .header { margin-bottom: 3.5rem; }
                .badge { background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%); color: white; padding: 0.35rem 1rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 1rem; }
                h1 { font-size: 3rem; font-weight: 700; background: linear-gradient(to right, #ffffff, #9ca3af); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 1rem; }
                .subtitle { color: #9ca3af; font-size: 1.15rem; max-width: 600px; margin: 0 auto; }
                
                .setup-banner { background: rgba(99, 102, 241, 0.1); backdrop-filter: blur(12px); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; padding: 1.5rem; margin-bottom: 3rem; max-width: 800px; margin-left: auto; margin-right: auto; display: flex; gap: 1.5rem; align-items: flex-start; text-align: left; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2); animation: fade-in 1s ease-out; }
                @keyframes fade-in { from { opacity: 0; transform: translateY(-10px); } to { opacity: 1; transform: translateY(0); } }
                .setup-banner-icon { font-size: 2rem; }
                .setup-banner-content h4 { font-size: 1.2rem; margin-bottom: 0.5rem; color: #e0e7ff; }
                .setup-banner-content p { color: #9ca3af; line-height: 1.5; margin-bottom: 1rem; }
                .setup-banner-content pre { background: #111827; padding: 1rem; border-radius: 0.5rem; border: 1px solid #1f2937; overflow-x: auto; color: #a5b4fc; font-family: ui-monospace, monospace; font-size: 0.9rem; margin: 0; }
                
                .pricing-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 2rem; max-width: 1000px; margin: 0 auto; }
                .pricing-card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; padding: 3rem 2rem; text-align: left; display: flex; flex-direction: column; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); position: relative; }
                .pricing-card:hover { transform: translateY(-8px); border-color: rgba(99, 102, 241, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
                .pricing-card.premium { border: 2px solid #6366f1; }
                .pricing-card.premium::after { content: 'Best Value'; position: absolute; top: -14px; right: 24px; background: #6366f1; color: white; font-size: 0.75rem; font-weight: 700; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; }
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
                .btn-checkout.primary { background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); color: white; box-shadow: 0 4px 14px rgba(99, 102, 241, 0.4); }
                .btn-checkout.primary:hover { background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%); box-shadow: 0 6px 20px rgba(99, 102, 241, 0.6); }
                .btn-checkout.secondary { background: rgba(255, 255, 255, 0.08); color: white; border: 1px solid rgba(255, 255, 255, 0.1); }
                .btn-checkout.secondary:hover { background: rgba(255, 255, 255, 0.15); border-color: rgba(255, 255, 255, 0.25); }"
            </style>
        </head>
        <body>
            <div class="glow-bg"></div>
            <div class="glow-bg-right"></div>
            <div class="container">
                
                <div class="setup-banner">
                    <div class="setup-banner-icon">"🚀"</div>
                    <div class="setup-banner-content">
                        <h4>"Stripe Setup Required"</h4>
                        <p>"To enable real checkouts, create a " <code>".env"</code> " file in your project root with your API keys:"</p>
                        <pre><code>"BILLING_PROVIDER=stripe
BILLING_API_KEY=sk_test_...
BILLING_WEBHOOK_SECRET=whsec_..."</code></pre>
                    </div>
                </div>

                <div class="header">
                    <span class="badge">"Rullst Capital"</span>
                    <h1>"Simple, Transparent Pricing"</h1>
                    <p class="subtitle">"Choose the perfect plan to boost your application with next-gen fullstack performance."</p>
                </div>
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
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Up to 5 Projects"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Standard SQLite Database"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Email Support"
                            </li>
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
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Unlimited Projects"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "PostgreSQL & SQLite Support"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Adaptive WAF & Bot Management"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Priority Support (Sub-1 hour)"
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_pro" class="btn-checkout primary">"Go Pro"</a>
                    </div>
                </div>
            </div>
        </body>
        </html>
    })
}
