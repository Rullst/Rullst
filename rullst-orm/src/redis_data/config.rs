use std::{fmt, net::IpAddr};

use url::Url;

use crate::polyglot::PolyglotError;

const MAX_CREDENTIAL_BYTES: usize = 2_048;

enum RedisAuth {
    Acl { username: String, password: String },
    UnauthenticatedLoopback,
}

/// Safe connection and immutable namespace for native Redis structures.
pub struct RedisDataConfig {
    pub(super) endpoint: String,
    auth: RedisAuth,
    pub(super) namespace: String,
}

impl RedisDataConfig {
    /// Configures an authenticated Redis endpoint. Empty or `mock_*` values
    /// select the deterministic offline backend.
    pub fn new(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth: RedisAuth::Acl {
                username: username.into(),
                password: password.into(),
            },
            namespace: namespace.into(),
        }
    }

    /// Explicitly selects an unauthenticated loopback Redis instance.
    pub fn unauthenticated_local(
        endpoint: impl Into<String>,
        namespace: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            auth: RedisAuth::UnauthenticatedLoopback,
            namespace: namespace.into(),
        }
    }

    pub(super) fn requests_mock(&self) -> bool {
        is_mock_value(&self.endpoint)
            || matches!(
                &self.auth,
                RedisAuth::Acl { username, password }
                    if is_mock_value(username) || is_mock_value(password)
            )
    }

    pub(super) fn validate(self) -> Result<ValidatedRedisDataConfig, PolyglotError> {
        crate::query_cache::validate_namespace(&self.namespace).map_err(|_| {
            invalid(
                "namespace must contain 1-64 ASCII letters, digits, dots, dashes, or underscores",
            )
        })?;
        let endpoint = validate_endpoint(&self.endpoint)?;
        let credentials = match self.auth {
            RedisAuth::Acl { username, password } => {
                validate_credential(&username)?;
                validate_credential(&password)?;
                Some((username, password))
            }
            RedisAuth::UnauthenticatedLoopback => {
                if !endpoint.host_str().is_some_and(is_loopback_host) {
                    return Err(invalid(
                        "unauthenticated mode is restricted to loopback endpoints",
                    ));
                }
                None
            }
        };
        Ok(ValidatedRedisDataConfig {
            endpoint,
            namespace: self.namespace,
            credentials,
        })
    }

    pub(super) fn validate_mock_namespace(&self) -> Result<(), PolyglotError> {
        crate::query_cache::validate_namespace(&self.namespace).map_err(|_| {
            invalid(
                "namespace must contain 1-64 ASCII letters, digits, dots, dashes, or underscores",
            )
        })
    }
}

impl fmt::Debug for RedisDataConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisDataConfig")
            .field("endpoint", &"[CONFIGURED]")
            .field("credentials", &"[REDACTED]")
            .field("namespace", &self.namespace)
            .finish()
    }
}

pub(super) struct ValidatedRedisDataConfig {
    pub(super) endpoint: Url,
    pub(super) namespace: String,
    pub(super) credentials: Option<(String, String)>,
}

fn validate_endpoint(value: &str) -> Result<Url, PolyglotError> {
    let endpoint = Url::parse(value).map_err(|_| invalid("endpoint is not a valid URL"))?;
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || !matches!(endpoint.path(), "" | "/")
    {
        return Err(invalid(
            "endpoint must not contain credentials, a database path, query parameters, or a fragment",
        ));
    }
    if !matches!(endpoint.scheme(), "redis" | "rediss") {
        return Err(invalid(
            "only redis:// and rediss:// endpoints are supported",
        ));
    }
    let local = endpoint.host_str().is_some_and(is_loopback_host);
    if endpoint.scheme() != "rediss" && !(endpoint.scheme() == "redis" && local) {
        return Err(invalid("remote endpoints must use rediss:// TLS"));
    }
    Ok(endpoint)
}

fn validate_credential(value: &str) -> Result<(), PolyglotError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_CREDENTIAL_BYTES
        && value
            .bytes()
            .all(|byte| byte == b' ' || byte.is_ascii_graphic());
    if !valid {
        return Err(invalid(
            "credentials must contain 1-2,048 visible ASCII bytes",
        ));
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
        backend: "Redis",
        reason,
    }
}
