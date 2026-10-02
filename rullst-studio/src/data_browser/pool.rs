//! Studio database pool selection.
//!
//! Studio uses the process-wide ORM pool that `Server`, Artisan or an explicit
//! `Orm::init` initialized. When none exists yet, it initializes one from the
//! process `DATABASE_URL` or `[database].url` parsed from `./Rullst.toml`, and
//! never invents a SQLite fallback.

use rullst_core::config::RullstConfig;
use std::ffi::OsString;
use std::path::Path;

const DATABASE_NOT_CONFIGURED: &str = "No database is configured for Rullst Studio; set DATABASE_URL or [database].url in Rullst.toml";
const DATABASE_CONFIGURATION_INVALID: &str =
    "Rullst Studio could not read the database configuration";
const DATABASE_INITIALIZATION_FAILED: &str =
    "Rullst Studio could not initialize the configured database";

/// Serializes Studio's own pool initialization so that concurrent requests
/// connect once.
static INITIALIZATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Legacy URL helper kept for API compatibility.
///
/// Studio no longer uses this function. It does not parse `Rullst.toml` as
/// TOML and falls back to `sqlite://db.sqlite`. Let `Server` or Artisan
/// initialize the ORM pool instead of passing this value to `Orm::init`.
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
/// When no pool exists, Studio initializes it once from the current directory:
/// the process `DATABASE_URL`, then `[database].url` parsed from
/// `./Rullst.toml`. `Server` and Artisan, which normally initialize the shared
/// pool first, also read `DATABASE_URL` from `./.env`. Without a configured
/// database this returns an error and creates nothing. Error messages never
/// contain configuration file content or the URL.
pub async fn ensure_pool_initialized() -> Result<&'static rullst_core::db::RullstPool, sqlx::Error>
{
    ensure_pool_initialized_for(Path::new("."), |name| std::env::var_os(name)).await
}

async fn ensure_pool_initialized_for(
    project_dir: &Path,
    environment: impl Fn(&str) -> Option<OsString>,
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

/// Resolves the process `DATABASE_URL`, then `[database].url` from
/// `Rullst.toml` in `project_dir`. `None` means that no database is configured.
async fn resolve_studio_database_url(
    project_dir: &Path,
    environment: impl Fn(&str) -> Option<OsString>,
) -> Result<Option<String>, sqlx::Error> {
    let invalid = || sqlx::Error::Configuration(DATABASE_CONFIGURATION_INVALID.into());
    if let Some(value) = environment("DATABASE_URL") {
        return value.into_string().map(Some).map_err(|_| invalid());
    }
    let path = project_dir.join("Rullst.toml");
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(invalid()),
    };
    let config = RullstConfig::from_toml(&content).map_err(|_| invalid())?;
    Ok(config.database.url)
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
mod tests {
    use super::*;

    fn no_environment(_: &str) -> Option<OsString> {
        None
    }

    #[tokio::test]
    async fn resolves_the_environment_before_the_parsed_toml_without_a_fallback() {
        let project = std::env::temp_dir().join(format!(
            "rullst-studio-resolver-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        std::fs::create_dir_all(&project).expect("temporary project");

        // No configuration: nothing is resolved and no SQLite file is created.
        assert_eq!(
            resolve_studio_database_url(&project, no_environment)
                .await
                .expect("unconfigured project"),
            None
        );
        assert!(!project.join("db.sqlite").exists());

        // `url` outside `[database]` is not a database URL, and query
        // parameters after a second `=` are kept.
        std::fs::write(
            project.join("Rullst.toml"),
            "[cache]\nurl = \"redis://cache\"\n\n[database]\nurl = \"postgres://db/app?application_name=x&sslmode=verify-full\"\n",
        )
        .expect("write Rullst.toml");
        assert_eq!(
            resolve_studio_database_url(&project, no_environment)
                .await
                .expect("TOML database URL")
                .as_deref(),
            Some("postgres://db/app?application_name=x&sslmode=verify-full")
        );

        // The process environment wins over the project file.
        let environment =
            |name: &str| (name == "DATABASE_URL").then(|| OsString::from("sqlite://env.sqlite"));
        assert_eq!(
            resolve_studio_database_url(&project, environment)
                .await
                .expect("environment URL")
                .as_deref(),
            Some("sqlite://env.sqlite")
        );

        // A malformed file fails without echoing its content.
        std::fs::write(project.join("Rullst.toml"), "[database\nurl = \"secret\"")
            .expect("write malformed Rullst.toml");
        let error = resolve_studio_database_url(&project, no_environment)
            .await
            .expect_err("malformed configuration");
        assert!(!error.to_string().contains("secret"));

        std::fs::remove_dir_all(&project).expect("remove temporary project");
    }
}
