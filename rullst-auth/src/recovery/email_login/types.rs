use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

/// A trusted server clock, never a timestamp from a request or email link.
pub trait EmailLoginClock: Send + Sync {
    fn now(&self) -> Result<u64, RecoveryError>;
}

pub struct SystemEmailLoginClock;
impl EmailLoginClock for SystemEmailLoginClock {
    fn now(&self) -> Result<u64, RecoveryError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .map_err(|_| RecoveryError::InvalidAction)
    }
}

/// Independently generated browser secret. Keep it in a Secure, HttpOnly,
/// SameSite cookie; never add it to the email or the link. Redemption requires
/// both this browser secret and the separately delivered email credential.
#[derive(Debug)]
pub struct BrowserBinding(SecretToken);
impl BrowserBinding {
    pub fn generate() -> Result<Self, RecoveryError> {
        Ok(Self(SecretToken::generate()?))
    }
    pub fn from_cookie(value: &str) -> Result<Self, RecoveryError> {
        Ok(Self(SecretToken::from_encoded(value)?))
    }
    /// Sensitive plaintext for setting/reading the dedicated browser cookie only.
    pub fn expose_cookie(&self) -> &str {
        self.0.expose()
    }
}

/// Uniform public acknowledgement: it contains no account existence or status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginRequestAccepted;

/// Immutable server-owned namespace, landing endpoint and local destination.
#[derive(Clone)]
pub struct EmailLoginConfig {
    pub(super) namespace: String,
    pub(super) landing: Url,
    pub(super) destination: String,
    pub(super) capacity: usize,
    pub(super) development: bool,
}

impl std::fmt::Debug for EmailLoginConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailLoginConfig")
            .field("capacity", &self.capacity)
            .field("development", &self.development)
            .finish_non_exhaustive()
    }
}

impl EmailLoginConfig {
    /// Production landing requires HTTPS. Destination is a fixed local path;
    /// query parameters, fragments, encoding, backslashes and authority syntax
    /// are rejected instead of interpreting client-provided redirect targets.
    pub fn new(
        namespace: impl Into<String>,
        landing: impl Into<String>,
        destination: impl Into<String>,
        capacity: usize,
    ) -> Result<Self, RecoveryError> {
        Self::build(
            namespace.into(),
            landing.into(),
            destination.into(),
            capacity,
            false,
        )
    }

    /// Explicit development profile; HTTP is accepted only on loopback hosts.
    pub fn for_development(
        namespace: impl Into<String>,
        landing: impl Into<String>,
        destination: impl Into<String>,
        capacity: usize,
    ) -> Result<Self, RecoveryError> {
        Self::build(
            namespace.into(),
            landing.into(),
            destination.into(),
            capacity,
            true,
        )
    }

    fn build(
        namespace: String,
        landing: String,
        destination: String,
        capacity: usize,
        development: bool,
    ) -> Result<Self, RecoveryError> {
        if !super::super::valid_subject(&namespace)
            || !(1..=100_000).contains(&capacity)
            || landing.len() > 2048
            || !safe_path(&destination)
        {
            return Err(RecoveryError::Configuration);
        }
        let landing = Url::parse(&landing).map_err(|_| RecoveryError::Configuration)?;
        let local = landing.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if landing.host_str().is_none()
            || !landing.username().is_empty()
            || landing.password().is_some()
            || landing.query().is_some()
            || landing.fragment().is_some()
            || !safe_path(landing.path())
            || !(landing.scheme() == "https"
                || (development && local && landing.scheme() == "http"))
        {
            return Err(RecoveryError::Configuration);
        }
        Ok(Self {
            namespace,
            landing,
            destination,
            capacity,
            development,
        })
    }
}

fn safe_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 512
        && !value.split('/').any(|part| part == "." || part == "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_'))
}

/// Session returned only after committed one-use redemption and session creation.
/// Debug omits the account, cookie and destination. Ordinary authorization still
/// resolves active tenant membership and roles on each application request.
pub struct EmailLoginSession {
    pub(super) token: SecretToken,
    pub(super) subject: String,
    pub(super) destination: String,
}
impl std::fmt::Debug for EmailLoginSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EmailLoginSession([REDACTED])")
    }
}
impl EmailLoginSession {
    pub fn token(&self) -> &SecretToken {
        &self.token
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn destination(&self) -> &str {
        &self.destination
    }
}
