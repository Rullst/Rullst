use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::Engine;
use std::{fmt, net::SocketAddr};
use subtle::ConstantTimeEq;

pub(crate) const NEXUS_ADMIN_ROLE: &str = "NexusAdmin";

/// Authenticated administrator capability inserted only by a validated Nexus access policy.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NexusPrincipal;

impl rullst_auth::HasRole for NexusPrincipal {
    fn has_role(&self, role: &str) -> bool {
        role == NEXUS_ADMIN_ROLE
    }
}

/// Minimum accepted length for a Nexus Basic Auth password.
pub const MIN_NEXUS_PASSWORD_LENGTH: usize = 16;
/// Environment variable used for the Nexus Basic Auth username.
pub const NEXUS_ADMIN_USERNAME_ENV: &str = "NEXUS_ADMIN_USERNAME";
/// Environment variable used for the Nexus Basic Auth password.
pub const NEXUS_ADMIN_PASSWORD_ENV: &str = "NEXUS_ADMIN_PASSWORD";

/// Errors returned while securely constructing a Nexus router.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NexusBuildError {
    /// No access policy was selected. Nexus never mounts an open admin router.
    MissingAuthenticationPolicy,
    /// The Basic Auth username is empty or only contains whitespace.
    EmptyUsername,
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
    /// Unauthenticated local access is never available in release builds.
    LocalAccessRequiresDebugBuild,
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
            Self::LocalAccessRequiresDebugBuild => formatter.write_str(
                "Nexus loopback-only access is restricted to debug builds; configure an authenticated policy for release builds",
            ),
        }
    }
}

impl std::error::Error for NexusBuildError {}

/// Validated credentials for the built-in Nexus Basic Auth policy.
///
/// The password is deliberately redacted from `Debug` output.
#[non_exhaustive]
#[derive(Clone)]
pub struct NexusBasicAuth {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl NexusBasicAuth {
    /// Validates Basic Auth credentials before they can protect a Nexus router.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, NexusBuildError> {
        let username = username.into();
        let password = password.into();
        validate_username(&username)?;
        validate_password(&password)?;

        Ok(Self { username, password })
    }
}

impl fmt::Debug for NexusBasicAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NexusBasicAuth")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Explicit capability for running Nexus without credentials on loopback in development.
///
/// This policy is accepted only in debug builds. Every request must also contain Axum
/// `ConnectInfo<SocketAddr>` for a loopback peer; missing connection metadata is denied.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct LocalNexusAccess {
    _private: (),
}

impl LocalNexusAccess {
    /// Opts in to debug-only, loopback-verified local access.
    pub const fn loopback_only() -> Self {
        Self { _private: () }
    }
}

/// Access policies that may protect a Nexus admin router.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum NexusAuthPolicy {
    /// HTTP Basic Auth with credentials validated at configuration time.
    Basic(NexusBasicAuth),
    /// Credential-free access restricted to verified loopback peers in debug builds.
    LoopbackOnly(LocalNexusAccess),
}

impl NexusAuthPolicy {
    /// Creates a validated Basic Auth policy.
    pub fn basic(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, NexusBuildError> {
        NexusBasicAuth::new(username, password).map(Self::Basic)
    }

    /// Loads and validates Basic Auth credentials from the process environment or `.env`.
    ///
    /// The required variables are [`NEXUS_ADMIN_USERNAME_ENV`] and
    /// [`NEXUS_ADMIN_PASSWORD_ENV`]. No username or password fallback is provided.
    pub fn basic_from_env() -> Result<Self, NexusBuildError> {
        let _ = dotenvy::dotenv();
        let username = required_environment_variable(NEXUS_ADMIN_USERNAME_ENV)?;
        let password = required_environment_variable(NEXUS_ADMIN_PASSWORD_ENV)?;
        Self::basic(username, password)
    }

    /// Creates an explicitly opted-in loopback-only development policy.
    pub const fn loopback_only(access: LocalNexusAccess) -> Self {
        Self::LoopbackOnly(access)
    }
}

pub(crate) fn validate_policy(policy: NexusAuthPolicy) -> Result<NexusAuthPolicy, NexusBuildError> {
    match policy {
        NexusAuthPolicy::Basic(credentials) => {
            validate_username(&credentials.username)?;
            validate_password(&credentials.password)?;
            Ok(NexusAuthPolicy::Basic(credentials))
        }
        NexusAuthPolicy::LoopbackOnly(access) => {
            if cfg!(debug_assertions) {
                Ok(NexusAuthPolicy::LoopbackOnly(access))
            } else {
                Err(NexusBuildError::LocalAccessRequiresDebugBuild)
            }
        }
    }
}

pub(crate) async fn basic_auth_middleware(
    credentials: NexusBasicAuth,
    mut request: Request,
    next: Next,
) -> Response {
    if has_valid_basic_credentials(&request, &credentials) {
        request.extensions_mut().insert(NexusPrincipal);
        next.run(request).await
    } else {
        unauthorized_response()
    }
}

pub(crate) async fn loopback_only_middleware(mut request: Request, next: Next) -> Response {
    let is_loopback = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|connection| connection.0.ip().is_loopback());

