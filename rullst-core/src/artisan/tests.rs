//! Unit tests for Artisan CLI argument translation and Studio endpoints.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::artisan::runner::translate_artisan_args;
use crate::artisan::studio_server::{
    handle_rollback_migrations, handle_run_migrations, handle_run_seeders,
};
use crate::artisan::studio_views::{
    is_ai_configured, studio_ai_handler, studio_capital_handler, studio_data_handler,
    studio_home_handler, studio_security_handler, studio_telemetry_handler, studio_traces_handler,
};

#[test]
fn test_translate_artisan_args_none() {
    // No args
    assert!(translate_artisan_args(&[]).is_none());
    // Only 1 arg (the binary name)
    assert!(translate_artisan_args(&["cargo-rullst".to_string()]).is_none());
    // Non-matching command
    assert!(translate_artisan_args(&["cargo-rullst".to_string(), "run".to_string()]).is_none());
}

#[test]
fn test_translate_artisan_args_translation() {
    let args = vec!["artisan".to_string(), "db:migrate".to_string()];
    let expected = vec!["artisan".to_string(), "migrate".to_string()];
    assert_eq!(translate_artisan_args(&args), Some(expected));

    let args_rollback = vec!["artisan".to_string(), "db:rollback".to_string()];
    let expected_rollback = vec!["artisan".to_string(), "migrate:rollback".to_string()];
    assert_eq!(
        translate_artisan_args(&args_rollback),
        Some(expected_rollback)
    );

    let args_with_extra = vec![
        "artisan".to_string(),
        "db:migrate".to_string(),
        "--force".to_string(),
    ];
    let expected_with_extra = vec![
        "artisan".to_string(),
        "migrate".to_string(),
        "--force".to_string(),
    ];
    assert_eq!(
        translate_artisan_args(&args_with_extra),
        Some(expected_with_extra)
    );
}

#[tokio::test]
async fn test_check_and_run_artisan_noop() {
    // Calling check_and_run_artisan in test execution should return Ok(())
    // because the command line arguments won't match any artisan commands.
    let result = check_and_run_artisan(vec![], vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_all_studio_views_and_api_handlers() {
    let _ = is_ai_configured();

    // 1. Home Control Center
    let home_html = studio_home_handler().await;
    assert!(home_html.0.contains("Control Center"));

    // 2. Data / Database Tools
    let data_html = studio_data_handler().await;
    assert!(data_html.0.contains("Database Tools") || data_html.0.contains("Database"));

    // 3. AI Playground
    let ai_html = studio_ai_handler().await;
    assert!(ai_html.0.contains("AI Playground") || ai_html.0.contains("AI"));

    // 4. Telemetry
    let telem_html = studio_telemetry_handler().await;
    assert!(telem_html.0.contains("Telemetry") || telem_html.0.contains("Radar"));

    // 5. Capital
    let cap_html = studio_capital_handler().await;
    assert!(cap_html.0.contains("Capital") || cap_html.0.contains("Revenue"));

    // 6. Security Threat Radar
    let sec_html = studio_security_handler().await;
    assert!(sec_html.0.contains("Threat Radar") || sec_html.0.contains("Security"));

    // 7. Process-local span records
    let trace_html = studio_traces_handler().await;
    assert!(trace_html.0.contains("Local Span Records"));
    assert!(trace_html.0.contains("SpanCollector"));
}

async fn assert_registry_operation_fails_closed(
    response: impl axum::response::IntoResponse,
    operation: &str,
) {
    let response = response.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    assert_eq!(
        response.headers().get(axum::http::header::CONTENT_TYPE),
        Some(&axum::http::HeaderValue::from_static("application/json"))
    );
    let body = axum::body::to_bytes(response.into_body(), 4 * 1024)
        .await
        .expect("bounded Studio registry error body");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("Studio registry error JSON");
    assert_eq!(payload["success"], false);
    assert!(
        payload["message"]
            .as_str()
            .is_some_and(|message| message.contains(operation))
    );
    assert!(
        payload["message"]
            .as_str()
            .is_some_and(|message| message.contains("explicitly supplied application registry"))
    );
}

#[tokio::test]
async fn studio_mutations_never_claim_success_without_an_application_registry() {
    assert_registry_operation_fails_closed(handle_run_migrations().await, "run migrations").await;
    assert_registry_operation_fails_closed(
        handle_rollback_migrations().await,
        "roll back migrations",
    )
    .await;
    assert_registry_operation_fails_closed(handle_run_seeders().await, "run seeders").await;
}
