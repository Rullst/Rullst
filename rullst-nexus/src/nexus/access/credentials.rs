//! Validation and environment loading of Nexus Basic Auth credentials.

use super::{MIN_NEXUS_PASSWORD_LENGTH, NexusBuildError};
use rullst_core::config::ConfigError;

pub(super) fn validate_username(username: &str) -> Result<(), NexusBuildError> {
    if username.trim().is_empty() {
        return Err(NexusBuildError::EmptyUsername);
    }
    if username.contains(':') {
        return Err(NexusBuildError::UsernameContainsSeparator);
    }
    if username.trim().len() != username.len()
        || username.len() > 255
        || username.chars().any(char::is_control)
    {
        return Err(NexusBuildError::InvalidUsername);
    }
    if is_placeholder_username(username) {
        return Err(NexusBuildError::PlaceholderUsername);
    }
    Ok(())
}

pub(super) fn validate_password(password: &str) -> Result<(), NexusBuildError> {
    if password.chars().count() < MIN_NEXUS_PASSWORD_LENGTH {
        return Err(NexusBuildError::WeakPassword {
            minimum: MIN_NEXUS_PASSWORD_LENGTH,
        });
    }
    if is_placeholder_password(password) {
        return Err(NexusBuildError::PlaceholderPassword);
    }
    Ok(())
}

fn is_placeholder_username(username: &str) -> bool {
    matches!(
        username.trim().to_ascii_lowercase().as_str(),
        "username" | "user_name" | "your_username" | "your_user" | "change_me" | "changeme"
    )
}

fn is_placeholder_password(password: &str) -> bool {
    let normalized = password.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "password"
            | "password123"
            | "admin_password"
            | "your_password"
            | "your_strong_password"
            | "replace_with_a_strong_password"
            | "change_me_before_deploying"
            | "changeme_before_deploying"
    ) || normalized.starts_with("replace_me_")
        || normalized.starts_with("change_me_")
        || normalized.starts_with("your_password_")
}

/// Reads `name` from the process environment, then from the working
/// directory's `.env`, which is never loaded into the process environment.
pub(super) fn required_environment_variable(name: &'static str) -> Result<String, NexusBuildError> {
    match rullst_core::config::project_setting(name) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(NexusBuildError::MissingCredential { variable: name }),
        Err(ConfigError::NonUnicodeEnvironmentVariable(_)) => {
            Err(NexusBuildError::InvalidCredentialEncoding { variable: name })
        }
        Err(_) => Err(NexusBuildError::InvalidDotenv { variable: name }),
    }
}
