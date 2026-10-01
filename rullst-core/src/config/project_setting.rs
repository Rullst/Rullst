//! Application settings with the precedence `Server` applies to its own:
//! the process environment first, then the project's `.env`.

use super::{ConfigError, read_environment_variable};
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
/// Nexus for its administrator credentials. Unpublished v13 API.
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

fn project_setting_in(project_dir: &Path, name: &str) -> Result<Option<String>, ConfigError> {
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
}
