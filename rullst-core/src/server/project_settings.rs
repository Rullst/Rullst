//! Project settings with the `Server` precedence, for first-party crates.
//!
//! A setting comes from the process environment first and then from the
//! project's `.env`, which never overrides the environment, as `Server`
//! resolves `DATABASE_URL`, `PORT` and the runtime environment. These are
//! hidden support APIs for first-party crates such as `rullst-mail`, not a
//! stable extension point.

use super::ServerError;
use super::builder::read_optional_environment_variable;
use super::database_url::load_dotenv_file;
use crate::config::{ConfigError, Environment};
use std::collections::HashMap;
use std::path::Path;

/// The `.env` values of one project directory, read without changing the
/// process environment. `Debug` never shows them.
#[doc(hidden)]
#[derive(Clone, Default)]
pub struct ProjectSettings {
    dotenv: HashMap<String, String>,
}

impl std::fmt::Debug for ProjectSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProjectSettings")
            .field("dotenv_entries", &self.dotenv.len())
            .finish()
    }
}

impl ProjectSettings {
    /// Reads `project_dir/.env`; a missing file yields no values. Errors name
    /// at most the failing entry number, never file content.
    pub async fn load(project_dir: &Path) -> Result<Self, ServerError> {
        Ok(Self {
            dotenv: load_dotenv_file(&project_dir.join(".env")).await?,
        })
    }

    /// Returns `name` from the process environment, else from `.env`.
    pub fn get(&self, name: &str) -> Result<Option<String>, ServerError> {
        Ok(read_optional_environment_variable(name)?.or_else(|| self.dotenv.get(name).cloned()))
    }

    /// Resolves the runtime environment exactly as `Server` does: the process
    /// `RULLST_ENV`, then the process `APP_ENV`, then `RULLST_ENV` or `APP_ENV`
    /// from `.env`, then `configured` (`[app].env` from `Rullst.toml`).
    pub fn environment(&self, configured: Option<&str>) -> Result<Environment, ServerError> {
        resolve_environment(&self.dotenv, configured)
    }
}

/// Reads one setting for the project in the working directory: the process
/// environment first, then `./.env`, which is read only when needed.
#[doc(hidden)]
pub async fn read_project_setting(name: &str) -> Result<Option<String>, ServerError> {
    if let Some(value) = read_optional_environment_variable(name)? {
        return Ok(Some(value));
    }
    ProjectSettings::load(Path::new(".")).await?.get(name)
}

/// The `Server` environment precedence over an already loaded `.env`. An
/// invalid name is reported without echoing it, since it may come from `.env`.
pub(super) fn resolve_environment(
    dotenv: &HashMap<String, String>,
    configured: Option<&str>,
) -> Result<Environment, ServerError> {
    let rullst_env = read_optional_environment_variable("RULLST_ENV")?;
    let app_env = read_optional_environment_variable("APP_ENV")?;
    let fallback = dotenv
        .get("RULLST_ENV")
        .or_else(|| dotenv.get("APP_ENV"))
        .map(String::as_str)
        .or(configured);
    Environment::resolve(rullst_env.as_deref(), app_env.as_deref(), fallback).map_err(|error| {
        ServerError::Configuration(match error {
            ConfigError::InvalidEnvironment(_) => {
                "RULLST_ENV, APP_ENV or [app].env is not a valid Rullst environment".to_string()
            }
            other => other.to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// Restores the listed process variables when dropped.
    struct VariableGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

    impl VariableGuard {
        fn clear(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect();
            for key in keys {
                unsafe { std::env::remove_var(key) };
            }
            Self(saved)
        }

        fn set(&self, key: &'static str, value: &str) {
            unsafe { std::env::set_var(key, value) };
        }

        fn remove(&self, key: &'static str) {
            unsafe { std::env::remove_var(key) };
        }
    }

    impl Drop for VariableGuard {
        fn drop(&mut self) {
            for (key, value) in &self.0 {
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    fn project(dotenv: &str) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join(".env"), dotenv).unwrap();
        directory
    }

    #[tokio::test]
    async fn dotenv_values_fill_in_but_never_override_the_process_environment() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let variables =
            VariableGuard::clear(&["RULLST_PROJECT_SETTING_A", "RULLST_PROJECT_SETTING_B"]);
        let directory = project(
            "RULLST_PROJECT_SETTING_A=from-dotenv\nRULLST_PROJECT_SETTING_B=\"quoted value\"\n",
        );
        let settings = ProjectSettings::load(directory.path()).await.unwrap();

        assert_eq!(
            settings.get("RULLST_PROJECT_SETTING_A").unwrap().as_deref(),
            Some("from-dotenv")
        );
        assert_eq!(
            settings.get("RULLST_PROJECT_SETTING_B").unwrap().as_deref(),
            Some("quoted value")
        );
        assert_eq!(
            settings.get("RULLST_PROJECT_SETTING_MISSING").unwrap(),
            None
        );
        variables.set("RULLST_PROJECT_SETTING_A", "from-process");
        assert_eq!(
            settings.get("RULLST_PROJECT_SETTING_A").unwrap().as_deref(),
            Some("from-process")
        );
        let debug = format!("{settings:?}");
        assert!(!debug.contains("from-dotenv") && !debug.contains("quoted value"));

        let missing = tempfile::tempdir().unwrap();
        let empty = ProjectSettings::load(missing.path()).await.unwrap();
        assert_eq!(empty.get("RULLST_PROJECT_SETTING_B").unwrap(), None);

        // The working-directory helper prefers the process environment and
        // never needs `.env` for a variable that is set.
        assert_eq!(
            read_project_setting("RULLST_PROJECT_SETTING_A")
                .await
                .unwrap()
                .as_deref(),
            Some("from-process")
        );
    }

    #[tokio::test]
    async fn malformed_dotenv_fails_without_echoing_its_content() {
        let directory = project("SAFE=1\nSECRET_MARKER_KEY='unterminated-secret-marker\n");
        let error = ProjectSettings::load(directory.path()).await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains(".env"));
        assert!(!message.contains("SECRET_MARKER"));
        assert!(!message.contains("secret-marker"));
    }

    #[tokio::test]
    async fn environment_follows_the_server_precedence_without_echoing_values() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let variables = VariableGuard::clear(&["RULLST_ENV", "APP_ENV"]);
        let directory = project("APP_ENV=production\n");
        let settings = ProjectSettings::load(directory.path()).await.unwrap();

        assert_eq!(
            settings.environment(Some("test")).unwrap(),
            Environment::Production
        );
        assert_eq!(
            ProjectSettings::default()
                .environment(Some("test"))
                .unwrap(),
            Environment::Test
        );
        assert_eq!(
            ProjectSettings::default().environment(None).unwrap(),
            Environment::Development
        );
        variables.set("APP_ENV", "staging");
        assert_eq!(settings.environment(None).unwrap(), Environment::Staging);
        variables.set("RULLST_ENV", "development");
        assert_eq!(
            settings.environment(None).unwrap(),
            Environment::Development
        );

        variables.remove("RULLST_ENV");
        variables.remove("APP_ENV");
        let invalid = project("RULLST_ENV=private-marker\n");
        let invalid = ProjectSettings::load(invalid.path()).await.unwrap();
        let error = invalid.environment(None).unwrap_err().to_string();
        assert!(error.contains("not a valid Rullst environment"));
        assert!(!error.contains("private-marker"));
    }
}
