//! Unit tests for Artisan CLI argument translation and Studio endpoints.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::artisan::runner::{ArtisanError, run_artisan_command, translate_artisan_args};
use crate::artisan::studio_server::{
    handle_rollback_migrations, handle_run_migrations, handle_run_seeders,
};
use crate::artisan::studio_views::{
    is_ai_configured, studio_ai_handler, studio_capital_handler, studio_data_handler,
    studio_home_handler, studio_security_handler, studio_telemetry_handler, studio_traces_handler,
};
use crate::server::ServerError;
use crate::server::database_url::resolve_project_database_url;

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

/// A process environment containing at most `DATABASE_URL`.
fn environment(
    database_url: Option<&'static str>,
) -> impl Fn(&str) -> Result<Option<String>, ServerError> {
    move |name| {
        Ok(database_url
            .filter(|_| name == "DATABASE_URL")
            .map(str::to_string))
    }
}

/// A unique project directory under the system temporary directory, removed on drop.
struct ProjectDir(std::path::PathBuf);

impl ProjectDir {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ProjectDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn project(dotenv: Option<&str>, rullst_toml: Option<&str>) -> ProjectDir {
    let project =
        ProjectDir(std::env::temp_dir().join(format!("rullst-artisan-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir_all(project.path()).expect("temporary project");
    if let Some(dotenv) = dotenv {
        std::fs::write(project.path().join(".env"), dotenv).expect(".env");
    }
    if let Some(rullst_toml) = rullst_toml {
        std::fs::write(project.path().join("Rullst.toml"), rullst_toml).expect("Rullst.toml");
    }
    project
}

#[tokio::test]
async fn artisan_database_url_uses_server_precedence() {
    let project = project(
        Some("DATABASE_URL=sqlite://stale-dev.db\n"),
        Some("[database]\nurl = \"sqlite://from-toml.db\"\n"),
    );
    let resolve = |explicit: Option<&'static str>, database_url: Option<&'static str>| {
        resolve_project_database_url(project.path(), explicit, environment(database_url))
    };

    // A deployment's process environment wins over a stale `.env`.
    assert_eq!(
        resolve(None, Some("postgres://prod"))
            .await
            .unwrap()
            .as_deref(),
        Some("postgres://prod")
    );
    // `.env` supplies the URL only when the process environment has none.
    assert_eq!(
        resolve(None, None).await.unwrap().as_deref(),
        Some("sqlite://stale-dev.db")
    );
    // A `Server::with_db` value wins over every file and variable.
    assert_eq!(
        resolve(Some("postgres://explicit"), Some("postgres://prod"))
            .await
            .unwrap()
            .as_deref(),
        Some("postgres://explicit")
    );

    let toml_only = self::project(None, Some("[database]\nurl = \"sqlite://from-toml.db\"\n"));
    assert_eq!(
        resolve_project_database_url(toml_only.path(), None, environment(None))
            .await
            .unwrap()
            .as_deref(),
        Some("sqlite://from-toml.db")
    );
}

#[tokio::test]
async fn artisan_database_url_reads_only_the_database_table_with_a_toml_parser() {
    let url = "postgres://app:secret@db/app?sslmode=require&application_name=rullst";
    let project = project(
        None,
        Some(&format!(
            "[database]\nurl = \"{url}\"\n\n[cache]\nurl = \"redis://127.0.0.1/\"\n"
        )),
    );
    assert_eq!(
        resolve_project_database_url(project.path(), None, environment(None))
            .await
            .unwrap()
            .as_deref(),
        Some(url)
    );

    // Neither a `url` key in another table nor a stray SQLite file selects a database.
    let unrelated = self::project(None, Some("[cache]\nurl = \"redis://127.0.0.1/\"\n"));
    std::fs::write(unrelated.path().join("rullst.db"), b"").unwrap();
    assert_eq!(
        resolve_project_database_url(unrelated.path(), None, environment(None))
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn artisan_database_url_errors_never_echo_configuration_content() {
    let canary = "artisan-secret-canary";
    for (dotenv, rullst_toml) in [
        (
            Some(format!("DATABASE_URL=\"postgres://owner:{canary}@db\n")),
            None,
        ),
        (
            None,
            Some(format!(
                "[database]\nurl = \"postgres://owner:{canary}@db\n"
            )),
        ),
    ] {
        let project = project(dotenv.as_deref(), rullst_toml.as_deref());
        let error = resolve_project_database_url(project.path(), None, environment(None))
            .await
            .expect_err("malformed configuration must fail");
        assert!(matches!(error, ServerError::Configuration(_)));
        assert!(!format!("{error} {error:?}").contains(canary));
    }
}

#[tokio::test]
async fn artisan_command_fails_without_a_configured_database() {
    let project = project(None, None);
    std::fs::write(project.path().join("rullst.db"), b"").unwrap();
    for command in ["db:migrate", "db:rollback", "db:status", "db:seed"] {
        let args = translate_artisan_args(&["app".to_string(), command.to_string()]).unwrap();
        let error = run_artisan_command(
            command,
            &args,
            Some((vec![], vec![])),
            None,
            project.path(),
            environment(None),
        )
        .await
        .expect_err("a database command needs a configured database");
        assert!(matches!(&error, ArtisanError::DatabaseNotConfigured(name) if name == command));
        assert!(error.to_string().contains("DATABASE_URL"));
    }
    assert!(!project.path().join("db.sqlite").exists());
}

#[tokio::test]
async fn artisan_command_propagates_configuration_and_database_failures() {
    let canary = "artisan-command-canary";
    let malformed = project(
        Some(&format!("DATABASE_URL=\"postgres://owner:{canary}@db\n")),
        None,
    );
    let args = translate_artisan_args(&["app".to_string(), "db:migrate".to_string()]).unwrap();
    let error = run_artisan_command(
        "db:migrate",
        &args,
        Some((vec![], vec![])),
        None,
        malformed.path(),
        environment(None),
    )
    .await
    .expect_err("malformed .env must fail");
    assert!(matches!(error, ArtisanError::Configuration(_)));
    assert!(!error.to_string().contains(canary));

    // `Orm::init` rejects the unconfigured placeholder before connecting.
    let empty = project(None, None);
    let error = run_artisan_command(
        "db:migrate",
        &args,
        Some((vec![], vec![])),
        None,
        empty.path(),
        environment(Some("postgres://[your-database-id]/app")),
    )
    .await
    .expect_err("an initialization failure must not be discarded");
    assert!(matches!(error, ArtisanError::Database(_)));
    assert!(rullst_orm::Orm::try_pool().is_err());
}

#[tokio::test]
async fn server_intercepted_database_commands_fail_without_a_registry() {
    let project = project(None, None);
    for command in ["db:migrate", "db:rollback", "db:status", "db:seed"] {
        let args = translate_artisan_args(&["app".to_string(), command.to_string()]).unwrap();
        // A configured database must not turn an empty registry into success.
        let error = run_artisan_command(
            command,
            &args,
            None,
            Some("sqlite::memory:"),
            project.path(),
            environment(None),
        )
        .await
        .expect_err("Server::run has no migration or seeder registry");
        assert!(matches!(&error, ArtisanError::RegistryMissing(name) if name == command));
        assert!(error.to_string().contains("rullst::artisan!"));
    }
}
