//! Studio database pool selection.
//!
//! Studio uses the process-wide ORM pool. When the host application has not
//! initialized it yet, Studio resolves the project database with the same
//! resolver as `Server` and Artisan and never invents a SQLite fallback.

use rullst_core::server::{ServerError, read_optional_environment_variable};
use std::path::Path;

const DATABASE_NOT_CONFIGURED: &str = "No database is configured for Rullst Studio; set DATABASE_URL in the environment or .env, or [database].url in Rullst.toml";
const DATABASE_INITIALIZATION_FAILED: &str =
    "Rullst Studio could not initialize the configured database";

/// Serializes Studio's own pool initialization so that concurrent requests
/// connect once.
static INITIALIZATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Legacy URL helper kept for API compatibility.
///
/// Studio no longer uses this function. It does not follow the shared
/// `Server`/Artisan precedence (it ignores `./.env` and does not parse
/// `Rullst.toml` as TOML) and falls back to `sqlite://db.sqlite`. Let `Server`
/// or Artisan initialize the ORM pool instead of passing this value to
/// `Orm::init`.
pub fn resolve_db_url(provided: &str) -> String {
    if !provided.trim().is_empty() {
        return provided.trim().to_string();
    }
    if let Ok(env_url) = std::env::var("DATABASE_URL")
        && !env_url.trim().is_empty()
    {
        return env_url.trim().to_string();
    }
    if let Ok(toml_content) = std::fs::read_to_string("Rullst.toml") {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("url =") || trimmed.starts_with("url="))
                && let Some(val) = trimmed.split('=').nth(1)
            {
                let clean = val.trim().trim_matches('"').trim_matches('\'');
                if !clean.is_empty() {
                    return clean.to_string();
                }
            }
        }
    }
    if std::path::Path::new("db.sqlite").exists() {
        return "sqlite://db.sqlite".to_string();
    }
    if std::path::Path::new("rullst.db").exists() {
        return "sqlite://rullst.db".to_string();
    }
    "sqlite://db.sqlite".to_string()
}

/// Returns the process-wide ORM pool.
///
/// When no pool exists, Studio initializes it once from the current directory
/// with the resolver shared by `Server` and Artisan: the process
/// `DATABASE_URL`, then `DATABASE_URL` from `./.env` (never overriding the
/// process environment), then `[database].url` parsed from `./Rullst.toml`.
/// Without a configured database this returns an error and creates nothing.
/// Error messages never contain configuration file content or the URL.
pub async fn ensure_pool_initialized() -> Result<&'static rullst_core::db::RullstPool, sqlx::Error>
{
    ensure_pool_initialized_for(Path::new("."), read_optional_environment_variable).await
}

async fn ensure_pool_initialized_for(
    project_dir: &Path,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
) -> Result<&'static rullst_core::db::RullstPool, sqlx::Error> {
    if let Some(pool) = rullst_core::db::safe_pool() {
        return Ok(pool);
    }
    let _initializing = INITIALIZATION.lock().await;
    if let Some(pool) = rullst_core::db::safe_pool() {
        return Ok(pool);
    }
    let url = resolve_studio_database_url(project_dir, environment)
        .await?
        .ok_or_else(|| sqlx::Error::Configuration(DATABASE_NOT_CONFIGURED.into()))?;
    // The ORM error may describe the connection target, so it is not echoed.
    // If the host application published a pool meanwhile, that pool is used.
    let _ = rullst_orm::Orm::init(&url).await;
    rullst_core::db::safe_pool()
        .ok_or_else(|| sqlx::Error::Configuration(DATABASE_INITIALIZATION_FAILED.into()))
}

/// Resolves the database for `project_dir` exactly as `Server` and Artisan do.
async fn resolve_studio_database_url(
    project_dir: &Path,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
) -> Result<Option<String>, sqlx::Error> {
    rullst_core::server::resolve_project_database_url(project_dir, None, environment)
        .await
        .map_err(|error| sqlx::Error::Configuration(error.to_string().into()))
}

/// Initializes the SQLite pool shared by this crate's unit tests.
///
/// A file outlives the per-test runtimes, whereas an in-memory database
/// disappears with the connection that owns it. Each test drops and recreates
/// its own tables, so the file is reused across runs.
#[cfg(all(
    test,
    not(miri),
    not(any(feature = "strict-postgres", feature = "strict-mysql"))
))]
pub(crate) async fn test_sqlite_pool() -> &'static rullst_core::db::RullstPool {
    let _initializing = INITIALIZATION.lock().await;
    if rullst_core::db::safe_pool().is_none() {
        let path = std::env::temp_dir().join("rullst-studio-unit-tests.sqlite");
        let url = format!(
            "sqlite://{}?mode=rwc",
            path.to_string_lossy().replace('\\', "/")
        );
        rullst_orm::Orm::init(&url)
            .await
            .expect("temporary Studio test database");
    }
    rullst_core::db::safe_pool().expect("Studio test pool")
}

#[cfg(test)]
#[cfg(not(miri))]
mod tests;
