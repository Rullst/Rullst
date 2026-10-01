//! Command-line argument translator and Artisan CLI dispatcher.

use crate::artisan::studio_server::start_studio_server;
use crate::server::ServerError;
use crate::server::builder::read_optional_environment_variable;
use crate::server::database_url::resolve_project_database_url;
use rullst_orm::Seeder;
use rullst_orm::schema::{Migration, run_artisan_with_args};
use std::env;
use std::path::Path;

#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn translate_artisan_args(args: &[String]) -> Option<Vec<String>> {
    if args.len() < 2 {
        return None;
    }
    let command = &args[1];
    if command == "db:migrate"
        || command == "db:rollback"
        || command == "db:status"
        || command == "db:seed"
        || command == "studio"
    {
        let mut translated_args = vec![args[0].clone()];
        match command.as_str() {
            "db:migrate" => translated_args.push("migrate".to_string()),
            "db:rollback" => translated_args.push("migrate:rollback".to_string()),
            "db:status" => translated_args.push("status".to_string()),
            "db:seed" => translated_args.push("db:seed".to_string()),
            _ => translated_args.push(command.clone()),
        }

        // Forward any trailing arguments
        if args.len() > 2 {
            translated_args.extend_from_slice(&args[2..]);
        }
        Some(translated_args)
    } else {
        None
    }
}

/// Failures of one intercepted Artisan command. Messages never contain
/// configuration file content.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ArtisanError {
    /// `.env`, `Rullst.toml` or the process environment could not be read.
    #[error("{0}")]
    Configuration(#[from] ServerError),

    /// No database URL is configured for a database command.
    #[error(
        "no database is configured for `{0}`; set DATABASE_URL in the environment or .env, or [database].url in Rullst.toml"
    )]
    DatabaseNotConfigured(String),

    /// The configured database could not be initialized.
    #[error("database initialization failed: {0}")]
    Database(String),

    /// The ORM command itself failed.
    #[error("{0}")]
    Command(String),

    /// The `studio` command could not bind or serve its local port.
    #[error("Rullst Studio could not serve on 127.0.0.1:5555: {0}")]
    Studio(#[source] std::io::Error),

    /// `Server::run` intercepted a `db:*` command, but the application never
    /// supplied its migrations and seeders.
    #[error(
        "`{0}` needs the application's migrations and seeders; call `rullst::artisan!(migrations, seeders)` before `Server::run`"
    )]
    RegistryMissing(String),
}

/// Migrations and seeders an intercepted command runs against. `None` means the
/// command was intercepted by `Server::run`, which has no registry.
pub(crate) type ArtisanRegistry = Option<(Vec<Box<dyn Migration>>, Vec<Box<dyn Seeder>>)>;

/// Intercepts command line database calls (like `db:migrate` or `studio`) before AXUM web server starts.
///
/// The database URL is resolved exactly like [`crate::Server`] does: the
/// process `DATABASE_URL`, then `DATABASE_URL` in `./.env` (never overriding
/// the process environment), then `[database].url` in `Rullst.toml`. A `db:*`
/// command without a configured database fails instead of creating a local
/// SQLite file. When a requested command runs, the process exits with status 0
/// on success and 1 on any failure, including database initialization.
/// Without an Artisan command this returns `Ok(())` and has no side effects.
///
/// `Server::run` also intercepts these commands, but it has no registry: there
/// a `db:*` command exits with status 1 and asks for `rullst::artisan!` instead
/// of reporting success for an empty registry. `studio` still runs.
#[cfg_attr(mutants, mutants::skip)]
pub async fn check_and_run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn Seeder>>,
) -> Result<(), Box<dyn std::error::Error>> {
    intercept_artisan_command(Some((migrations, seeders)), None).await;
    Ok(())
}

/// Runs a requested Artisan command and exits the process, or returns when
/// the arguments contain no Artisan command. `explicit_db_url` carries a
/// `Server::with_db` value so the server and its commands share one database.
#[cfg_attr(mutants, mutants::skip)]
pub(crate) async fn intercept_artisan_command(
    registry: ArtisanRegistry,
    explicit_db_url: Option<&str>,
) {
    let args: Vec<String> = env::args().collect();
    let Some(translated_args) = translate_artisan_args(&args) else {
        return;
    };
    let command = args.get(1).map_or("", String::as_str);

    let result = run_artisan_command(
        command,
        &translated_args,
        registry,
        explicit_db_url,
        Path::new("."),
        read_optional_environment_variable,
    )
    .await;

    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("❌ Error: Executing artisan command failed: {error}");
            std::process::exit(1);
        }
    }
}

/// Resolves the database for `project_dir`, initializes the ORM pool and runs
/// one translated Artisan command. `environment` reads process variables.
/// Without a registry only `studio` runs; `db:*` fails before touching the
/// database.
pub(crate) async fn run_artisan_command(
    command: &str,
    translated_args: &[String],
    registry: ArtisanRegistry,
    explicit_db_url: Option<&str>,
    project_dir: &Path,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
) -> Result<(), ArtisanError> {
    if registry.is_none() && command != "studio" {
        return Err(ArtisanError::RegistryMissing(command.to_string()));
    }

    // Like `Server`, reuse a pool the application initialized explicitly.
    let pool_ready = rullst_orm::Orm::try_pool().is_ok();
    let database_url = if pool_ready {
        None
    } else {
        resolve_project_database_url(project_dir, explicit_db_url, environment).await?
    };

    match database_url {
        Some(url) => rullst_orm::Orm::init(&url)
            .await
            .map_err(|error| ArtisanError::Database(error.to_string()))?,
        None if pool_ready => {}
        None if command == "studio" => {
            eprintln!(
                "⚠️  Rullst Studio: no database is configured; database tools are unavailable."
            );
        }
        None => return Err(ArtisanError::DatabaseNotConfigured(command.to_string())),
    }

    // Migrations and seeders may read other `.env` values. Values already in
    // the process environment always win, matching the URL resolution above.
    let _ = dotenvy::from_path(project_dir.join(".env"));

    if command == "studio" {
        return start_studio_server().await.map_err(ArtisanError::Studio);
    }

    let (migrations, seeders) = registry.unwrap_or_default();
    run_artisan_with_args(translated_args, migrations, seeders)
        .await
        .map_err(|error| ArtisanError::Command(error.to_string()))
}
