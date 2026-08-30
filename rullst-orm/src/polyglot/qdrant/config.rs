use std::{fmt, net::IpAddr};

use reqwest::Url;

use super::super::PolyglotError;

const DEFAULT_RESPONSE_LIMIT: usize = 2 * 1024 * 1024;
const MIN_RESPONSE_LIMIT: usize = 1024;
const MAX_RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
const MAX_API_KEY_BYTES: usize = 2_048;

enum QdrantAuth {
    ApiKey(String),
    UnauthenticatedLoopback,
}

/// Safe configuration for the bounded Qdrant HTTP adapter.
pub struct QdrantConfig {
    pub(super) endpoint: String,
    auth: QdrantAuth,
    pub(super) response_limit: usize,
}

impl QdrantConfig {
    /// Configures an authenticated Qdrant endpoint. Empty or `mock_*` values
    /// select the deterministic offline backend.
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth: QdrantAuth::ApiKey(api_key.into()),
            response_limit: DEFAULT_RESPONSE_LIMIT,
        }
    }

    /// Explicitly selects an unauthenticated loopback Qdrant instance.
    pub fn unauthenticated_local(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth: QdrantAuth::UnauthenticatedLoopback,
            response_limit: DEFAULT_RESPONSE_LIMIT,
        }
    }

    /// Overrides the response-memory ceiling, from 1 KiB through 16 MiB.
    pub fn with_response_limit(mut self, bytes: usize) -> Result<Self, PolyglotError> {
        if !(MIN_RESPONSE_LIMIT..=MAX_RESPONSE_LIMIT).contains(&bytes) {
            return Err(PolyglotError::InvalidConfiguration {
                backend: "Qdrant",
                reason: "response limit must be between 1 KiB and 16 MiB",
            });
        }
        self.response_limit = bytes;
        Ok(self)
    }

    pub(super) fn requests_mock(&self) -> bool {
        is_mock_value(&self.endpoint)
            || matches!(&self.auth, QdrantAuth::ApiKey(key) if is_mock_value(key))
    }

    pub(super) fn validate(self) -> Result<ValidatedQdrantConfig, PolyglotError> {
        let endpoint = validate_endpoint(&self.endpoint)?;
        let api_key = match self.auth {
            QdrantAuth::ApiKey(key) => {
                validate_api_key(&key)?;
                Some(key)
            }
            QdrantAuth::UnauthenticatedLoopback => {
                if !endpoint.host_str().is_some_and(is_loopback_host) {
                    return Err(PolyglotError::InvalidConfiguration {
                        backend: "Qdrant",
                        reason: "unauthenticated mode is restricted to loopback endpoints",
                    });
                }
                None
            }
        };
        Ok(ValidatedQdrantConfig {
            endpoint,
            api_key,
            response_limit: self.response_limit,
        })
    }
}

impl fmt::Debug for QdrantConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QdrantConfig")
            .field("endpoint", &"[CONFIGURED]")
            .field("api_key", &"[REDACTED]")
            .field("response_limit", &self.response_limit)
            .finish()
    }
}

pub(super) struct ValidatedQdrantConfig {
    pub(super) endpoint: Url,
    pub(super) api_key: Option<String>,
    pub(super) response_limit: usize,
}

fn validate_endpoint(value: &str) -> Result<Url, PolyglotError> {
    let mut endpoint = Url::parse(value).map_err(|_| invalid("endpoint is not a valid URL"))?;
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || !matches!(endpoint.path(), "" | "/")
    {
        return Err(invalid(
            "endpoint must not contain credentials, a path, query parameters, or a fragment",
        ));
    }
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err(invalid("only HTTP and HTTPS endpoints are supported"));
    }
    let local = endpoint.host_str().is_some_and(is_loopback_host);
    if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && local) {
        return Err(invalid("remote endpoints must use HTTPS"));
    }
    endpoint.set_path("/");
    Ok(endpoint)
}

fn validate_api_key(value: &str) -> Result<(), PolyglotError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_API_KEY_BYTES
        && value
            .bytes()
            .all(|byte| byte == b' ' || byte.is_ascii_graphic());
    if !valid {
        return Err(invalid("API key must contain 1-2,048 visible ASCII bytes"));
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn is_mock_value(value: &str) -> bool {
    value.is_empty() || value.starts_with("mock_") || value.starts_with("mock://")
}

fn invalid(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidConfiguration {
        backend: "Qdrant",
        reason,
    }
}
