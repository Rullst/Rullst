use super::CloudError;
use aws_credential_types::Credentials;
use reqwest::Url;
use std::{
    fmt,
    net::IpAddr,
    time::{Duration, SystemTime},
};

/// Explicit signing credentials. Debug never reveals their values.
#[derive(Clone)]
pub struct CloudCredentials {
    pub(super) credentials: Credentials,
    pub(super) mock: bool,
}

impl fmt::Debug for CloudCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudCredentials")
            .field("mock", &self.mock)
            .finish_non_exhaustive()
    }
}

impl CloudCredentials {
    /// Empty credentials or two `mock_*` values select deterministic offline mode.
    /// Partially configured or mixed live/mock credentials are rejected.
    pub fn new(
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Result<Self, CloudError> {
        let access_key = access_key.into();
        let secret_key = secret_key.into();
        let empty = access_key.is_empty() && secret_key.is_empty();
        let mock = access_key.starts_with("mock_") && secret_key.starts_with("mock_");
        if !empty
            && !mock
            && (access_key.is_empty()
                || secret_key.is_empty()
                || access_key.starts_with("mock_")
                || secret_key.starts_with("mock_"))
        {
            return Err(CloudError::Configuration);
        }
        if access_key.len() > 128
            || secret_key.len() > 256
            || !access_key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !secret_key.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(CloudError::Configuration);
        }
        Ok(Self {
            credentials: Credentials::new(access_key, secret_key, None, None, "rullst-storage"),
            mock: empty || mock,
        })
    }

    /// Attaches an explicit session token and its known expiry; no STS lookup occurs.
    pub fn with_session_token(
        self,
        token: impl Into<String>,
        expires_at: SystemTime,
    ) -> Result<Self, CloudError> {
        let token = token.into();
        if self.mock
            || token.is_empty()
            || token.len() > 8192
            || !token.bytes().all(|b| b.is_ascii_graphic())
            || expires_at <= SystemTime::now()
        {
            return Err(CloudError::Configuration);
        }
        Ok(Self {
            credentials: Credentials::new(
                self.credentials.access_key_id(),
                self.credentials.secret_access_key(),
                Some(token),
                Some(expires_at),
                "rullst-storage",
            ),
            mock: false,
        })
    }
}

/// Resource and environment policy for one explicitly configured cloud client.
#[derive(Clone, Debug)]
pub struct CloudStorageConfig {
    pub(super) credentials: CloudCredentials,
    pub(super) max_object_bytes: usize,
    pub(super) timeout: Duration,
    pub(super) loopback_endpoint: Option<Url>,
    production: bool,
}

impl CloudStorageConfig {
    /// Uses a 16 MiB object limit and a 30 second deadline per operation.
    pub fn new(credentials: CloudCredentials) -> Self {
        Self {
            credentials,
            max_object_bytes: 16 * 1024 * 1024,
            timeout: Duration::from_secs(30),
            loopback_endpoint: None,
            production: false,
        }
    }

    /// Sets finite limits: 1 byte–256 MiB and 1 millisecond–120 seconds.
    pub fn with_limits(
        mut self,
        max_object_bytes: usize,
        timeout: Duration,
    ) -> Result<Self, CloudError> {
        if max_object_bytes == 0
            || max_object_bytes > 256 * 1024 * 1024
            || timeout < Duration::from_millis(1)
            || timeout > Duration::from_secs(120)
        {
            return Err(CloudError::Configuration);
        }
        self.max_object_bytes = max_object_bytes;
        self.timeout = timeout;
        Ok(self)
    }

    /// Explicitly enables a literal-loopback HTTP(S) endpoint for development tests.
    /// DNS names, user information, path prefixes, queries and fragments are rejected.
    pub fn with_loopback_test_endpoint(
        mut self,
        endpoint: impl Into<String>,
    ) -> Result<Self, CloudError> {
        if self.production {
            return Err(CloudError::Configuration);
        }
        let url = Url::parse(&endpoint.into()).map_err(|_| CloudError::Configuration)?;
        let host = url.host_str().ok_or(CloudError::Configuration)?;
        let ip: IpAddr = host
            .trim_matches(['[', ']'])
            .parse()
            .map_err(|_| CloudError::Configuration)?;
        if !ip.is_loopback()
            || !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(CloudError::Configuration);
        }
        self.loopback_endpoint = Some(url);
        Ok(self)
    }

    /// Fails closed if the configuration uses mocks or a development endpoint.
    /// Call this at the application's production configuration boundary.
    pub fn require_production(mut self) -> Result<Self, CloudError> {
        if self.credentials.mock || self.loopback_endpoint.is_some() {
            return Err(CloudError::Configuration);
        }
        self.production = true;
        Ok(self)
    }
}
