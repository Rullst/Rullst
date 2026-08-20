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
    println!("📊 Starting Rullst Studio on http://127.0.0.1:5555...");
    let app = axum::Router::new()
        .route("/", axum::routing::get(studio_home_handler))
        .route("/data", axum::routing::get(studio_data_handler))
        .route("/ai", axum::routing::get(studio_ai_handler))
        .route("/security", axum::routing::get(studio_security_handler))
        .route("/telemetry", axum::routing::get(studio_telemetry_handler))
        .route("/capital", axum::routing::get(studio_capital_handler))
        .route("/traces", axum::routing::get(studio_traces_handler))
        .route(
            "/_studio/api/migrations/run",
            axum::routing::post(handle_run_migrations),
        )
        .route(
            "/_studio/api/migrations/rollback",
            axum::routing::post(handle_rollback_migrations),
        )
        .route(
            "/_studio/api/seeders/run",
            axum::routing::post(handle_run_seeders),
        );

    if let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:5555").await {
        let _ = axum::serve(listener, app).await;
    }
}

async fn handle_run_migrations() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "migrate".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Migrations executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Migration error: {}", e),
        }),
    }
}

async fn handle_rollback_migrations() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "migrate:rollback".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Rollback executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Rollback error: {}", e),
        }),
    }
}

async fn handle_run_seeders() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "db:seed".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Seeders executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Seeder error: {}", e),
        }),
    }
}
