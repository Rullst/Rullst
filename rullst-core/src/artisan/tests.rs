//! Unit tests for Artisan CLI argument translation and Studio endpoints.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::artisan::runner::translate_artisan_args;
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

    // 7. Traces
    let trace_html = studio_traces_handler().await;
    assert!(trace_html.0.contains("Traces") || trace_html.0.contains("Trace"));
}
