//! Showcase Navigation Bar for Rullst Sovereign SaaS Blog & Publisher.
//! Provides runtime switches and visual indicators for all Rullst capabilities.

use rullst::html;

/// Renders the universal Sovereign Showcase Header with navigation buttons.
pub fn render_showcase_nav(active_route: &str) -> String {
    let routes = [
        (
            "/",
            "⚡ Server-rendered HTML + HTMX",
            "Typed server-rendered HTML with optional HTMX browser behavior",
        ),
        (
            "/live-feed",
            "🔴 LiveView WS (rullst::live)",
            "Persistent WebSocket bidirectional state sync (Phoenix & Dioxus pattern)",
        ),
        (
            "/pico-demo",
            "🎨 Pico Semantic CSS",
            "Zero-build semantic CSS, auto dark mode, 0 Node.js/NPM (Pico.css v2)",
        ),
        (
            "/templates-demo",
            "📄 Embedded File Template",
            "External HTML embedded and populated by a deliberately small example renderer",
        ),
        (
            "/posts/repository",
            "🔀 Repository ORM",
            "Decoupled Data Mapper & Aggregations",
        ),
        (
            "/pricing",
            "💳 Capital Billing",
            "Billing adapters, quota checks and offline checkout fixtures",
        ),
        (
            "/security-demo",
            "🛡️ Security & RASP",
            "WAF, Login Jail, Tarpit & Honeypots",
        ),
        (
            "/ai-assistant",
            "🤖 AI & RAG",
            "Vector semantic search & Prompt Shield",
        ),
        (
            "/omni",
            "📱 Omni App",
            "Interactive Mobile Viewport Simulator and Desktop Exporter",
        ),
    ];

    let buttons_html: String = routes
        .iter()
        .map(|(path, label, title)| {
            let is_active = *path == active_route;
            let active_class = if is_active {
                "showcase-btn active"
            } else {
                "showcase-btn"
            };
            html! {
                <a href={path} class={active_class} title={title}>
                    {label}
                </a>
            }
        })
        .collect();

    let tenant_id =
        rullst::multitenant::current_tenant_id().unwrap_or_else(|| "community".to_string());

    let portals_html = if cfg!(debug_assertions) {
        [
            html! {
                <a href="http://127.0.0.1:5555" target="_blank" rel="noopener noreferrer" class="portal-btn studio-btn" title="Open the local Developer Control Room">
                    "🚀 Local Studio"
                </a>
            },
            html! {
                <a href="/nexus" target="_blank" rel="noopener noreferrer" class="portal-btn nexus-btn" title="Open the loopback-only development CMS">
                    "🛡️ Local Nexus"
                </a>
            },
        ]
        .concat()
    } else {
        [
            html! {
                <span class="portal-btn portal-disabled" title="Studio is a loopback-only developer tool and is not exposed by this public showcase">
                    "🚀 Studio: local only"
                </span>
            },
            html! {
                <span class="portal-btn portal-disabled" title="Nexus requires a deployment-specific protected administration policy">
                    "🛡️ Nexus: protected"
                </span>
            },
        ]
        .concat()
    };

    html! {
        <div class="showcase-banner">
            <div class="showcase-banner-inner">
                <a href="/" class="showcase-brand">
                    <img src="/assets/rullst-logo.png" alt="Rullst Logo" class="showcase-brand-img" />
                    <span class="showcase-logo">"RULLST"</span>
                    <span class="showcase-badge">"v12 showcase"</span>
                    <span class="tenant-badge" title="Active Multi-Tenant Context">
                        "Tenant: " <strong>{&tenant_id}</strong>
                    </span>
                </a>

                <button type="button" class="hamburger-btn" data-action="toggle-drawer" data-target="showcase-drawer" aria-controls="showcase-drawer" aria-expanded="false" aria-label="Toggle Navigation Menu">
                    "☰"
                </button>

                <div class="showcase-nav-list desktop-nav">
                    { rullst::html::RawHtml(buttons_html.clone()) }
                </div>
                <div class="showcase-portals desktop-nav">
                    { rullst::html::RawHtml(portals_html.clone()) }
                </div>

                <div id="showcase-drawer" class="showcase-mobile-drawer">
                    <div class="showcase-mobile-nav-list">
                        { rullst::html::RawHtml(buttons_html) }
                    </div>
                    <div class="showcase-mobile-portals">
                        { rullst::html::RawHtml(portals_html) }
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Renders the same-origin icon, stylesheet and behavior module shared by every page.
///
/// The production Content Security Policy (`style-src 'self' 'nonce-…'`,
/// `script-src 'self' 'nonce-…'`) admits these files without nonces or a
/// relaxed policy; see [`crate::assets`].
pub fn render_head_assets() -> String {
    html! {
        <link rel="icon" type="image/png" href="/assets/rullst-logo.png" />
        <link rel="stylesheet" href="/assets/showcase.css" />
        <link rel="stylesheet" href="/assets/pages.css" />
        <script type="module" src="/assets/showcase.js"></script>
    }
}
