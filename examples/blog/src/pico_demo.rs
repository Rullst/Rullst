//! Pico.css semantic CSS demonstration without a Node.js build pipeline.
//! Pico.css is a vendored same-origin stylesheet and the two interactive
//! controls use the shared `data-action` module instead of inline handlers.

use crate::showcase_nav::{render_head_assets, render_showcase_nav};
use axum::response::Html;
use rullst::html;

/// Renders the Pico.css Semantic CSS demo page as an Axum HTML response.
pub async fn render_pico_demo_page() -> Html<String> {
    let showcase_nav = render_showcase_nav("/pico-demo");
    let head_assets = render_head_assets();

    let page_html = html! {
        <html lang="en" data-theme="dark">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Pico.css &mdash; Zero-Build Semantic CSS Engine"</title>
                <link rel="stylesheet" href="/assets/vendor/pico-2.1.1.slate.min.css" />
                { rullst::html::RawHtml(head_assets) }
            </head>
            <body>
                { rullst::html::RawHtml(showcase_nav) }

                <div class="pico-container">
                    <div class="pico-hero">
                        <span class="pico-badge">"🎨 Zero-Build Semantic CSS (Pico.css v2)"</span>
                        <h1>
                            "Pico.css: Zero-Build Semantic CSS in Rust"
                        </h1>
                        <p>
                            "Write semantic HTML5 tags in your <code class=\"pico-code-emerald\">html!</code> macros and let Pico.css style standard controls. This page adds a few layout classes and a small same-origin script for the dialog and progress buttons, so it is a practical integration example rather than a classless or JavaScript-free claim."
                        </p>
                    </div>

                    <article>
                        <header>
                            <h3 class="pico-heading">"🧪 Interactive Semantic Controls"</h3>
                        </header>
                        <p class="pico-note">
                            "Pico.css supplies the baseline styling for standard <code class=\"pico-code-sky\">&lt;input&gt;</code>, <code class=\"pico-code-sky\">&lt;select&gt;</code>, <code class=\"pico-code-sky\">&lt;button&gt;</code>, <code class=\"pico-code-sky\">&lt;progress&gt;</code>, and <code class=\"pico-code-sky\">&lt;dialog&gt;</code> elements."
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

                        <div class="grid pico-actions">
                            <button type="button" data-action="open-dialog" data-target="demo-modal">
                                "✨ Open Native Semantic Dialog (&lt;dialog&gt;)"
                            </button>
                            <button type="button" class="secondary" data-action="advance-progress" data-target="health_progress">
                                "⚡ Simulate Buffer Load (+15%)"
                            </button>
                        </div>
                    </article>

                    <dialog id="demo-modal">
                        <article>
                            <header>
                                <button type="button" aria-label="Close" rel="prev" class="pico-close" data-action="close-dialog" data-target="demo-modal"></button>
                                <h3 class="pico-heading">"🛡️ Native HTML5 &lt;dialog&gt; Modal"</h3>
                            </header>
                            <p>
                                "This modal is a standard HTML5 <code class=\"pico-code-sky\">&lt;dialog&gt;</code> element. Pico.css provides built-in backdrop blurring, animations, and typography with zero JavaScript UI libraries."
                            </p>
                            <footer>
                                <button type="button" data-action="close-dialog" data-target="demo-modal">"Close Dialog"</button>
                            </footer>
                        </article>
                    </dialog>

                    <div class="comparison-grid">
                        <article class="comparison-htmx">
                            <header>
                                <h4 class="comparison-title">"⚡ HTMX + Tailwind SSR"</h4>
                                <span class="comparison-subtitle">"One option for application interfaces"</span>
                            </header>
                            <ul class="comparison-list">
                                <li><strong>"Partial Updates"</strong>": HTMX can request and swap server-rendered fragments without a full navigation."</li>
                                <li><strong>"Tailwind CSS Utility"</strong>": Pixel-perfect custom designs with utility classes."</li>
                                <li><strong>"Consider For"</strong>": Server-oriented forms, CRUD surfaces, and progressively enhanced dashboards."</li>
                            </ul>
                        </article>

                        <article class="comparison-pico">
                            <header>
                                <h4 class="comparison-title">"🎨 Zero-Build Semantic CSS (Pico.css)"</h4>
                                <span class="comparison-subtitle">"A lightweight semantic-CSS option"</span>
                            </header>
                            <ul class="comparison-list">
                                <li><strong>"Semantic Defaults"</strong>": Standard controls receive useful baseline styling."</li>
                                <li><strong>"No Node.js Pipeline"</strong>": The vendored stylesheet needs no local NPM build step."</li>
                                <li><strong>"Consider For"</strong>": Prototypes, documentation, and restrained internal interfaces."</li>
                            </ul>
                        </article>
                    </div>
                </div>
            </body>
        </html>
    };

    Html(page_html)
}
