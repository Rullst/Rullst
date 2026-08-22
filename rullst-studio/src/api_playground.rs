#![cfg_attr(mutants, mutants::skip)]
use axum::Router;
use utoipa::openapi::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn router(openapi: OpenApi) -> Router {
    SwaggerUi::new("/")
        .url("/api-docs/openapi.json", openapi)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_playground_router() {
        let openapi = OpenApi::default();
        let _ = router(openapi);
    }
}
