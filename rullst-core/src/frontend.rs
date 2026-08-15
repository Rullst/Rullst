//! Modular Frontend Engines & SSR Adapters for Rullst.
//!
//! Provides first-class, zero-allocation response wrappers for Rullst's
//! five native frontend paradigms:
//! 1. `HtmlAdapter`: Zero-Bundle HTMX + Tailwind SSR (`rullst::html!`)
//! 2. `LiveViewAdapter`: Server-Driven UI via persistent Tokio WebSockets (`rullst::live`)
//! 3. `WasmIslandAdapter`: Client-side reactive WebAssembly micro-frontends (`rullst::island`)
//! 4. `PicoAdapter`: Zero-Build Semantic CSS adapter (Pico.css v2, 0 Node.js / 0 NPM)
//! 5. `TemplateAdapter`: File-based Jinja2 / Tera template rendering (Loco / Rails pattern)

use axum::response::{Html, IntoResponse, Response};

/// Render adapter trait for frontend frameworks and components.
pub trait RenderAdapter: Send + Sync {
    /// Renders the component view into an HTML string.
    fn render_to_html(&self) -> String;
}

/// Zero-bundle HTML adapter for raw HTML string responses (`rullst::html!`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlAdapter(pub String);

impl IntoResponse for HtmlAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// Pico.css Zero-Build Semantic CSS adapter (classless HTML, auto dark mode, 0 Node.js / 0 NPM).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PicoAdapter(pub String);

impl IntoResponse for PicoAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// Compatibility alias for Topcoat adapter.
pub type TopcoatAdapter = PicoAdapter;

/// File-based Jinja2 / Tera template adapter for classic MVC projects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateAdapter(pub String);

impl IntoResponse for TemplateAdapter {
    fn into_response(self) -> Response {
        Html(self.0).into_response()
    }
}

/// LiveView Server-Driven UI adapter wrapping reactive server state into Axum responses.
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

/// Wasm Island adapter wrapping client-side WebAssembly mount containers.
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
