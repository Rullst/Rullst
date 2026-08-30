#![cfg_attr(mutants, mutants::skip)]
use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn router(openapi: OpenApi) -> Router {
    SwaggerUi::new("/studio/api")
        .url("/studio/api/openapi.json", openapi)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn api_playground_mounts_canonical_ui_and_document_routes() {
        let openapi = OpenApi::default();
        let app = router(openapi);

        let document = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/studio/api/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(document.status(), axum::http::StatusCode::OK);

        let legacy = app
            .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(legacy.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
