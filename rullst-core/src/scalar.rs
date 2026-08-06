//! Interactive Scalar API Documentation Page (`/docs`)
#![cfg(not(target_arch = "wasm32"))]

use axum::{
    Router,
    response::{Html, Json, IntoResponse},
    routing::get,
};
use serde_json::json;

/// Generates the HTML5 Scalar API documentation interface.
pub fn render_scalar_html(openapi_url: &str) -> String {
    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>API Documentation — Rullst Scalar UI</title>
    <link rel="icon" type="image/svg+xml" href="https://rullst.dev/favicon.svg">
    <style>
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
    <script id="api-reference" data-url="{openapi_url}"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
    <script>
        // Fallback for offline development environments
        if (typeof Scalar === 'undefined') {{
            console.warn('Rullst Scalar: CDN unreachable, rendering offline fallback status page.');
            document.body.innerHTML = `
                <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100vh;text-align:center;">
                    <h1 style="color:#38bdf8;">Rullst Interactive API Documentation</h1>
                    <p style="color:#94a3b8;">Scalar UI CDN is offline or unavailable. Your OpenAPI spec is available at: <a href="{openapi_url}" style="color:#38bdf8;">{openapi_url}</a></p>
                </div>
            `;
        }}
    </script>
</body>
</html>"###,
        openapi_url = openapi_url
    )
}

/// Returns an Axum `Router` mounting the interactive Scalar API documentation at `/docs` and `/openapi.json`.
pub fn scalar_docs_router(openapi_url: &'static str) -> Router {
    let html_content = render_scalar_html(openapi_url);
    Router::new()
        .route("/docs", get(move || async move { Html(html_content) }))
        .route("/openapi.json", get(|| async move {
            if let Ok(content) = std::fs::read_to_string("openapi.json") {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    return Json(parsed).into_response();
                }
            }
            Json(json!({
                "openapi": "3.0.0",
                "info": {
                    "title": "Rullst Application API",
                    "description": "Interactive API documentation powered by Rullst & Scalar UI.",
                    "version": "1.0.0"
                },
                "paths": {}
            })).into_response()
        }))
}

#[cfg(test)]
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
}
