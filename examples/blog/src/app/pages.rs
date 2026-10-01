//! Showcase pages and small public endpoints that do not touch the `Post` model.

use crate::live_counter::CounterComponent;
use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
use rullst::{
    html,
    response::{Html, IntoResponse, Redirect},
};

/// LiveView WebSocket Feed Page (`/live-feed` and `/live-counter`)
pub async fn live_demo() -> impl IntoResponse {
    let nav = render_showcase_nav("/live-feed");
    let styles = render_shared_styles();
    let component_mount = rullst::live::Live::mount::<CounterComponent>("/_live").await;

    Html(html! {
        <html lang="en">
        <head>
            <meta charset="utf-8" />
            <title>"Rullst LiveView - Real-time WebSockets Feed"</title>
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png" />
            <style>{ rullst::html::RawHtml(styles) }</style>
            <script src="https://unpkg.com/htmx.org@1.9.12"></script>
            <script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/ws.js"></script>
        </head>
        <body>
            { rullst::html::RawHtml(nav) }
            <div class="container">
                <div class="card">
                    <h1 class="card-title">
                        "🔴 LiveView Server-Driven UI"
                        <span class="feature-tag tag-ai">"rullst::live"</span>
                    </h1>
                    <p style="color: var(--text-muted); margin-bottom: 1.5rem;">
                        "State mutations execute on Tokio server tasks. The page loads HTMX and its WebSocket extension as a thin browser transport that sends events and applies returned markup."
                    </p>

                    <div style="background: #05070c; border: 1px solid #1e293b; border-radius: 0.5rem; padding: 2rem; text-align: center;">
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

pub async fn favicon_handler() -> impl IntoResponse {
    Redirect::temporary("https://raw.githubusercontent.com/Rullst/Rullst/main/Rullst.png")
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

pub async fn set_security_headers(
    mut response: axum::response::Response,
) -> axum::response::Response {
    let headers = response.headers_mut();
    headers.insert(
        "Content-Security-Policy",
        axum::http::HeaderValue::from_static(
            "default-src 'self' https://unpkg.com https://cdn.tailwindcss.com https://cdn.jsdelivr.net https://cdnjs.cloudflare.com https://fonts.googleapis.com https://fonts.gstatic.com https://raw.githubusercontent.com data:; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://unpkg.com https://cdn.tailwindcss.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.tailwindcss.com https://cdn.jsdelivr.net https://cdnjs.cloudflare.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: https://raw.githubusercontent.com https://*.githubusercontent.com; connect-src 'self' ws: wss:; frame-ancestors 'self';",
        ),
    );
    headers.insert(
        "Cross-Origin-Resource-Policy",
        axum::http::HeaderValue::from_static("cross-origin"),
    );
    headers.insert(
        "X-Content-Type-Options",
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "X-Frame-Options",
        axum::http::HeaderValue::from_static("SAMEORIGIN"),
    );
    response
}
