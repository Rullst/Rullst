//! Same-origin, build-time Nexus browser assets.
//!
//! The admin shell loads only these files, so the production CSP
//! (`script-src 'self' 'nonce-…'; style-src 'self' 'nonce-…'`) covers the panel
//! without inline code, a CDN or relaxing the application-wide policy.

use axum::{
    Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

/// Path of the vendored htmx release under the Nexus mount point.
pub(crate) const HTMX_PATH: &str = "/nexus/assets/htmx-2.0.4.min.js";
/// Path of the Nexus behaviour script under the Nexus mount point.
pub(crate) const SCRIPT_PATH: &str = "/nexus/assets/nexus.js";
/// Path of the Nexus stylesheet under the Nexus mount point.
pub(crate) const STYLESHEET_PATH: &str = "/nexus/assets/nexus.css";

/// Unmodified upstream htmx 2.0.4 (`dist/htmx.min.js`, Zero-Clause BSD).
const HTMX_JS: &str = include_str!("../../assets/htmx-2.0.4.min.js");
const NEXUS_JS: &str = include_str!("../../assets/nexus.js");

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/assets/htmx-2.0.4.min.js", get(htmx_js))
        .route("/assets/nexus.js", get(nexus_js))
        .route("/assets/nexus.css", get(nexus_css))
}

async fn htmx_js() -> Response {
    asset_response("text/javascript; charset=utf-8", HTMX_JS)
}

async fn nexus_js() -> Response {
    asset_response("text/javascript; charset=utf-8", NEXUS_JS)
}

async fn nexus_css() -> Response {
    asset_response("text/css; charset=utf-8", crate::nexus::ui::NEXUS_CSS)
}

fn asset_response(content_type: &'static str, body: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn assets_are_same_origin_typed_and_self_contained() {
        for (path, content_type, marker) in [
            (HTMX_PATH, "text/javascript; charset=utf-8", "var htmx="),
            (
                SCRIPT_PATH,
                "text/javascript; charset=utf-8",
                "htmx:configRequest",
            ),
            (STYLESHEET_PATH, "text/css; charset=utf-8", ".nexus-body"),
        ] {
            let route = path.trim_start_matches("/nexus");
            let response = router::<()>()
                .oneshot(Request::get(route).body(Body::empty()).expect("request"))
                .await
                .expect("asset response");
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let headers = response.headers();
            assert_eq!(headers[header::CONTENT_TYPE], content_type, "{path}");
            assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
            let body = axum::body::to_bytes(response.into_body(), 256 * 1024)
                .await
                .expect("bounded asset");
            let body = String::from_utf8(body.to_vec()).expect("UTF-8 asset");
            assert!(body.contains(marker), "{path}");
            assert!(!body.contains("unpkg.com") && !body.contains("googleapis"));
        }
        // Nexus code must stay runnable under a CSP without 'unsafe-eval'.
        for forbidden in ["eval(", "new Function("] {
            assert!(!NEXUS_JS.contains(forbidden), "{forbidden}");
        }
    }
}
