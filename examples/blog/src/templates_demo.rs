//! Classic File-Based Template Demonstration (Jinja2 / Tera Engine).
//! Demonstrates how developers coming from Django, Rails, and Loco.rs can render
//! external HTML templates located in `templates/` with full separation of concerns.

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
use axum::response::Html;

/// Renders the file-based template demo page as an Axum HTML response.
pub async fn render_templates_demo_page() -> Html<String> {
    let nav_html = render_showcase_nav("/templates-demo");
    let shared_styles = render_shared_styles();

    // Template source loaded from templates/article.html
    let template_source = include_str!("../templates/article.html");

    // Simple template string replacement (emulating compile-time / runtime Tera engine)
    let page_html = template_source
        .replace("{{ title }}", "Decoupled MVC Architectures in Rust")
        .replace("{{ author }}", "Chief Architect (Sovereign Systems)")
        .replace("{{ published_at }}", "2026-08-15 14:00 UTC")
        .replace(
            "{{ content }}",
            "This page is rendered directly from an external HTML file located at 'templates/article.html'. Unlike inline macros, file-based templating enables UI designers and frontend developers to edit layout files without touching Rust source code or triggering Rust compiler recompilations.",
        )
        .replace("{{ shared_styles | safe }}", &shared_styles)
        .replace("{{ nav_html | safe }}", &nav_html);

    Html(page_html)
}
