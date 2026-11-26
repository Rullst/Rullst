//! Application settings with the precedence `Server` applies to its own:
//! the process environment first, then the project's `.env`.

use super::{ConfigError, Environment, RullstConfig, read_environment_variable};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;

/// Reads the setting `name` for the application in the current working
/// directory: the process environment first, then `./.env`.
///
/// This is the precedence [`crate::Server`] applies to `DATABASE_URL`, `PORT`
/// and the runtime environment. `.env` never overrides the process environment
/// and is never loaded into it; it is read (synchronously) only when the
/// process environment lacks `name`, and the last entry for `name` wins. An
/// absent setting is `Ok(None)`; a setting assigned an empty value is
/// `Ok(Some(String::new()))`.
///
/// Generated application code uses it for keys such as `BILLING_*`, and Rullst
/// Nexus for its administrator credentials. Added in v13.
///
/// # Errors
/// [`ConfigError::NonUnicodeEnvironmentVariable`] when the process value is
/// not Unicode, [`ConfigError::Read`] when `./.env` exists but cannot be read
/// as UTF-8 text, and [`ConfigError::Parse`] when any of its entries is
/// malformed. Errors name at most the variable and the failing entry number,
/// never `.env` content.
///
/// ```no_run
/// let api_key = rullst_core::config::project_setting("BILLING_API_KEY")?
///     .unwrap_or_default();
/// # Ok::<(), rullst_core::config::ConfigError>(())
/// ```
pub fn project_setting(name: &str) -> Result<Option<String>, ConfigError> {
    project_setting_in(Path::new("."), name)
}

/// [`project_setting`] for the project in `project_dir`.
pub(crate) fn project_setting_in(
    project_dir: &Path,
    name: &str,
) -> Result<Option<String>, ConfigError> {
    if let Some(value) = read_environment_variable(name)? {
        return Ok(Some(value));
    }
    let content = match std::fs::read_to_string(project_dir.join(".env")) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ConfigError::Read(format!(
                ".env could not be read: {}",
                error.kind()
            )));
        }
    };
    Ok(parse_dotenv_entries(&content)
        .map_err(ConfigError::Parse)?
        .remove(name))
}

/// Resolves the runtime environment of the application in the current working
/// directory exactly as [`crate::Server`] does, without starting one: the
/// process `RULLST_ENV`, then the process `APP_ENV`, then either one from
/// `./.env`, then `env` under `[app]` in `./Rullst.toml`; none set is
/// development.
///
/// Use it where a decision depends on the environment before a `Server`
/// exists, such as when a router is built. [`Environment::detect`] reads the
/// process variables only and so misses an environment set in `.env` or
/// `Rullst.toml`. Added in 13.0.0.
///
/// # Errors
/// [`ConfigError::InvalidEnvironment`] for an unknown name,
/// [`ConfigError::NonUnicodeEnvironmentVariable`], and the read and parse
/// errors of [`project_setting`] and [`RullstConfig::from_toml`]. A caller that
/// guards a staging or production boundary treats any error as production.
///
/// ```no_run
/// let secure = rullst_core::config::project_environment()
///     .map_or(true, |environment| environment.requires_secure_defaults());
/// # let _ = secure;
/// ```
pub fn project_environment() -> Result<Environment, ConfigError> {
    project_environment_in(Path::new("."))
}

/// [`project_environment`] for the project in `project_dir`.
pub(crate) fn project_environment_in(project_dir: &Path) -> Result<Environment, ConfigError> {
    let dotenv = match project_setting_in(project_dir, "RULLST_ENV")? {
        Some(value) => Some(value),
        None => project_setting_in(project_dir, "APP_ENV")?,
    };
    let configured = match dotenv {
        Some(value) => Some(value),
        None => configured_environment(project_dir)?,
    };
    // The process variables come first, as in `Server`; `project_setting_in`
    // returned a process value above only when it also wins here.
    Environment::detect(configured.as_deref())
}

/// `env` under `[app]` in `Rullst.toml`; a missing file sets none.
fn configured_environment(project_dir: &Path) -> Result<Option<String>, ConfigError> {
    match std::fs::read_to_string(project_dir.join("Rullst.toml")) {
        Ok(content) => Ok(RullstConfig::from_toml(&content)?.app.env),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ConfigError::Read(format!(
            "Rullst.toml could not be read: {}",
            error.kind()
        ))),
    }
}

