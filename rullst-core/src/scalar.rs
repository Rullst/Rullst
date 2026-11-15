//! Interactive Scalar API Documentation Page (`/docs`)
#![cfg(not(target_arch = "wasm32"))]

use axum::{
    Extension, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
};
use serde_json::json;

use crate::security::CspNonce;

/// The Scalar API reference bundle the page loads: one pinned file.
const SCALAR_SCRIPT: &str =
    "https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.67.0/dist/browser/standalone.js";
/// Subresource Integrity of [`SCALAR_SCRIPT`], so the browser refuses any
/// other content at that URL.
const SCALAR_INTEGRITY: &str =
    "sha384-6c7Vmx+i0yi8gBbltn0x1cavD+zsMGw2xmXXVyacPJLIGBxwaVimW5TW0WiW17Ir";

/// Generates the HTML5 Scalar API documentation interface.
///
/// Its inline style and fallback script carry no nonce, so a strict Content
/// Security Policy blocks them; [`scalar_docs_router`] serves the page with
/// the request's nonce and a policy of its own.
pub fn render_scalar_html(openapi_url: &str) -> String {
    render_page(openapi_url, None)
}

/// The page, with `nonce` on its inline style and scripts and in the
/// `csp-nonce` meta element Scalar reads for the styles it inserts.
fn render_page(openapi_url: &str, nonce: Option<&CspNonce>) -> String {
    let configuration = json!({ "url": openapi_url, "withDefaultFonts": false }).to_string();
    let configuration = crate::html::escape_str(&configuration);
    let openapi_url = crate::html::escape_str(openapi_url);
    let (meta, nonce) = match nonce {
        Some(nonce) => (
            format!(
                "\n    <meta property=\"csp-nonce\" content=\"{}\">",
                nonce.as_str()
            ),
            format!(" nonce=\"{}\"", nonce.as_str()),
        ),
        None => (String::new(), String::new()),
    };
    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">{meta}
    <title>API Documentation — Rullst Scalar UI</title>
    <style{nonce}>
        body {{
            margin: 0;
            padding: 0;
            background-color: #0f172a;
            color: #f8fafc;
            font-family: system-ui, -apple-system, sans-serif;
        }}
    </style>
</head>
<body>
    <script id="api-reference" data-url="{openapi_url}" data-configuration="{configuration}"{nonce}></script>
    <script src="{SCALAR_SCRIPT}" integrity="{SCALAR_INTEGRITY}" crossorigin="anonymous"{nonce}></script>
    <script{nonce}>
        // Status-only fallback for offline development environments.
        if (typeof Scalar === 'undefined') {{
            console.warn('Rullst Scalar: CDN unreachable, rendering offline fallback status page.');
            const specUrl = document.getElementById('api-reference').dataset.url;
            const container = document.createElement('main');
            const title = document.createElement('h1');
            const message = document.createElement('p');
            const path = document.createElement('code');
            title.textContent = 'Rullst API Documentation';
            message.textContent = 'Scalar UI is unavailable. OpenAPI document: ';
            path.textContent = specUrl;
            message.appendChild(path);
            container.append(title, message);
            document.body.replaceChildren(container);
        }}
    </script>
</body>
</html>"###
    )
}

/// The Content Security Policy of the docs page: its own nonce, the one
/// pinned Scalar file, styles Scalar inserts with the nonce, the same-origin
/// OpenAPI document, and nothing else. No `'unsafe-inline'` or
/// `'unsafe-eval'`: Scalar 1.67.0 evaluates no strings and reads the nonce
/// from the `csp-nonce` meta element for its style sheets. Its toast
/// notifications insert one style element without a nonce, which this
/// policy blocks; the reference itself is unaffected.
fn docs_policy(nonce: &CspNonce) -> String {
    let nonce = nonce.as_str();
    format!(
        "default-src 'none'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; \
         form-action 'self'; script-src 'nonce-{nonce}' {SCALAR_SCRIPT}; \
         style-src 'self' 'nonce-{nonce}'; img-src 'self' data:; font-src 'self'; \
         connect-src 'self'; worker-src 'self' blob:"
    )
}

/// `GET /docs`: the page with the request's CSP nonce (a fresh one without a
/// security layer in front) and its own policy, which the security baseline
/// keeps because it never replaces a policy a response already has.
async fn docs_page(
    State(openapi_url): State<&'static str>,
    nonce: Option<Extension<CspNonce>>,
) -> Response {
    let nonce = nonce.map_or_else(CspNonce::generate, |Extension(nonce)| nonce);
    let mut response = Html(render_page(openapi_url, Some(&nonce))).into_response();
    if let Ok(policy) = HeaderValue::from_str(&docs_policy(&nonce)) {
        response
            .headers_mut()
            .insert(header::CONTENT_SECURITY_POLICY, policy);
    }
    response
}

