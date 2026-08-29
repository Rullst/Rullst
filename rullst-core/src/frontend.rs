//! Modular Frontend Engines & SSR Adapters for Rullst.
//!
//! Provides small HTML response wrappers used by Rullst frontend compatibility
//! profiles. These types do not install browser assets, establish a WebSocket
//! lifecycle, hydrate Wasm, load Pico.css, or render Tera templates by
//! themselves; those integrations remain explicit.

use axum::response::{Html, IntoResponse, Response};

/// Render adapter trait for frontend frameworks and components.
pub trait RenderAdapter: Send + Sync {
    /// Renders the component view into an HTML string.
    fn render_to_html(&self) -> String;
}

/// HTML adapter for raw HTML string responses (`rullst::html!`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlAdapter(pub String);

impl IntoResponse for HtmlAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// HTML response compatibility wrapper for a page that may load Pico.css.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PicoAdapter(pub String);

impl IntoResponse for PicoAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// Compatibility alias for Topcoat adapter.
pub type TopcoatAdapter = PicoAdapter;

/// HTML response compatibility wrapper for output rendered by a template engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateAdapter(pub String);

impl IntoResponse for TemplateAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// HTML response wrapper; the LiveView WebSocket lifecycle is configured separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveViewAdapter<T> {
    /// Inner component or live view state.
    pub view: T,
}

impl<T: std::fmt::Display> LiveViewAdapter<T> {
    /// Creates a new LiveView adapter.
    pub fn new(view: T) -> Self {
        Self { view }
    }
}

impl<T: std::fmt::Display> IntoResponse for LiveViewAdapter<T> {
    fn into_response(self) -> Response {
        Html(self.view.to_string()).into_response()
    }
}

/// HTML response wrapper for Wasm Island mount markup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasmIslandAdapter<T> {
    /// Inner component or island markup.
    pub component: T,
}

impl<T: std::fmt::Display> WasmIslandAdapter<T> {
    /// Creates a new Wasm Island adapter.
    pub fn new(component: T) -> Self {
        Self { component }
    }
}

impl<T: std::fmt::Display> IntoResponse for WasmIslandAdapter<T> {
    fn into_response(self) -> Response {
        Html(self.component.to_string()).into_response()
    }
}

/// Compatibility alias for Leptos pattern views.
pub type LeptosAdapter<T> = LiveViewAdapter<T>;

/// Compatibility alias for Dioxus pattern components.
pub type DioxusAdapter<T> = WasmIslandAdapter<T>;
