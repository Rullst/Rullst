//! Same-origin, build-time Studio assets.

use axum::{
    Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

const STUDIO_CSS: &str = include_str!("../assets/studio.css");
const LOGGER_JS: &str = r#"document.addEventListener("DOMContentLoaded",()=>{const target=document.getElementById("studio-request-stream");if(!target||typeof EventSource==="undefined")return;const source=new EventSource("/studio/requests/stream");source.onmessage=(event)=>{const row=document.createElement("div");row.innerHTML=event.data;while(row.lastChild)target.prepend(row.lastChild);};window.addEventListener("beforeunload",()=>source.close(),{once:true});});"#;

pub(crate) fn router() -> Router {
    Router::new()
        .route("/studio.css", get(studio_css))
        .route("/logger.js", get(logger_js))
}

async fn studio_css() -> Response {
    asset_response("text/css; charset=utf-8", STUDIO_CSS)
}

async fn logger_js() -> Response {
    asset_response("application/javascript; charset=utf-8", LOGGER_JS)
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
        for (path, content_type) in [
            ("/studio.css", "text/css; charset=utf-8"),
            ("/logger.js", "application/javascript; charset=utf-8"),
        ] {
            let response = router()
                .oneshot(
                    Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .expect("asset request"),
                )
                .await
                .expect("asset response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers().get(header::CONTENT_TYPE),
                Some(&axum::http::HeaderValue::from_static(content_type)),
            );
        }
        assert!(!STUDIO_CSS.contains("cdn.tailwindcss.com"));
        assert!(!LOGGER_JS.contains("unpkg.com"));
    }
}
