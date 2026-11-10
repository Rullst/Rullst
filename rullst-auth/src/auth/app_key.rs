//! Application key validation and resolution from the process environment,
//! `.env`, `Rullst.toml` or the development key file.

use crate::error::AuthError;
use std::collections::HashMap;
use std::fs;

const MIN_APP_KEY_BYTES: usize = 32;
const MIN_APP_KEY_ENTROPY_BITS: f64 = 128.0;

/// Parses the application key from a given TOML content string.
///
/// This legacy parser keeps the value before the next `=` for compatibility
/// with existing session keys. Prefer `APP_KEY` for values containing `=`.
pub fn parse_app_key_from_toml(toml_content: &str) -> Option<Vec<u8>> {
    for line in toml_content.lines() {
        let trimmed = line.trim();
        let mut assignment = trimmed.split('=');
        if let (Some(name), Some(val)) = (assignment.next(), assignment.next())
            && matches!(name.trim(), "app_key" | "key")
        {
            return Some(val.trim().trim_matches('"').as_bytes().to_vec());
        }
    }
    None
}

static CACHED_APP_KEY: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
/// Serializes development-key resolution so concurrent first callers agree.
static DEVELOPMENT_KEY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Validates the minimum size and estimated entropy required for application secrets.
pub fn validate_app_key(key: &[u8]) -> Result<(), AuthError> {
    if key.len() < MIN_APP_KEY_BYTES {
        return Err(AuthError::MissingAppKey(format!(
            "APP_KEY must contain at least {MIN_APP_KEY_BYTES} bytes"
        )));
    }

    let normalized = String::from_utf8_lossy(key).trim().to_ascii_lowercase();
    if normalized.starts_with("mock_")
        || matches!(
            normalized.as_str(),
            "changeme"
                | "change_me"
                | "password"
                | "replace_me"
                | "secret"
                | "replace_with_your_32_char_random_key"
                | "change_me_to_a_secure_random_key"
                | "replace_with_a_strong_random_key"
                | "replace-with-at-least-32-random-bytes"
        )
    {
        return Err(AuthError::MissingAppKey(
            "APP_KEY must not use a documented placeholder".to_string(),
        ));
    }

    let mut frequencies = [0usize; 256];
    for byte in key {
        frequencies[usize::from(*byte)] += 1;
    }
    let length = key.len() as f64;
    let estimated_entropy = frequencies
        .iter()
        .filter(|frequency| **frequency > 0)
        .map(|frequency| {
            let probability = *frequency as f64 / length;
            -probability * probability.log2()
        })
        .sum::<f64>()
        * length;

    if estimated_entropy < MIN_APP_KEY_ENTROPY_BITS {
        return Err(AuthError::MissingAppKey(format!(
            "APP_KEY estimated entropy must be at least {MIN_APP_KEY_ENTROPY_BITS:.0} bits"
        )));
    }

    Ok(())
}

fn load_dotenv_values() -> Result<HashMap<String, String>, AuthError> {
    if !std::path::Path::new(".env").exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(".env")
        .map_err(|error| AuthError::General(format!("failed to read .env: {}", error.kind())))?;
    parse_dotenv(&content)
}

/// Parses dotenv content with errors that never contain file content: dotenvy's
/// own parse error embeds the unparsed remainder, which can include secrets.
pub(super) fn parse_dotenv(content: &str) -> Result<HashMap<String, String>, AuthError> {
    let mut values = HashMap::new();
    for (index, entry) in dotenvy::from_read_iter(content.as_bytes()).enumerate() {
        let (name, value) = entry.map_err(|error| {
            AuthError::General(match error {
                dotenvy::Error::LineParse(..) => {
                    format!("invalid .env syntax in entry {}", index + 1)
                }
                dotenvy::Error::Io(error) => format!("failed to read .env: {}", error.kind()),
                _ => "invalid .env file".to_string(),
            })
        })?;
        values.insert(name, value);
    }
    Ok(values)
}

fn read_process_environment(name: &str) -> Result<Option<String>, AuthError> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(AuthError::General(format!("{name} is not valid Unicode")))
        }
    }
}