/// Returns an Axum `Router` mounting the interactive Scalar API documentation at `/docs` and `/openapi.json`.
///
/// `/docs` answers with a Content Security Policy of its own that admits only
/// what the Scalar page needs (the request's nonce and the pinned, integrity
/// checked Scalar file), so the page works behind the strict default policy
/// of the security baseline, in every environment.
pub fn scalar_docs_router(openapi_url: &'static str) -> Router {
    Router::new()
        .route("/docs", get(docs_page))
        .route(
            "/openapi.json",
            get(|| openapi_file_response("openapi.json")),
        )
        .with_state(openapi_url)
}

async fn openapi_file_response(path: &'static str) -> Response {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(parsed) => Json(parsed).into_response(),
            Err(error) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "OpenAPI document is malformed",
                    "detail": error.to_string()
                })),
            )
                .into_response(),
        },
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "OpenAPI document is unavailable",
                "detail": error.to_string()
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_scalar_docs_endpoint() {
        let app = scalar_docs_router("/openapi.json");

        let req = Request::builder().uri("/docs").body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn scalar_document_escapes_the_spec_url_and_pins_the_remote_asset() {
        let html = render_scalar_html("javascript:alert(1)\" onload=\"alert(2)");
        assert!(html.contains(&format!(
            "<script src=\"{SCALAR_SCRIPT}\" integrity=\"{SCALAR_INTEGRITY}\" crossorigin=\"anonymous\"></script>"
        )));
        assert!(!html.contains("nonce"));
        assert!(html.contains("data-configuration=\"{&quot;url&quot;:&quot;javascript:alert(1)\\&quot; onload=\\&quot;alert(2)&quot;,&quot;withDefaultFonts&quot;:false}\""));
        assert!(html.contains("&quot; onload=&quot;"));
        assert!(!html.contains("link.href"));
        assert!(!html.contains("data-url=\"javascript:alert(1)\" onload="));
    }

    /// The page's CSP header and the nonce its `csp-nonce` meta element
    /// carries, after the request went through `app`.
    async fn docs_through(app: Router) -> (axum::http::HeaderMap, String, String) {
        let response = app
            .oneshot(Request::get("/docs").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        let nonce = html
            .split("<meta property=\"csp-nonce\" content=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap()
            .to_owned();
        (headers, html, nonce)
    }

    fn expected_policy(nonce: &str) -> String {
        format!(
            "default-src 'none'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; \
             form-action 'self'; script-src 'nonce-{nonce}' \
             https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.67.0/dist/browser/standalone.js; \
             style-src 'self' 'nonce-{nonce}'; img-src 'self' data:; font-src 'self'; \
             connect-src 'self'; worker-src 'self' blob:"
        )
    }

    #[tokio::test]
    async fn docs_keep_their_own_policy_under_the_default_production_stack() {
        for environment in [
            crate::config::Environment::Production,
            crate::config::Environment::Development,
        ] {
            let app = crate::security::apply_security_baseline(
                scalar_docs_router("/openapi.json"),
                crate::config::SecurityConfig::default(),
                environment,
            )
            .unwrap();
            let (headers, html, nonce) = docs_through(app).await;
            let policy = headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
            assert_eq!(policy, expected_policy(&nonce), "{environment:?}");
            assert!(!policy.contains("unsafe-inline") && !policy.contains("unsafe-eval"));
            // The rest of the baseline still applies.
            assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
            assert_eq!(headers[header::CACHE_CONTROL], "no-store");
            // The inline style and every script carry the request's nonce.
            let attribute = format!("nonce=\"{nonce}\"");
            assert_eq!(html.matches(&attribute).count(), 4, "{html}");
            assert!(html.contains(&format!(
                "<script src=\"{SCALAR_SCRIPT}\" integrity=\"{SCALAR_INTEGRITY}\" crossorigin=\"anonymous\" {attribute}></script>"
            )));
        }
    }

    #[tokio::test]
    async fn docs_mint_a_nonce_without_a_security_layer() {
        let (headers, html, nonce) = docs_through(scalar_docs_router("/openapi.json")).await;
        assert_eq!(nonce.len(), 24);
        assert_eq!(
            headers[header::CONTENT_SECURITY_POLICY].to_str().unwrap(),
            expected_policy(&nonce)
        );
        assert!(html.contains(&format!("<style nonce=\"{nonce}\">")));
    }

    #[tokio::test]
    async fn missing_openapi_document_fails_closed() {
        let response = openapi_file_response("definitely-missing-rullst-openapi.json").await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
