//! Pico.css Zero-Build Semantic CSS Demonstration.
//! Demonstrates classless, semantic HTML styling with Pico.css v2 with 0 KB JavaScript,
//! zero NPM/Node.js dependencies, automatic OS Dark/Light theme detection, and pure Rust server-side rendering.

use axum::response::Html;
use rullst::html;
use crate::showcase_nav::{render_shared_styles, render_showcase_nav};

/// Renders the Pico.css Semantic CSS demo page as an Axum HTML response.
pub async fn render_pico_demo_page() -> Html<String> {
    let showcase_nav = render_showcase_nav("/pico-demo");
    let shared_styles = render_shared_styles();

    let page_html = html! {
        <html lang="en" data-theme="dark">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Pico.css &mdash; Zero-Build Semantic CSS Engine"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.slate.min.css" />
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
                <style>
                    { rullst::html::RawHtml(shared_styles) }
                    "
                    body {
                        font-family: 'Outfit', sans-serif;
                        margin: 0;
                        padding: 0;
                        min-height: 100vh;
                    }
                    .pico-container {
                        max-width: 1000px;
                        margin: 0 auto;
                        padding: 2.5rem 1.5rem 4rem 1.5rem;
                    }
                    .mono { font-family: 'JetBrains Mono', monospace; font-size: 0.85rem; }
                    .badge {
                        display: inline-block;
                        padding: 0.3rem 0.8rem;
                        background: rgba(16, 185, 129, 0.15);
                        border: 1px solid rgba(16, 185, 129, 0.3);
                        color: #34d399;
                        font-weight: 700;
                        border-radius: 9999px;
                        font-size: 0.75rem;
                        letter-spacing: 0.05em;
                        text-transform: uppercase;
                    }
                    .comparison-grid {
                        display: grid;
                        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
                        gap: 1.5rem;
                        margin-top: 2rem;
                    }
                    "
                </style>
            </head>
            <body>
                { rullst::html::RawHtml(showcase_nav) }

                <div class="pico-container">
                    <div style="text-align: center; margin-bottom: 2.5rem;">
                        <span class="badge">"🎨 Zero-Build Semantic CSS (Pico.css v2)"</span>
                        <h1 style="font-size: 2.75rem; font-weight: 800; margin: 0.75rem 0 0.5rem 0; letter-spacing: -0.025em;">
                            "Pico.css: Zero-Build Semantic CSS in Rust"
                        </h1>
                        <p style="color: #94a3b8; max-width: 760px; margin: 0 auto; font-size: 1.1rem; line-height: 1.6;">
                            "Write plain, semantic HTML5 tags in your <code style=\"color:#34d399;\">html!</code> macros with <strong style=\"color:#fff;\">0 custom CSS classes</strong>. Pico.css automatically applies modern typography, form controls, and dark/light themes directly to standard HTML tags."
                        </p>
                    </div>

                    <article>
                        <header>
                            <h3 style="margin: 0; font-weight: 700;">"🧪 Interactive Semantic Console (100% Classless HTML)"</h3>
                        </header>
                        <p style="color: #94a3b8; font-size: 0.95rem;">
                            "Every element below is styled strictly by its HTML tag name (<code style=\"color:#38bdf8;\">&lt;input&gt;</code>, <code style=\"color:#38bdf8;\">&lt;select&gt;</code>, <code style=\"color:#38bdf8;\">&lt;button&gt;</code>, <code style=\"color:#38bdf8;\">&lt;progress&gt;</code>, <code style=\"color:#38bdf8;\">&lt;dialog&gt;</code>) without writing any CSS classes."
                        </p>

                        <div class="grid">
                            <div>
                                <label for="node_name">"Cluster Node Name"</label>
                                <input type="text" id="node_name" name="node_name" value="edge-san-francisco-01.rullst.cloud" />
                            </div>
                            <div>
                                <label for="env_mode">"Deployment Environment"</label>
                                <select id="env_mode">
                                    <option selected="true">"Production Edge (High Availability)"</option>
                                    <option>"Staging Sandbox"</option>
                                    <option>"Local Development"</option>
                                </select>
                            </div>
                        </div>

                        <label for="health_progress">"Real-time Telemetry Buffer Saturation"</label>
                        <progress id="health_progress" value="78" max="100"></progress>

                        <div class="grid" style="margin-top: 1.25rem;">
                            <button type="button" onclick="document.getElementById('demo-modal').showModal()">
                                "✨ Open Native Semantic Dialog (&lt;dialog&gt;)"
                            </button>
                            <button type="button" class="secondary" onclick="var p = document.getElementById('health_progress'); p.value = (p.value >= 100) ? 20 : p.value + 15;">
                                "⚡ Simulate Buffer Load (+15%)"
                            </button>
                        </div>
                    </article>

                    <dialog id="demo-modal">
                        <article>
                            <header>
                                <button aria-label="Close" rel="prev" onclick="document.getElementById('demo-modal').close()" style="float: right;"></button>
                                <h3 style="margin: 0; font-weight: 700;">"🛡️ Native HTML5 &lt;dialog&gt; Modal"</h3>
                            </header>
                            <p>
                                "This modal is a standard HTML5 <code style=\"color:#38bdf8;\">&lt;dialog&gt;</code> element. Pico.css provides built-in backdrop blurring, animations, and typography with zero JavaScript UI libraries."
                            </p>
                            <footer>
                                <button type="button" onclick="document.getElementById('demo-modal').close()">"Close Dialog"</button>
                            </footer>
                        </article>
                    </dialog>

                    <div class="comparison-grid">
                        <article style="border-top: 4px solid #10b981;">
                            <header>
                                <h4 style="margin: 0; font-weight: 700; color: #10b981;">"⚡ Zero-Bundle HTMX + Tailwind SSR"</h4>
                                <span style="font-size: 0.8rem; color: #94a3b8;">"Rullst Flagship (Best for Full Apps)"</span>
                            </header>
                            <ul style="color: #cbd5e1; font-size: 0.9rem; line-height: 1.6; padding-left: 1.25rem;">
                                <li><strong>"Full Interactivity"</strong>": Partial DOM swaps via AJAX without page reloads."</li>
                                <li><strong>"Tailwind CSS Utility"</strong>": Pixel-perfect custom designs with utility classes."</li>
                                <li><strong>"Best For"</strong>": SaaS platforms, complete web apps, e-commerce, complex dashboards."</li>
                            </ul>
                        </article>

                        <article style="border-top: 4px solid #38bdf8;">
                            <header>
                                <h4 style="margin: 0; font-weight: 700; color: #38bdf8;">"🎨 Zero-Build Semantic CSS (Pico.css)"</h4>
                                <span style="font-size: 0.8rem; color: #94a3b8;">"Minimalist (Best for Internal Tools)"</span>
                            </header>
                            <ul style="color: #cbd5e1; font-size: 0.9rem; line-height: 1.6; padding-left: 1.25rem;">
                                <li><strong>"Classless HTML"</strong>": Zero CSS classes to learn or configure."</li>
                                <li><strong>"0 Node.js / NPM"</strong>": Pure Cargo development with 0 build steps."</li>
                                <li><strong>"Best For"</strong>": Backend developers, CLI dashboards, sensor monitors, internal tools."</li>
                            </ul>
                        </article>
                    </div>
                </div>
            </body>
        </html>
    };

    Html(page_html)
}
