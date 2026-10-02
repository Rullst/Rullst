//! Database migrations proposed by the assistant run only for development
//! or test projects.
//!
//! The environment is resolved with the `Server` precedence
//! (`rullst_core::server::ProjectSettings::environment`): the process
//! `RULLST_ENV`, then the process `APP_ENV`, then `RULLST_ENV` or `APP_ENV`
//! from the project's `.env` (which never overrides the process), then
//! `[app].env` in `Rullst.toml`. Values never leave this function. An
//! unreadable, non-Unicode or unknown setting refuses.

use rullst_core::config::Environment;
use std::path::Path;

const MAX_FILE_BYTES: u64 = 256 * 1024;

fn bounded_text(path: &Path) -> Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES => Err(format!(
            "{} is not a regular file of at most 256 KiB",
            path.file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        )),
        Ok(_) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|_| "a project settings file could not be read".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("a project settings file could not be inspected".to_string()),
    }
}

/// `RULLST_ENV` and `APP_ENV` from the project's `.env`.
fn dotenv_selectors(root: &Path) -> Result<(Option<String>, Option<String>), String> {
    let Some(text) = bounded_text(&root.join(".env"))? else {
        return Ok((None, None));
    };
    let (mut rullst, mut app) = (None, None);
    for entry in dotenvy::from_read_iter(text.as_bytes()) {
        let (key, value) = entry.map_err(|_| ".env could not be parsed".to_string())?;
        match key.as_str() {
            "RULLST_ENV" => rullst = Some(value),
            "APP_ENV" => app = Some(value),
            _ => {}
        }
    }
    Ok((rullst, app))
}

fn configured_environment(root: &Path) -> Result<Option<String>, String> {
    let Some(text) = bounded_text(&root.join("Rullst.toml"))? else {
        return Ok(None);
    };
    let value: toml::Value =
        toml::from_str(&text).map_err(|_| "Rullst.toml could not be parsed".to_string())?;
    Ok(value
        .get("app")
        .and_then(|app| app.get("env"))
        .and_then(toml::Value::as_str)
        .map(str::to_string))
}

/// Resolves the project environment from explicit sources (for tests).
pub(super) fn resolve(
    process: impl Fn(&str) -> Option<String>,
    root: &Path,
) -> Result<Environment, String> {
    let (dotenv_rullst, dotenv_app) = dotenv_selectors(root)?;
    let configured = configured_environment(root)?;
    // Both process variables outrank `.env`, as in `Server`; a `.env`
    // `RULLST_ENV=development` must not hide a process `APP_ENV=production`.
    let fallback = dotenv_rullst.or(dotenv_app).or(configured);
    Environment::resolve(
        process("RULLST_ENV").as_deref(),
        process("APP_ENV").as_deref(),
        fallback.as_deref(),
    )
    .map_err(|_| "the project environment name is not recognized".to_string())
}

/// A process variable; a non-Unicode value is kept as an unrecognized name
/// instead of being skipped, so it refuses rather than falling through.
fn process_variable(name: &str) -> Option<String> {
    std::env::var_os(name).map(|value| {
        value
            .into_string()
            .unwrap_or_else(|_| char::REPLACEMENT_CHARACTER.to_string())
    })
}

/// `Ok` only for a development or test project.
pub(super) fn ensure_migration_allowed(root: &Path) -> Result<(), String> {
    match resolve(process_variable, root)? {
        Environment::Development | Environment::Test => Ok(()),
        environment => Err(format!(
            "db:migrate is refused in the {environment} environment; run it yourself after review"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn environment_follows_the_application_precedence() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        assert_eq!(resolve(none, root).unwrap(), Environment::Development);

        fs::write(root.join("Rullst.toml"), "[app]\nenv = \"staging\"\n").unwrap();
        assert_eq!(resolve(none, root).unwrap(), Environment::Staging);

        fs::write(root.join(".env"), "APP_ENV=test\nSECRET=\"value\"\n").unwrap();
        assert_eq!(resolve(none, root).unwrap(), Environment::Test);

        let process = |name: &str| (name == "RULLST_ENV").then(|| "prod".to_string());
        assert_eq!(resolve(process, root).unwrap(), Environment::Production);
    }

    #[test]
    fn process_variables_outrank_every_dotenv_value() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        // The scaffolded `.env` of every new project.
        fs::write(root.join(".env"), "RULLST_ENV=development\n").unwrap();
        let process = |name: &str| (name == "APP_ENV").then(|| "production".to_string());
        assert_eq!(resolve(process, root).unwrap(), Environment::Production);

        fs::write(root.join(".env"), "RULLST_ENV=development\nAPP_ENV=test\n").unwrap();
        assert_eq!(resolve(none, root).unwrap(), Environment::Development);
        let process = |name: &str| (name == "APP_ENV").then(|| "staging".to_string());
        assert_eq!(resolve(process, root).unwrap(), Environment::Staging);

        let unreadable = |name: &str| (name == "APP_ENV").then(|| "\u{fffd}".to_string());
        assert!(resolve(unreadable, root).is_err());
    }

    /// The same project files resolve as in `Server` when the process sets
    /// neither variable (the test does not change the process environment).
    #[tokio::test]
    async fn project_files_resolve_like_the_server() {
        if std::env::var_os("RULLST_ENV").is_some() || std::env::var_os("APP_ENV").is_some() {
            return;
        }
        let cases = [
            ("", None),
            ("APP_ENV=staging\n", None),
            ("RULLST_ENV=development\nAPP_ENV=production\n", None),
            ("APP_ENV=test\n", Some("production")),
            ("", Some("staging")),
        ];
        for (dotenv, configured) in cases {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path();
            fs::write(root.join(".env"), dotenv).unwrap();
            if let Some(configured) = configured {
                fs::write(
                    root.join("Rullst.toml"),
                    format!("[app]\nenv = \"{configured}\"\n"),
                )
                .unwrap();
            }
            let server = rullst_core::server::ProjectSettings::load(root)
                .await
                .unwrap()
                .environment(configured)
                .unwrap();
            assert_eq!(
                resolve(none, root).unwrap(),
                server,
                "{dotenv:?} {configured:?}"
            );
        }
    }

    #[test]
    fn production_staging_and_unknown_settings_refuse() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        for value in ["production", "staging", "mystery"] {
            fs::write(root.join(".env"), format!("RULLST_ENV={value}\n")).unwrap();
            let error = resolve(none, root)
                .map_err(|error| error.to_string())
                .and_then(|environment| match environment {
                    Environment::Development | Environment::Test => Ok(()),
                    other => Err(other.to_string()),
                });
            assert!(error.is_err(), "{value}");
        }
        fs::write(root.join(".env"), "BROKEN LINE WITHOUT EQUALS\n").unwrap();
        assert!(resolve(none, root).is_err());
    }
}
