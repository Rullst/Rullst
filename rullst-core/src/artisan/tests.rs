//! Unit tests for Artisan CLI argument translation and Studio endpoints.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::artisan::runner::translate_artisan_args;
use crate::artisan::studio_server::is_ai_configured;
use crate::artisan::studio_views::studio_security_handler;

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
async fn test_is_ai_configured_and_security_handler() {
    let _ = is_ai_configured();
    let html_res = studio_security_handler().await;
    assert!(html_res.0.contains("Threat Radar"));
}
