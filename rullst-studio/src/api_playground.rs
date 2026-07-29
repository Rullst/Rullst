use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn router(openapi: OpenApi) -> Router {
    SwaggerUi::new("/")
        .url("/api-docs/openapi.json", openapi)
        .into()
}
