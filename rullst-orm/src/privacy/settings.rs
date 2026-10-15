//! Field-encryption settings: the process environment first, then `./.env`.
//!
//! This is the precedence of `rullst::config::project_setting` and the
//! server's own settings. `.env` never overrides the process environment and
//! is never loaded into it; it is read only when the process lacks the name.

use super::PrivacyError;
use std::path::Path;

/// Reads `name` for the application in the current working directory.
pub(super) fn setting(name: &str) -> Result<Option<String>, PrivacyError> {
    setting_in(Path::new("."), name)
}

pub(super) fn setting_in(project_dir: &Path, name: &str) -> Result<Option<String>, PrivacyError> {
    match std::env::var(name) {
        Ok(value) => return Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => {}
        // The value itself is never formatted: it may be a key.
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(PrivacyError::EnvError(format!(
                "{name} is not valid Unicode"
            )));
        }
    }
    let content = match std::fs::read_to_string(project_dir.join(".env")) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(PrivacyError::EnvError(format!(
                ".env could not be read: {}",
                error.kind()
            )));
        }
    };
    dotenv_value(&content, name)
}

/// The last `.env` entry for `name`. Errors name the failing entry, never
/// content: dotenvy's own parse error embeds the unparsed remainder.
fn dotenv_value(content: &str, name: &str) -> Result<Option<String>, PrivacyError> {
    let mut value = None;
    for (index, entry) in dotenvy::from_read_iter(content.as_bytes()).enumerate() {
        let (key, entry_value) = entry.map_err(|error| {
            PrivacyError::EnvError(match error {
                dotenvy::Error::LineParse(..) => {
                    format!("invalid .env syntax in entry {}", index + 1)
                }
                _ => "invalid .env file".to_string(),
            })
        })?;
        if key == name {
            value = Some(entry_value);
        }
    }
    Ok(value)
}

/// `name`, or an error naming where it was looked up.
pub(super) fn required_setting(name: &str) -> Result<String, PrivacyError> {
    setting(name)?.ok_or_else(|| {
        PrivacyError::EnvError(format!(
            "{name} is set neither in the process environment nor in ./.env"
        ))
    })
}

#[cfg(test)]
mod tests;
