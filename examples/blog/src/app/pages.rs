//! Showcase pages and small public endpoints that do not touch the `Post` model.

use crate::live_counter::CounterComponent;
use crate::showcase_nav::{render_head_assets, render_showcase_nav};
use rullst::{
    html,
    response::{Html, IntoResponse},
};

/// LiveView WebSocket Feed Page (`/live-feed` and `/live-counter`)
pub async fn live_demo() -> impl IntoResponse {
    let nav = render_showcase_nav("/live-feed");
    let head_assets = render_head_assets();
    let component_mount = rullst::live::Live::mount::<CounterComponent>("/_live").await;

    Html(html! {
        <html lang="en">
        <head>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <meta name="htmx-config" content="{&quot;includeIndicatorStyles&quot;:false}" />
            <title>"Rullst LiveView - Real-time WebSockets Feed"</title>
            { rullst::html::RawHtml(head_assets) }
            <script src="/assets/vendor/htmx-1.9.12.min.js"></script>
            <script src="/assets/vendor/htmx-ext-ws-1.9.12.js"></script>
        </head>
        <body>
            { rullst::html::RawHtml(nav) }
            <div class="container">
                <div class="card">
                    <h1 class="card-title">
                        "🔴 LiveView Server-Driven UI"
                        <span class="feature-tag tag-ai">"rullst::live"</span>
                    </h1>
                    <p class="lead">
                        "State mutations execute on Tokio server tasks. The page loads a vendored, same-origin HTMX 1.9.12 and its WebSocket extension as a thin browser transport that sends events and applies returned markup."
                    </p>

                    <div class="panel centered">
                        { rullst::html::RawHtml(component_mount) }
                    </div>
                </div>
            </div>
        </body>
        </html>
    })
}

/// WebSocket handler for LiveView
pub async fn live_ws(ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
    rullst::live::live_ws_handler::<CounterComponent>(ws).await
}

/// Honeypot sensor endpoint (`/wp-admin`)
pub async fn honeypot_trap() -> impl IntoResponse {
    tracing::warn!("🚨 Honeypot trap triggered on /wp-admin! IP logged to threat radar.");
    (
        axum::http::StatusCode::FORBIDDEN,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        "Access Denied: Incident logged in Rullst SOC Threat Radar.",
    )
}

pub async fn robots_txt() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        "User-agent: *\nDisallow: /nexus\n",
    )
}

pub async fn sitemap_xml() -> impl IntoResponse {
    let url_entry = crate::public_origin::configured_public_origin()
        .map(|origin| format!("<url><loc>{origin}/</loc></url>"))
        .unwrap_or_default();
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{url_entry}</urlset>"#
        ),
    )
}
