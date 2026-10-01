//! Same-origin browser assets embedded at build time.
//!
//! Serving the stylesheet, scripts, vendored HTMX/Pico.css and the logo from
//! the application's own origin lets every page render under the production
//! Content Security Policy (`script-src 'self' 'nonce-…'; style-src 'self'
//! 'nonce-…'; img-src 'self' data:`) without a CDN or a relaxed policy.
//! Provenance of the vendored files is recorded in `assets/vendor/README.md`.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

const SHOWCASE_CSS: &str = include_str!("../assets/showcase.css");
const PAGES_CSS: &str = include_str!("../assets/pages.css");
const SHOWCASE_JS: &str = include_str!("../assets/showcase.js");
const HTMX_JS: &str = include_str!("../assets/vendor/htmx-1.9.12.min.js");
const HTMX_WS_JS: &str = include_str!("../assets/vendor/htmx-ext-ws-1.9.12.js");
const PICO_CSS: &str = include_str!("../assets/vendor/pico-2.1.1.slate.min.css");
const RULLST_LOGO_PNG: &[u8] = include_bytes!("../assets/rullst-logo.png");

const CSS: &str = "text/css; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";

fn asset(content_type: &'static str, body: &'static [u8]) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        body,
    )
        .into_response()
}

pub async fn showcase_css() -> Response {
    asset(CSS, SHOWCASE_CSS.as_bytes())
}

pub async fn pages_css() -> Response {
    asset(CSS, PAGES_CSS.as_bytes())
}

pub async fn showcase_js() -> Response {
    asset(JAVASCRIPT, SHOWCASE_JS.as_bytes())
}

pub async fn htmx_js() -> Response {
    asset(JAVASCRIPT, HTMX_JS.as_bytes())
}

pub async fn htmx_ws_js() -> Response {
    asset(JAVASCRIPT, HTMX_WS_JS.as_bytes())
}

pub async fn pico_css() -> Response {
    asset(CSS, PICO_CSS.as_bytes())
}

/// The Rullst logo, also answered for `/favicon.ico`.
pub async fn rullst_logo() -> Response {
    asset("image/png", RULLST_LOGO_PNG)
}