    if is_loopback {
        request.extensions_mut().insert(NexusPrincipal);
        next.run(request).await
    } else {
        status_response(StatusCode::FORBIDDEN)
    }
}

fn has_valid_basic_credentials(request: &Request, credentials: &NexusBasicAuth) -> bool {
    let Some(encoded) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
    else {
        return false;
    };

    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(decoded) = String::from_utf8(decoded) else {
        return false;
    };
    let Some((username, password)) = decoded.split_once(':') else {
        return false;
    };

    constant_time_equal(username.as_bytes(), credentials.username.as_bytes())
        & constant_time_equal(password.as_bytes(), credentials.password.as_bytes())
}

fn constant_time_equal(candidate: &[u8], expected: &[u8]) -> bool {
    candidate.len() == expected.len() && bool::from(candidate.ct_eq(expected))
}

fn validate_username(username: &str) -> Result<(), NexusBuildError> {
    if username.trim().is_empty() {
        return Err(NexusBuildError::EmptyUsername);
    }
    if username.contains(':') {
        return Err(NexusBuildError::UsernameContainsSeparator);
    }
    if is_placeholder_username(username) {
        return Err(NexusBuildError::PlaceholderUsername);
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), NexusBuildError> {
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

fn required_environment_variable(name: &'static str) -> Result<String, NexusBuildError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => {
            Err(NexusBuildError::MissingCredential { variable: name })
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(NexusBuildError::InvalidCredentialEncoding { variable: name })
        }
    }
}

fn unauthorized_response() -> Response {
    let mut response = status_response(StatusCode::UNAUTHORIZED);
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"Nexus Admin Panel\""),
    );
    response
}

fn status_response(status: StatusCode) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn basic_auth_debug_output_redacts_password() {
        let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
            .expect("test credentials should be valid");
        let output = format!("{credentials:?}");

        assert!(output.contains("ops"));
        assert!(!output.contains("unique-test-secret-42"));
    }

    #[test]
    fn basic_auth_rejects_weak_and_placeholder_credentials() {
        assert_eq!(
            NexusBasicAuth::new("ops", "too-short").expect_err("short password must fail"),
            NexusBuildError::WeakPassword {
                minimum: MIN_NEXUS_PASSWORD_LENGTH
            }
        );
        assert_eq!(
            NexusBasicAuth::new("ops", "change_me_before_deploying")
                .expect_err("placeholder password must fail"),
            NexusBuildError::PlaceholderPassword
        );
        assert_eq!(
            NexusBasicAuth::new("your_username", "unique-test-secret-42")
                .expect_err("placeholder username must fail"),
            NexusBuildError::PlaceholderUsername
        );
    }

    #[test]
    fn basic_credentials_require_both_exact_values() {
        let credentials = NexusBasicAuth::new("ops", "unique-test-secret-42")
            .expect("test credentials should be valid");
        let valid_header =
            base64::engine::general_purpose::STANDARD.encode("ops:unique-test-secret-42");
        let wrong_user =
            base64::engine::general_purpose::STANDARD.encode("bad:unique-test-secret-42");
        let wrong_password = base64::engine::general_purpose::STANDARD.encode("ops:wrong-value");

        let valid = Request::builder()
            .header(header::AUTHORIZATION, format!("Basic {valid_header}"))
            .body(Body::empty())
            .expect("valid request");
        let invalid_user = Request::builder()
            .header(header::AUTHORIZATION, format!("Basic {wrong_user}"))
            .body(Body::empty())
            .expect("valid request");
        let invalid_password = Request::builder()
            .header(header::AUTHORIZATION, format!("Basic {wrong_password}"))
            .body(Body::empty())
            .expect("valid request");

        assert!(has_valid_basic_credentials(&valid, &credentials));
        assert!(!has_valid_basic_credentials(&invalid_user, &credentials));
        assert!(!has_valid_basic_credentials(
            &invalid_password,
            &credentials
        ));
    }
}
