use std::{fmt, net::IpAddr};

use reqwest::Url;

use super::PolyglotError;

const DEFAULT_RESPONSE_LIMIT: usize = 1024 * 1024;
const MIN_RESPONSE_LIMIT: usize = 1024;
const MAX_RESPONSE_LIMIT: usize = 8 * 1024 * 1024;

/// Authentication accepted by SurrealDB's HTTP protocol.
#[non_exhaustive]
pub enum SurrealAuth {
    /// Connect to an explicitly unauthenticated local or protected instance.
    None,
    /// HTTP Basic authentication for root, namespace, or database users.
    Basic { username: String, password: String },
    /// Bearer token authentication.
    Bearer(String),
}

impl SurrealAuth {
    /// Creates Basic authentication without exposing secrets through `Debug`.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates bearer authentication without exposing the token through `Debug`.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    pub(super) fn requests_mock(&self) -> bool {
        match self {
            Self::None => false,
            Self::Basic { username, password } => {
                is_mock_credential(username) || is_mock_credential(password)
            }
            Self::Bearer(token) => is_mock_credential(token),
        }
    }
}

impl fmt::Debug for SurrealAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("SurrealAuth::None"),
            Self::Basic { .. } => formatter.write_str("SurrealAuth::Basic([REDACTED])"),
            Self::Bearer(_) => formatter.write_str("SurrealAuth::Bearer([REDACTED])"),
        }
    }
}

/// Safe configuration for the SurrealDB HTTP adapter.
pub struct SurrealConfig {
    pub(super) endpoint: String,
    pub(super) namespace: String,
    pub(super) database: String,
    pub(super) auth: SurrealAuth,
    pub(super) response_limit: usize,
    pub(super) allow_insecure_http: bool,
}

impl SurrealConfig {
    /// Creates a configuration. Validation occurs in `connect_or_mock` so an
    /// empty or `mock_*` endpoint can select the deterministic fallback.
    pub fn new(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        auth: SurrealAuth,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            namespace: namespace.into(),
            database: database.into(),
            auth,
            response_limit: DEFAULT_RESPONSE_LIMIT,
            allow_insecure_http: false,
        }
    }

    /// Overrides the response-memory ceiling, from 1 KiB through 8 MiB.
    pub fn with_response_limit(mut self, bytes: usize) -> Result<Self, PolyglotError> {
        if !(MIN_RESPONSE_LIMIT..=MAX_RESPONSE_LIMIT).contains(&bytes) {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "SurrealDB",
                reason: "response limit must be between 1 KiB and 8 MiB",
            });
        }
        self.response_limit = bytes;
        Ok(self)
    }

    /// Explicitly permits cleartext HTTP for non-loopback development networks.
    pub fn allow_insecure_http(mut self) -> Self {
        self.allow_insecure_http = true;
        self
    }
}

impl fmt::Debug for SurrealConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SurrealConfig")
            .field("endpoint", &"[CONFIGURED]")
            .field("namespace", &self.namespace)
            .field("database", &self.database)
            .field("auth", &self.auth)
            .field("response_limit", &self.response_limit)
            .field("allow_insecure_http", &self.allow_insecure_http)
            .finish()
    }
}

pub(super) fn validate_endpoint(
    value: &str,
    allow_insecure_http: bool,
) -> Result<Url, PolyglotError> {
    let mut endpoint = Url::parse(value).map_err(|_| PolyglotError::InvalidConfiguration {
        backend: "SurrealDB",
        reason: "endpoint is not a valid URL",
    })?;
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(PolyglotError::InvalidConfiguration {
            backend: "SurrealDB",
            reason: "endpoint must not contain credentials, query parameters, or a fragment",
        });
    }
    let local = endpoint.host_str().is_some_and(is_loopback_host);
    if endpoint.scheme() != "https"
        && !(endpoint.scheme() == "http" && (local || allow_insecure_http))
    {
        return Err(PolyglotError::InvalidConfiguration {
            backend: "SurrealDB",
            reason: "use HTTPS, a loopback HTTP endpoint, or explicitly allow insecure HTTP",
        });
    }
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err(PolyglotError::InvalidConfiguration {
            backend: "SurrealDB",
            reason: "only HTTP and HTTPS endpoints are supported",
        });
    }
    if !endpoint.path().ends_with('/') {
        let path = format!("{}/", endpoint.path());
        endpoint.set_path(&path);
    }
    Ok(endpoint)
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

pub(super) fn is_mock_credential(value: &str) -> bool {
    value.is_empty() || value.starts_with("mock_") || value.starts_with("mock://")
}
