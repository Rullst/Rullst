mod rate_limit;
#[cfg(test)]
mod tests;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::Engine;
use rate_limit::{AuthGuardStatus, BasicAuthRateLimiter};
pub use rate_limit::{
    NEXUS_BASIC_AUTH_FAILURE_WINDOW, NEXUS_BASIC_AUTH_LOCKOUT, NEXUS_BASIC_AUTH_MAX_FAILURES,
    NEXUS_BASIC_AUTH_MAX_PEERS,
};
use std::{fmt, net::SocketAddr, sync::Arc, time::Duration};
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
    rate_limiter: Arc<BasicAuthRateLimiter>,
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

        Ok(Self {
            username,
            password,
            rate_limiter: Arc::new(BasicAuthRateLimiter::default()),
        })
    }
}

/// Capability inserted into requests only after direct TLS or a trusted TLS terminator has been
/// verified by the application.
///
/// Basic credentials are cleartext at the HTTP layer. Nexus therefore refuses Basic Auth unless
/// the request URI is HTTPS or this marker is present. Applications behind a reverse proxy must
/// insert this extension only from middleware that trusts the socket peer and verifies the
/// terminator's transport metadata; never derive it from an arbitrary forwarded header.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct NexusVerifiedTls {
    _private: (),
}

impl NexusVerifiedTls {
    /// Asserts that a trusted listener or reverse proxy already authenticated the TLS transport.
    pub const fn from_trusted_tls_termination() -> Self {
        Self { _private: () }
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
    if !has_verified_tls(&request) {
        return plaintext_basic_auth_response();
    }

    let Some(peer_ip) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connection| connection.0.ip())
    else {
        return status_response(StatusCode::SERVICE_UNAVAILABLE);
    };

    match credentials.rate_limiter.status(peer_ip) {
        AuthGuardStatus::Allowed => {}
        AuthGuardStatus::Locked(remaining) => return lockout_response(remaining),
        AuthGuardStatus::Unavailable => return status_response(StatusCode::SERVICE_UNAVAILABLE),
    }

    if has_valid_basic_credentials(&request, &credentials) {
        if !credentials.rate_limiter.record_success(peer_ip) {
            return status_response(StatusCode::SERVICE_UNAVAILABLE);
        }
        request.extensions_mut().insert(NexusPrincipal);
        next.run(request).await
    } else {
        match credentials.rate_limiter.record_failure(peer_ip) {
            AuthGuardStatus::Allowed => unauthorized_response(),
            AuthGuardStatus::Locked(remaining) => lockout_response(remaining),
            AuthGuardStatus::Unavailable => status_response(StatusCode::SERVICE_UNAVAILABLE),
        }
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
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("basic"))
        .map(|(_, encoded)| encoded)
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

fn has_verified_tls(request: &Request) -> bool {
    request.uri().scheme_str() == Some("https")
        || request.extensions().get::<NexusVerifiedTls>().is_some()
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

fn plaintext_basic_auth_response() -> Response {
    let mut response = status_response(StatusCode::UPGRADE_REQUIRED);
    response.headers_mut().insert(
        header::UPGRADE,
        HeaderValue::from_static("TLS/1.2, HTTP/1.1"),
    );
    response
}

fn lockout_response(remaining: Duration) -> Response {
    let mut response = status_response(StatusCode::TOO_MANY_REQUESTS);
    let retry_after = remaining.as_secs().max(1).to_string();
    if let Ok(value) = HeaderValue::from_str(&retry_after) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    rullst_security::SecurityStore::global().inc_rate_limit_blocks();
    response
}

fn status_response(status: StatusCode) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = status;
    response
}
