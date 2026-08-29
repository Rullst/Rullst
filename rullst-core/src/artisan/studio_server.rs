//! Studio Control Center local web server and API endpoints.

use crate::artisan::studio_views::{
    studio_ai_handler, studio_capital_handler, studio_data_handler, studio_home_handler,
    studio_security_handler, studio_telemetry_handler, studio_traces_handler,
};

#[derive(serde::Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

#[cfg_attr(mutants, mutants::skip)]
pub(crate) async fn start_studio_server() {
    println!("📊 Starting Rullst Studio on http://127.0.0.1:5555/studio...");
    let app = axum::Router::new()
        .route(
            "/",
            axum::routing::get(|| async { axum::response::Redirect::permanent("/studio") }),
        )
        .route("/studio", axum::routing::get(studio_home_handler))
        .route("/studio/data", axum::routing::get(studio_data_handler))
        .route("/studio/ai", axum::routing::get(studio_ai_handler))
        .route(
            "/studio/security",
            axum::routing::get(studio_security_handler),
        )
        .route(
            "/studio/telemetry",
            axum::routing::get(studio_telemetry_handler),
        )
        .route(
            "/studio/capital",
            axum::routing::get(studio_capital_handler),
        )
        .route("/studio/traces", axum::routing::get(studio_traces_handler))
        .route(
            "/studio/api/migrations/run",
            axum::routing::post(handle_run_migrations),
        )
        .route(
            "/studio/api/migrations/rollback",
            axum::routing::post(handle_rollback_migrations),
        )
        .route(
            "/studio/api/seeders/run",
            axum::routing::post(handle_run_seeders),
        );

    if let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:5555").await {
        let _ = axum::serve(listener, app).await;
    }
}

pub(crate) async fn handle_run_migrations() -> impl axum::response::IntoResponse {
    unavailable_registry("run migrations")
}

pub(crate) async fn handle_rollback_migrations() -> impl axum::response::IntoResponse {
    unavailable_registry("roll back migrations")
}

pub(crate) async fn handle_run_seeders() -> impl axum::response::IntoResponse {
    unavailable_registry("run seeders")
}

fn unavailable_registry(operation: &str) -> (axum::http::StatusCode, axum::Json<ApiResponse>) {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(ApiResponse {
            success: false,
            message: format!(
                "Studio cannot {operation} without an explicitly supplied application registry; use the project CLI"
            ),
        }),
    )
}