fn detect_environment_with_dotenv(
    dotenv: &HashMap<String, String>,
) -> Result<rullst_core::config::Environment, AuthError> {
    let configured_environment = match fs::read_to_string("Rullst.toml") {
        Ok(content) => Some(
            rullst_core::config::RullstConfig::from_toml(&content)
                .map_err(|error| AuthError::General(error.to_string()))?
                .app
                .env,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AuthError::General(error.to_string())),
    }
    .flatten();

    let rullst_env = read_process_environment("RULLST_ENV")?;
    let app_env = read_process_environment("APP_ENV")?;
    let fallback = dotenv
        .get("RULLST_ENV")
        .or_else(|| dotenv.get("APP_ENV"))
        .map(String::as_str)
        .or(configured_environment.as_deref());
    rullst_core::config::Environment::resolve(rullst_env.as_deref(), app_env.as_deref(), fallback)
        .map_err(|error| AuthError::General(error.to_string()))
}

pub(super) fn detect_environment() -> Result<rullst_core::config::Environment, AuthError> {
    // Process selectors outrank `.env`; when one is set, never read the file.
    if read_process_environment("RULLST_ENV")?.is_some()
        || read_process_environment("APP_ENV")?.is_some()
    {
        return detect_environment_with_dotenv(&HashMap::new());
    }
    detect_environment_with_dotenv(&load_dotenv_values()?)
}

/// Returns the key actually cached, so a caller that lost a concurrent first
/// resolution never seals data with a key that is not used afterwards.
fn cache_validated_app_key(key: Vec<u8>) -> Result<Vec<u8>, AuthError> {
    validate_app_key(&key)?;
    Ok(CACHED_APP_KEY.get_or_init(|| key).clone())
}

/// Resolves the application's unique secret key for encryption.
///
/// Reads `APP_KEY` from the process environment, then `.env`, then
/// `Rullst.toml`. Without one, staging and production fail with
/// [`AuthError::MissingAppKey`]. Other environments use a development key
/// kept outside the source tree (since 13.0.0-alpha.5): an existing
/// `.rullst_dev_key` in the current directory is still read, but none is
/// written there. Otherwise the project's key in the per-user state
/// directory (`$XDG_STATE_HOME`, else `~/.local/state`, or `%LOCALAPPDATA%`,
/// under `rullst/dev-keys/`) is used or created. A test harness (an
/// executable in Cargo's `target/<profile>/deps`, where `cargo test` and
/// cargo-nextest run unit and integration tests) never writes a key and
/// keeps a new one in memory, so set `APP_KEY` where tests in several
/// processes must share one. The resolved key is cached for the process.
#[cfg_attr(mutants, mutants::skip)]
pub fn get_app_key() -> Result<Vec<u8>, AuthError> {
    if let Some(cached) = CACHED_APP_KEY.get() {
        return Ok(cached.clone());
    }

    if let Some(env_key) = read_process_environment("APP_KEY")? {
        return cache_validated_app_key(env_key.into_bytes());
    }

    let dotenv = load_dotenv_values()?;
    if let Some(dotenv_key) = dotenv.get("APP_KEY") {
        return cache_validated_app_key(dotenv_key.as_bytes().to_vec());
    }

    if let Ok(toml_content) = fs::read_to_string("Rullst.toml")
        && let Some(key) = parse_app_key_from_toml(&toml_content)
    {
        return cache_validated_app_key(key);
    }

    // Enforce explicit APP_KEY when running in production.
    if detect_environment_with_dotenv(&dotenv)?.requires_secure_defaults() {
        return Err(AuthError::MissingAppKey(
            "APP_KEY is required in staging and production".to_string(),
        ));
    }

    // Only one caller in this process generates the key; later ones reuse it.
    let _generation = DEVELOPMENT_KEY_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(cached) = CACHED_APP_KEY.get() {
        return Ok(cached.clone());
    }
    let (key, origin) =
        super::dev_key::resolve().map_err(|error| AuthError::MissingAppKey(error.to_string()))?;
    match origin {
        super::dev_key::Origin::Existing => {}
        super::dev_key::Origin::Persisted(path) => eprintln!(
            "⚠️  Rullst Security Warning: generated a development APP_KEY in {} (outside the project). Set APP_KEY in staging and production.",
            path.display()
        ),
        super::dev_key::Origin::NoStateDirectory => eprintln!(
            "⚠️  Rullst Security Warning: no user state directory is configured, so this process uses a development APP_KEY of its own; sessions end when it stops. Set APP_KEY to keep them."
        ),
        super::dev_key::Origin::TestHarness => {}
    }
    cache_validated_app_key(key)
}
