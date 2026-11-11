//! Showcase pages and small public endpoints that do not touch the `Post` model.

use rullst::response::IntoResponse;

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