/// Parses dotenv content with errors that never contain file content: dotenvy's
/// own parse error embeds the unparsed remainder, which can include secrets.
pub(crate) fn parse_dotenv_entries(content: &str) -> Result<HashMap<String, String>, String> {
    let mut values = HashMap::new();
    for (index, entry) in dotenvy::from_read_iter(content.as_bytes()).enumerate() {
        let (name, value) = entry.map_err(|error| match error {
            dotenvy::Error::LineParse(..) => format!("invalid .env syntax in entry {}", index + 1),
            dotenvy::Error::Io(error) => format!("failed to read .env: {}", error.kind()),
            _ => "invalid .env file".to_string(),
        })?;
        values.insert(name, value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    const PUBLIC: &str = "RULLST_PUBLIC_PROJECT_SETTING_A";
    const QUOTED: &str = "RULLST_PUBLIC_PROJECT_SETTING_B";

    /// Removes the test variables now and restores them when dropped.
    struct Cleared(Vec<(&'static str, Option<std::ffi::OsString>)>);

    impl Cleared {
        fn new(keys: &[&'static str]) -> Self {
            let saved = keys
                .iter()
                .map(|key| (*key, std::env::var_os(key)))
                .collect();
            for key in keys {
                unsafe { std::env::remove_var(key) };
            }
            Self(saved)
        }
    }

    impl Drop for Cleared {
        fn drop(&mut self) {
            for (key, value) in &self.0 {
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
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
    async fn dotenv_fills_in_but_never_overrides_the_process_environment() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let _cleared = Cleared::new(&[PUBLIC, QUOTED]);
        let directory = project(&format!(
            "{PUBLIC}=first\n{QUOTED}=\"quoted value\"\n{PUBLIC}=from-dotenv\nEMPTY=\n"
        ));
        let read = |name| project_setting_in(directory.path(), name).unwrap();

        assert_eq!(read(PUBLIC).as_deref(), Some("from-dotenv"));
        assert_eq!(read(QUOTED).as_deref(), Some("quoted value"));
        assert_eq!(read("EMPTY").as_deref(), Some(""));
        assert_eq!(read("RULLST_PUBLIC_PROJECT_SETTING_MISSING"), None);

        unsafe { std::env::set_var(PUBLIC, "from-process") };
        assert_eq!(read(PUBLIC).as_deref(), Some("from-process"));
        // The working-directory form also prefers the process environment.
        assert_eq!(
            project_setting(PUBLIC).unwrap().as_deref(),
            Some("from-process")
        );

        let missing = tempfile::tempdir().unwrap();
        assert_eq!(project_setting_in(missing.path(), QUOTED).unwrap(), None);
    }

    #[tokio::test]
    async fn unreadable_or_malformed_dotenv_fails_without_echoing_content() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let _cleared = Cleared::new(&[PUBLIC]);
        let malformed = project("SAFE=1\nSECRET_MARKER_KEY='unterminated-secret-marker\n");
        let error = project_setting_in(malformed.path(), PUBLIC).unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
        let message = error.to_string();
        assert!(message.contains("invalid .env syntax in entry 2"));
        assert!(!message.contains("SECRET_MARKER") && !message.contains("secret-marker"));

        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join(".env"), b"KEY=\xff-binary-marker\n").unwrap();
        let error = project_setting_in(directory.path(), PUBLIC).unwrap_err();
        assert!(matches!(error, ConfigError::Read(_)));
        assert!(!error.to_string().contains("binary-marker"));

        // A set process variable never needs `.env`, even a malformed one.
        unsafe { std::env::set_var(PUBLIC, "from-process") };
        assert_eq!(
            project_setting_in(malformed.path(), PUBLIC)
                .unwrap()
                .as_deref(),
            Some("from-process")
        );
    }

    #[tokio::test]
    async fn project_environment_follows_the_server_precedence() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let _cleared = Cleared::new(&["RULLST_ENV", "APP_ENV"]);
        let directory = tempfile::tempdir().unwrap();
        let write = |name: &str, content: &str| {
            std::fs::write(directory.path().join(name), content).unwrap();
        };
        let resolve = || project_environment_in(directory.path());

        assert_eq!(resolve().unwrap(), Environment::Development);
        write("Rullst.toml", "[app]\nenv = \"staging\"\n");
        assert_eq!(resolve().unwrap(), Environment::Staging);
        write(".env", "APP_ENV=production\n");
        assert_eq!(resolve().unwrap(), Environment::Production);
        write(".env", "APP_ENV=test\nRULLST_ENV=production\n");
        assert_eq!(resolve().unwrap(), Environment::Production);
        unsafe { std::env::set_var("APP_ENV", "development") };
        assert_eq!(resolve().unwrap(), Environment::Development);
        unsafe { std::env::set_var("RULLST_ENV", "production") };
        assert_eq!(resolve().unwrap(), Environment::Production);
        // The working-directory form reads the same process variable first.
        assert_eq!(project_environment().unwrap(), Environment::Production);
    }

    #[tokio::test]
    async fn project_environment_reports_bad_sources_without_their_content() {
        let _lock = crate::server::TEST_ENV_LOCK.lock().await;
        let _cleared = Cleared::new(&["RULLST_ENV", "APP_ENV"]);
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join(".env"), "RULLST_ENV=nonsense\n").unwrap();
        assert!(matches!(
            project_environment_in(directory.path()),
            Err(ConfigError::InvalidEnvironment(_))
        ));
        std::fs::write(directory.path().join(".env"), "").unwrap();
        std::fs::write(
            directory.path().join("Rullst.toml"),
            "[app]\nenv = \"production\"\nSECRET_MARKER = [\n",
        )
        .unwrap();
        let error = project_environment_in(directory.path()).unwrap_err();
        assert!(matches!(error, ConfigError::Parse(_)));
        assert!(!error.to_string().contains("SECRET_MARKER"));

        // A Rullst.toml that exists but is not UTF-8 is an error, not "unset".
        std::fs::write(
            directory.path().join("Rullst.toml"),
            b"[app]\nenv = \"\xff-binary-marker\"\n",
        )
        .unwrap();
        let error = project_environment_in(directory.path()).unwrap_err();
        assert!(matches!(error, ConfigError::Read(_)));
        assert!(!error.to_string().contains("binary-marker"));
    }
}
