//! [`NexusBuildError`], the typed failures of [`crate::nexus::Nexus::try_build`].

use std::fmt;

/// Errors returned while securely constructing a Nexus router.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NexusBuildError {
    /// No access policy was selected. Nexus never mounts an open admin router.
    MissingAuthenticationPolicy,
    /// The Basic Auth username is empty or only contains whitespace.
    EmptyUsername,
    /// The Basic Auth username is padded, oversized, or contains control characters.
    InvalidUsername,
    /// Basic Auth usernames cannot contain the `:` credential separator.
    UsernameContainsSeparator,
    /// A public/example placeholder was supplied as the username.
    PlaceholderUsername,
    /// The configured password is shorter than the required minimum.
    WeakPassword { minimum: usize },
    /// A public/example placeholder was supplied as the password.
    PlaceholderPassword,
    /// A required Nexus credential was not provided by the process or `.env` file.
    MissingCredential { variable: &'static str },
    /// A required Nexus credential contains invalid Unicode.
    InvalidCredentialEncoding { variable: &'static str },
    /// A required Nexus credential is absent from the process environment and
    /// the project's `.env` file is unreadable or malformed. Added in v13.
    InvalidDotenv { variable: &'static str },
    /// Unauthenticated local access is never available in release builds.
    LocalAccessRequiresDebugBuild,
    /// Registered model metadata is ambiguous, oversized, or unsafe for dynamic CRUD.
    InvalidModelMetadata { reason: &'static str },
    /// The login page of [`NexusApplicationAuth::with_login_path`] is not a
    /// local path. Added in v13.
    ///
    /// [`NexusApplicationAuth::with_login_path`]: super::NexusApplicationAuth::with_login_path
    InvalidLoginPath,
    /// A stylesheet of [`Nexus::with_stylesheet`] is not a same-origin path,
    /// or more than eight were added. Added in 13.0.0-alpha.5.
    ///
    /// [`Nexus::with_stylesheet`]: crate::nexus::Nexus::with_stylesheet
    InvalidStylesheet,
}

impl fmt::Display for NexusBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAuthenticationPolicy => formatter.write_str(
                "Nexus requires an explicit authentication policy before it can be mounted",
            ),
            Self::EmptyUsername => {
                formatter.write_str("Nexus Basic Auth requires a non-empty username")
            }
            Self::InvalidUsername => formatter.write_str(
                "Nexus Basic Auth usernames must not have surrounding whitespace, must contain at most 255 bytes, and must not contain control characters",
            ),
            Self::UsernameContainsSeparator => formatter.write_str(
                "Nexus Basic Auth usernames cannot contain the ':' credential separator",
            ),
            Self::PlaceholderUsername => formatter.write_str(
                "Nexus Basic Auth rejected a public placeholder username; configure a real value",
            ),
            Self::WeakPassword { minimum } => write!(
                formatter,
                "Nexus Basic Auth passwords must contain at least {minimum} characters"
            ),
            Self::PlaceholderPassword => formatter.write_str(
                "Nexus Basic Auth rejected a public placeholder password; configure a unique secret",
            ),
            Self::MissingCredential { variable } => write!(
                formatter,
                "Nexus requires the {variable} environment variable; no default credential is used"
            ),
            Self::InvalidCredentialEncoding { variable } => write!(
                formatter,
                "Nexus requires {variable} to contain valid Unicode"
            ),
            Self::InvalidDotenv { variable } => write!(
                formatter,
                "Nexus could not read {variable}: the project .env file is unreadable or malformed"
            ),
            Self::LocalAccessRequiresDebugBuild => formatter.write_str(
                "Nexus loopback-only access is restricted to debug builds; configure an authenticated policy for release builds",
            ),
            Self::InvalidModelMetadata { reason } => {
                write!(formatter, "Nexus rejected registered model metadata: {reason}")
            }
            Self::InvalidLoginPath => formatter.write_str(
                "the Nexus login path must be a local path starting with a single '/', of at most 512 visible ASCII characters without '\\'",
            ),
            Self::InvalidStylesheet => formatter.write_str(
                "a Nexus stylesheet must be a same-origin path starting with a single '/', of at most 512 visible ASCII characters without quotes, '<', '>', '`' or '\\', and at most 8 may be added",
            ),
        }
    }
}

impl std::error::Error for NexusBuildError {}
