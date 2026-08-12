//! Showcase Navigation Bar for Rullst Sovereign SaaS Blog & Publisher.
//! Provides runtime switches and visual indicators for all Rullst capabilities.

use rullst::html;

/// Renders the universal Sovereign Showcase Header with navigation buttons.
pub fn render_showcase_nav(active_route: &str) -> String {
    let routes = [
        (
            "/",
            "⚡ HTMX SSR",
            "Zero-bundle instant server-side rendering",
        ),
        (
            "/editor",
            "🏝️ Wasm Island",
            "Client-side WebAssembly reactive component",
        ),
        (
            "/live-feed",
            "🔴 LiveView WS",
            "Persistent WebSocket bidirectional state sync",
        ),
        (
            "/posts/repository",
            "🔀 Repository ORM",
            "Decoupled Data Mapper & Aggregations",
        ),
        (
            "/pricing",
            "💳 Capital Billing",
            "SaaS MRR/ARR, Webhooks & SPED NFS-e",
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

    html! {
        <div class="showcase-banner">
            <div class="showcase-banner-inner">
                <div class="showcase-brand">
                    <span class="showcase-logo">"RULLST"</span>
                    <span class="showcase-badge">"v12.0 Enterprise"</span>
                    <span class="tenant-badge" title="Active Multi-Tenant Context">
                        "Tenant: " <strong>{&tenant_id}</strong>
                    </span>
                </div>
                <div class="showcase-nav-list">
                    { rullst::html::RawHtml(buttons_html) }
                </div>
                <div class="showcase-portals">
                    <a href="/studio" target="_blank" class="portal-btn studio-btn" title="Open Developer Control Room">
                        "🚀 Studio"
                    </a>
                    <a href="/nexus" target="_blank" class="portal-btn nexus-btn" title="Open Admin CMS">
                        "🛡️ Nexus"
                    </a>
                </div>
            </div>
        </div>
    }
}

/// Renders shared CSS stylesheet for the Showcase theme.
pub fn render_shared_styles() -> String {
    r#"
    :root {
        --bg-dark: #07090e;
        --card-bg: #0d121f;
        --border-color: #1e293b;
        --accent-cyan: #06b6d4;
        --accent-blue: #3b82f6;
        --accent-purple: #8b5cf6;
        --accent-emerald: #10b981;
        --text-main: #f8fafc;
        --text-muted: #94a3b8;
    }
    * { box-sizing: border-box; }
    body {
        background-color: var(--bg-dark);
        color: var(--text-main);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
        margin: 0;
        padding: 0;
        min-height: 100vh;
    }
    .showcase-banner {
        background: rgba(13, 18, 31, 0.95);
        backdrop-filter: blur(12px);
        border-bottom: 1px solid var(--border-color);
        position: sticky;
        top: 0;
        z-index: 1000;
        padding: 0.75rem 1.5rem;
    }
    .showcase-banner-inner {
        max-width: 1300px;
        margin: 0 auto;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 1rem;
        flex-wrap: wrap;
    }
    .showcase-brand {
        display: flex;
        align-items: center;
        gap: 0.6rem;
    }
    .showcase-logo {
        font-weight: 900;
        font-size: 1.15rem;
        letter-spacing: 0.15em;
        background: linear-gradient(135deg, var(--accent-cyan), var(--accent-blue));
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
    }
    .showcase-badge {
        font-size: 0.7rem;
        background: rgba(59, 130, 246, 0.15);
        color: var(--accent-cyan);
        border: 1px solid rgba(59, 130, 246, 0.3);
        padding: 0.15rem 0.4rem;
        border-radius: 9999px;
        font-weight: 600;
    }
    .tenant-badge {
        font-size: 0.72rem;
        background: rgba(16, 185, 129, 0.12);
        color: var(--accent-emerald);
        border: 1px solid rgba(16, 185, 129, 0.3);
        padding: 0.15rem 0.5rem;
        border-radius: 0.375rem;
    }
    .showcase-nav-list {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        flex-wrap: wrap;
    }
    .showcase-btn {
        color: var(--text-muted);
        text-decoration: none;
        font-size: 0.8rem;
        font-weight: 600;
        padding: 0.4rem 0.75rem;
        border-radius: 0.5rem;
        background: rgba(30, 41, 59, 0.5);
        border: 1px solid transparent;
        transition: all 0.2s ease;
    }
    .showcase-btn:hover {
        color: var(--text-main);
        background: rgba(59, 130, 246, 0.15);
        border-color: rgba(59, 130, 246, 0.4);
    }
    .showcase-btn.active {
        color: #fff;
        background: linear-gradient(135deg, rgba(6, 182, 212, 0.25), rgba(59, 130, 246, 0.25));
        border-color: var(--accent-cyan);
        box-shadow: 0 0 12px rgba(6, 182, 212, 0.3);
    }
    .showcase-portals {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .portal-btn {
        text-decoration: none;
        font-size: 0.8rem;
        font-weight: 700;
        padding: 0.4rem 0.85rem;
        border-radius: 0.5rem;
        transition: all 0.2s ease;
    }
    .studio-btn {
        background: linear-gradient(135deg, #4f46e5, #7c3aed);
        color: #fff;
        box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
    }
    .nexus-btn {
        background: linear-gradient(135deg, #059669, #10b981);
        color: #fff;
        box-shadow: 0 2px 8px rgba(16, 185, 129, 0.3);
    }
    .portal-btn:hover {
        opacity: 0.9;
        transform: translateY(-1px);
    }
    .container {
        max-width: 1100px;
        margin: 0 auto;
        padding: 2.5rem 1.5rem;
    }
    .card {
        background: var(--card-bg);
        border: 1px solid var(--border-color);
        border-radius: 0.75rem;
        padding: 1.75rem;
        margin-bottom: 1.5rem;
        box-shadow: 0 8px 24px rgba(0,0,0,0.3);
    }
    .card-title {
        font-size: 1.35rem;
        font-weight: 700;
        margin-top: 0;
        margin-bottom: 0.75rem;
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }
    .feature-tag {
        font-size: 0.7rem;
        padding: 0.2rem 0.5rem;
        border-radius: 0.25rem;
        font-weight: 700;
        text-transform: uppercase;
    }
    .tag-orm { background: rgba(59, 130, 246, 0.2); color: #60a5fa; }
    .tag-sec { background: rgba(239, 68, 68, 0.2); color: #f87171; }
    .tag-cap { background: rgba(16, 185, 129, 0.2); color: #34d399; }
    .tag-ai { background: rgba(168, 85, 247, 0.2); color: #c084fc; }
    .btn {
        background: linear-gradient(135deg, var(--accent-blue), var(--accent-purple));
        color: #fff;
        border: none;
        border-radius: 0.5rem;
        padding: 0.65rem 1.25rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
        text-decoration: none;
        display: inline-block;
    }
    .btn:hover { opacity: 0.9; transform: translateY(-1px); }
    .btn-danger { background: linear-gradient(135deg, #ef4444, #dc2626); }
    .btn-emerald { background: linear-gradient(135deg, #10b981, #059669); }
    .code-block {
        background: #05070c;
        border: 1px solid #1e293b;
        border-radius: 0.5rem;
        padding: 1rem;
        font-family: monospace;
        font-size: 0.85rem;
        color: #38bdf8;
        overflow-x: auto;
        white-space: pre-wrap;
    }
    "#
    .to_string()
}
