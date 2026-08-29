//! Interactive Scalar API Documentation Page (`/docs`)
#![cfg(not(target_arch = "wasm32"))]

use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
};
use serde_json::json;

/// Generates the HTML5 Scalar API documentation interface.
pub fn render_scalar_html(openapi_url: &str) -> String {
    let openapi_url = crate::html::escape_str(openapi_url);
    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>API Documentation — Rullst Scalar UI</title>
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
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.67.0"></script>
    <script>
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
</html>"###,
        openapi_url = openapi_url
    )
}

/// Returns an Axum `Router` mounting the interactive Scalar API documentation at `/docs` and `/openapi.json`.
pub fn scalar_docs_router(openapi_url: &'static str) -> Router {
    let html_content = render_scalar_html(openapi_url);
    Router::new()
        .route("/docs", get(move || async move { Html(html_content) }))
        .route(
            "/openapi.json",
            get(|| openapi_file_response("openapi.json")),
        )
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
        assert!(html.contains("@scalar/api-reference@1.67.0"));
        assert!(html.contains("&quot; onload=&quot;"));
        assert!(!html.contains("link.href"));
        assert!(!html.contains("data-url=\"javascript:alert(1)\" onload="));
    }

    #[tokio::test]
    async fn missing_openapi_document_fails_closed() {
        let response = openapi_file_response("definitely-missing-rullst-openapi.json").await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
