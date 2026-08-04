//! Hybrid Frontend SSR Adapters for Leptos, Dioxus, and HTMX.

use axum::response::{Html, IntoResponse, Response};

/// Render adapter trait for frontend frameworks.
pub trait RenderAdapter: Send + Sync {
    /// Renders the component view into an HTML string.
    fn render_to_html(&self) -> String;
}

/// HTML adapter for raw HTML string responses.
pub struct HtmlAdapter(pub String);

impl IntoResponse for HtmlAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// Leptos SSR Adapter wrapping Leptos views into Axum responses.
pub struct LeptosAdapter<T> {
    /// Inner Leptos view component.
    pub view: T,
}

impl<T: std::fmt::Display> LeptosAdapter<T> {
    /// Creates a new Leptos SSR Adapter.
    pub fn new(view: T) -> Self {
        Self { view }
    }
}

impl<T: std::fmt::Display> IntoResponse for LeptosAdapter<T> {
    fn into_response(self) -> Response {
        Html(self.view.to_string()).into_response()
    }
}

/// Dioxus SSR Adapter wrapping Dioxus components into Axum responses.
pub struct DioxusAdapter<T> {
    /// Inner Dioxus component.
    pub component: T,
}

impl<T: std::fmt::Display> DioxusAdapter<T> {
    /// Creates a new Dioxus SSR Adapter.
    pub fn new(component: T) -> Self {
        Self { component }
    }
}

impl<T: std::fmt::Display> IntoResponse for DioxusAdapter<T> {
    fn into_response(self) -> Response {
        Html(self.component.to_string()).into_response()
    }
}
