//! Database URL resolution shared by [`Server`](super::Server) and the Artisan runner.
//!
//! Both entry points must select the same database: a migration or rollback
//! run through Artisan otherwise changes a schema the server never uses.

use super::ServerError;
use crate::config::RullstConfig;
use std::collections::HashMap;
use std::path::Path;

/// Resolves the effective database URL with the `Server` precedence.
///
/// The order is an explicit `Server::with_db` value, the process
/// `DATABASE_URL`, `DATABASE_URL` from `.env`, then `[database].url` from
/// `Rullst.toml`. `.env` never overrides the process environment. `None`
/// means that no database is configured.
pub(crate) fn resolve_database_url(
    explicit: Option<&str>,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
    dotenv: &HashMap<String, String>,
    config: &RullstConfig,
) -> Result<Option<String>, ServerError> {
    if let Some(url) = explicit {
        return Ok(Some(url.to_string()));
    }
    if let Some(url) = environment("DATABASE_URL")? {
        return Ok(Some(url));
    }
    if let Some(url) = dotenv.get("DATABASE_URL") {
        return Ok(Some(url.clone()));
    }
    Ok(config.database.url.clone())
}

/// Loads `.env` and `Rullst.toml` from `project_dir` and resolves the
/// database URL exactly as `Server` does for the same directory.
///
/// Re-exported as hidden support API so that first-party tools such as Rullst
/// Studio select the same database as `Server` and Artisan. It is not a stable
/// extension point.
#[cfg(feature = "orm")]
pub async fn resolve_project_database_url(
    project_dir: &Path,
    explicit: Option<&str>,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
) -> Result<Option<String>, ServerError> {
    let dotenv = load_dotenv_file(&project_dir.join(".env")).await?;
    let config = load_config_file(&project_dir.join("Rullst.toml")).await?;
    resolve_database_url(explicit, environment, &dotenv, &config)
}

/// Reads an optional `.env` file without changing the process environment.
/// Parse errors never contain file content.
pub(crate) async fn load_dotenv_file(path: &Path) -> Result<HashMap<String, String>, ServerError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = tokio::fs::read_to_string(path).await?;
    super::builder::parse_dotenv(&content)
}

/// Parses an optional `Rullst.toml` with the real TOML parser. A missing file
/// yields the defaults; parse errors report only a line and column.
pub(crate) async fn load_config_file(path: &Path) -> Result<RullstConfig, ServerError> {
    if !path.exists() {
        return Ok(RullstConfig::new());
    }

    let content = tokio::fs::read_to_string(path).await.map_err(|error| {
        ServerError::Configuration(crate::config::ConfigError::Read(error.to_string()).to_string())
    })?;
    RullstConfig::from_toml(&content).map_err(|error| ServerError::Configuration(error.to_string()))
}
