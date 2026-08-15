//! Adobe Topcoat Zero-Build Pure CSS Demonstration.
//! Demonstrates 60 FPS native UI with Adobe Topcoat components with 0 KB JavaScript,
//! zero NPM/Node.js dependencies, and pure Rust server-side rendering.

use axum::response::Html;
use rullst::html;
use crate::showcase_nav::render_showcase_nav;

/// Renders the Topcoat Pure CSS demo page as an Axum HTML response.
pub async fn render_topcoat_demo_page() -> Html<String> {
    let showcase_nav = render_showcase_nav("/topcoat-demo");

    let page_html = html! {
        <html class="dark">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Topcoat Pure CSS &mdash; 60 FPS Zero-Build UI"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/topcoat/0.8.0/css/topcoat-desktop-dark.min.css" />
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet" />
                <style>
                    "
                    body {
                        background: #202428;
                        color: #c6c8c8;
                        font-family: 'Outfit', sans-serif;
                        margin: 0;
                        padding: 0;
                    }
                    .topcoat-page-container {
                        max-width: 1100px;
                        margin: 0 auto;
                        padding: 2rem 1.5rem;
                    }
                    .card-grid {
                        display: grid;
                        grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
                        gap: 1.5rem;
                        margin-top: 2rem;
                    }
                    .demo-card {
                        background: #2a2f34;
                        border: 1px solid #3b4249;
                        border-radius: 8px;
                        padding: 1.5rem;
                        box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                    }
                    .mono { font-family: 'JetBrains Mono', monospace; font-size: 0.85rem; }
                    .badge {
                        display: inline-block;
                        padding: 0.25rem 0.6rem;
                        background: #10b981;
                        color: #000;
                        font-weight: 700;
                        border-radius: 4px;
                        font-size: 0.75rem;
                    }
                    "
                </style>
            </head>
            <body>
                { rullst::html::RawHtml(showcase_nav) }

                <div class="topcoat-page-container">
                    <div style="text-align: center; margin-bottom: 2.5rem;">
                        <span class="badge">"60 FPS ZERO-BUILD CSS ENGINE"</span>
                        <h1 style="color: #fff; font-size: 2.5rem; margin: 0.75rem 0 0.5rem 0;">"Adobe Topcoat Pure CSS Showcase"</h1>
                        <p style="color: #8b949e; max-width: 700px; margin: 0 auto; font-size: 1.05rem;">
                            "Pure CSS component library created by Adobe Web Platform. Benchmarked at 60 FPS, with zero JavaScript dependencies, zero Node.js/NPM builds, and instant page loads."
                        </p>
                    </div>

                    <div class="topcoat-navigation-bar" style="margin-bottom: 2rem; border-radius: 6px;">
                        <div class="topcoat-navigation-bar__item left quarter">
                            <span class="topcoat-icon-button--quiet">
                                <span class="topcoat-icon topcoat-icon--menu-stack"></span>
                            </span>
                        </div>
                        <div class="topcoat-navigation-bar__item center half">
                            <h1 class="topcoat-navigation-bar__title">"Rullst Control Console &bull; Topcoat Desktop Dark"</h1>
                        </div>
                        <div class="topcoat-navigation-bar__item right quarter">
                            <button class="topcoat-button--cta">"Deploy Edge"</button>
                        </div>
                    </div>

                    <div class="card-grid">
                        <div class="demo-card">
                            <h3 style="color: #fff; margin-top: 0;">"1. Native Topcoat Buttons"</h3>
                            <p style="font-size: 0.9rem; color: #9da5b4;">
                                "GPU-accelerated buttons with built-in active, hover, and disabled states."
                            </p>
                            <div style="display: flex; flex-wrap: wrap; gap: 0.75rem; margin-top: 1rem;">
                                <button class="topcoat-button">"Default"</button>
                                <button class="topcoat-button--quiet">"Quiet"</button>
                                <button class="topcoat-button--large">"Large"</button>
                                <button class="topcoat-button--cta">"Call to Action"</button>
                                <button class="topcoat-button--large--cta">"Primary Large"</button>
                            </div>
                        </div>

                        <div class="demo-card">
                            <h3 style="color: #fff; margin-top: 0;">"2. Topcoat Text & Search Inputs"</h3>
                            <p style="font-size: 0.9rem; color: #9da5b4;">
                                "Clean text inputs with hardware-accelerated focus rings."
                            </p>
                            <div style="display: flex; flex-direction: column; gap: 0.75rem; margin-top: 1rem;">
                                <input type="text" class="topcoat-text-input" placeholder="Server Hostname" value="node-edge-01.rullst.internal" />
                                <input type="search" class="topcoat-search-input" placeholder="Search audit logs..." />
                                <textarea class="topcoat-textarea" rows="2" placeholder="System diagnostic notes..."></textarea>
                            </div>
                        </div>

                        <div class="demo-card">
                            <h3 style="color: #fff; margin-top: 0;">"3. Toggles & Checkbox Controls"</h3>
                            <p style="font-size: 0.9rem; color: #9da5b4;">
                                "Pure CSS toggle switches with smooth transitions and zero JS."
                            </p>
                            <div style="display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem;">
                                <label class="topcoat-switch">
                                    <input type="checkbox" class="topcoat-switch__input" checked="true" />
                                    <div class="topcoat-switch__toggle"></div>
                                </label>
                                <span style="font-size: 0.85rem; color: #cbd5e1;">"Enable RASP Real-time AST Inspection"</span>

                                <label class="topcoat-checkbox">
                                    <input type="checkbox" checked="true" />
                                    <div class="topcoat-checkbox__checkmark"></div>
                                    " Enforce Anti-Timing User Enumeration Guard"
                                </label>
                            </div>
                        </div>
                    </div>

                    <div style="background: #191c20; border-left: 4px solid #10b981; padding: 1.5rem; margin-top: 2.5rem; border-radius: 4px;">
                        <h4 style="color: #fff; margin: 0 0 0.5rem 0;">"⚡ Why Topcoat is Ideal for Rust Backend Engineers"</h4>
                        <p style="color: #94a3b8; font-size: 0.95rem; margin: 0; line-height: 1.6;">
                            "Unlike Tailwind which requires running a 200MB Node.js toolchain, Topcoat allows Rust developers to build stunning, dark-mode applications using pure HTML and CSS with <strong style=\"color:#fff;\">zero build steps</strong> and <strong style=\"color:#fff;\">0 KB JavaScript overhead</strong>."
                        </p>
                    </div>
                </div>
            </body>
        </html>
    };

    Html(page_html)
}
