use crate::{BrokerConfig, MessagingError, Namespace, Result};
use redis::{ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo};
use std::{fmt, net::IpAddr, time::Duration};

/// Explicit standalone Redis deployment and bounded messaging namespace.
///
/// URLs never contain credentials. Empty or `mock_*` passwords select an offline
/// fixture; live failures never do. Keep the generation in deployment configuration
/// and change it only for a deliberately new namespace, not on every process start.
#[derive(Clone)]
pub struct RedisBrokerConfig {
    pub(super) broker: BrokerConfig,
    pub(super) generation: Namespace,
    pub(super) endpoint: String,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) timeout: Duration,
    pub(super) loopback: bool,
    pub(super) ca_certificate: Option<Vec<u8>>,
    production: bool,
}

impl RedisBrokerConfig {
    /// Creates a TLS configuration with a ten-second total operation deadline.
    pub fn try_new(
        broker: BrokerConfig,
        generation: impl Into<String>,
        endpoint: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self> {
        if broker.max_retained_messages() > 10_000
            || broker.max_subscriptions() > 128
            || broker.max_payload_bytes() > 1024 * 1024
        {
            return Err(invalid(
                "Redis profile limits",
                "maximums are 10000 messages, 128 groups and one MiB per payload",
            ));
        }
        let config = Self {
            broker,
            generation: Namespace::try_new(generation)?,
            endpoint: endpoint.into(),
            username: username.into(),
            password: password.into(),
            timeout: Duration::from_secs(10),
            loopback: false,
            ca_certificate: None,
            production: false,
        };
        config.validate_credentials()?;
        config.validate_endpoint(true)?;
        Ok(config)
    }

    /// Enables plaintext only for a literal-loopback disposable test service.
    /// A production-sealed configuration rejects this override.
    pub fn allow_loopback_for_tests(mut self) -> Result<Self> {
        if self.production {
            return Err(invalid(
                "Redis endpoint",
                "production mode rejects test transport",
            ));
        }
        self.loopback = true;
        self.validate_endpoint(false)?;
        Ok(self)
    }

    /// Selects a total deadline from 100 milliseconds through 30 seconds.
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self> {
        if !(Duration::from_millis(100)..=Duration::from_secs(30)).contains(&timeout) {
            return Err(invalid(
                "Redis timeout",
                "must be between 100 milliseconds and 30 seconds",
            ));
        }
        self.timeout = timeout;
        Ok(self)
    }

    /// Uses one explicit PEM CA bundle for verified TLS (maximum 64 KiB).
    /// Hostname validation remains enabled; private keys are never accepted here.
    pub fn with_ca_certificate(mut self, pem: impl Into<Vec<u8>>) -> Result<Self> {
        let pem = pem.into();
        let text = std::str::from_utf8(&pem).map_err(|_| {
            invalid(
                "Redis CA certificate",
                "requires bounded PEM certificate data",
            )
        })?;
        if pem.is_empty()
            || pem.len() > 65536
            || !text.contains("-----BEGIN CERTIFICATE-----")
            || text.contains("PRIVATE KEY")
        {
            return Err(invalid(
                "Redis CA certificate",
                "requires bounded PEM certificate data",
            ));
        }
        self.ca_certificate = Some(pem);
        Ok(self)
    }

    /// Rejects mock credentials and test transport, retaining that restriction.
    pub fn require_production(mut self) -> Result<Self> {
        if self.is_mock() || self.loopback {
            return Err(invalid(
                "Redis deployment",
                "requires live credentials and verified TLS",
            ));
        }
        self.validate_endpoint(false)?;
        self.production = true;
        Ok(self)
    }

    /// Returns the broker's immutable namespace limits.
    pub fn broker(&self) -> &BrokerConfig {
        &self.broker
    }

    pub(super) fn is_mock(&self) -> bool {
        self.password.is_empty() || self.password.starts_with("mock_")
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.username.len() > 128
            || self.password.len() > 4096
            || self.username.chars().any(char::is_control)
            || self.password.chars().any(char::is_control)
            || (!self.is_mock() && (self.username.is_empty() || self.username.starts_with("mock_")))
        {
            return Err(invalid(
                "Redis credentials",
                "requires bounded explicit ACL credentials",
            ));
        }
        Ok(())
    }

    fn validate_endpoint(&self, defer_plaintext: bool) -> Result<()> {
        if self.endpoint.len() > 2048 {
            return Err(invalid("Redis endpoint", "endpoint is oversized"));
        }
        let url = url::Url::parse(&self.endpoint)
            .map_err(|_| invalid("Redis endpoint", "invalid URL"))?;
        if !matches!(url.scheme(), "redis" | "rediss")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !matches!(url.path(), "" | "/" | "/0")
            || url.host_str().is_none()
            || url.port() == Some(0)
        {
            return Err(invalid(
                "Redis endpoint",
                "requires a credential-free database-zero URL",
            ));
        }
        if url.scheme() == "redis" {
            let literal_loopback = url
                .host_str()
                .and_then(|host| {
                    host.trim_start_matches('[')
                        .trim_end_matches(']')
                        .parse::<IpAddr>()
                        .ok()
                })
                .is_some_and(|ip| ip.is_loopback());
            if !literal_loopback || (!defer_plaintext && !self.loopback) {
                return Err(invalid(
                    "Redis endpoint",
                    "plaintext requires an explicit literal-loopback test override",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn connection_info(&self) -> Result<ConnectionInfo> {
        self.validate_endpoint(false)?;
        let info = self
            .endpoint
            .as_str()
            .into_connection_info()
            .map_err(|_| invalid("Redis endpoint", "unsupported connection URL"))?;
        Ok(info.set_redis_settings(
            RedisConnectionInfo::default()
                .set_username(&self.username)
                .set_password(&self.password)
                .set_skip_set_lib_name(),
        ))
    }

    pub(super) fn signature(&self) -> String {
        format!(
            "rullst.redis-streams.v1:{}:{}:{}:{}:{}",
            self.generation,
            self.broker.max_retained_messages(),
            self.broker.max_subscriptions(),
            self.broker.max_attempts(),
            self.broker.max_payload_bytes()
        )
    }
}

pub(super) fn invalid(field: &'static str, reason: &'static str) -> MessagingError {
    MessagingError::Invalid { field, reason }
}

impl fmt::Debug for RedisBrokerConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RedisBrokerConfig")
            .field("broker", &self.broker)
            .field("is_mock", &self.is_mock())
            .field("timeout", &self.timeout)
            .field("test_transport", &self.loopback)
            .finish_non_exhaustive()
    }
}
