//! Minimal embedded file-template demonstration.
//! This fixture is intentionally not represented as a Tera/Jinja implementation.

use crate::showcase_nav::{render_shared_styles, render_showcase_nav};
use axum::response::Html;

/// Renders the file-based template demo page as an Axum HTML response.
pub async fn render_templates_demo_page() -> Html<String> {
    let nav_html = render_showcase_nav("/templates-demo");
    let shared_styles = render_shared_styles();

    // Template source loaded from templates/article.html
    let template_source = include_str!("../templates/article.html");

    // Deliberately small replacement fixture, not a general-purpose template parser.
    let page_html = template_source
        .replace("{{ title }}", "Decoupled MVC Architectures in Rust")
        .replace("{{ author }}", "Chief Architect (Sovereign Systems)")
        .replace("{{ published_at }}", "2026-08-15 14:00 UTC")
        .replace(
            "{{ content }}",
            "This page starts from templates/article.html and uses a deliberately small string-replacement fixture. Because include_str! embeds the file at compile time, editing it requires recompilation. Use a reviewed template engine when an application needs escaping rules, inheritance, runtime reloads, or a general template language.",
        )
        .replace("{{ shared_styles | safe }}", &shared_styles)
        .replace("{{ nav_html | safe }}", &nav_html);

    Html(page_html)
}
