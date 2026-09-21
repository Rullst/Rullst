use super::{LiveRecoveryError, LiveResult};
use http::{HeaderMap, Uri, header};
use std::{net::IpAddr, time::Duration};

/// Validated origin and hard limits for one shared Live recovery handler.
#[derive(Clone, Debug)]
pub struct LiveRecoveryConfig {
    pub(super) origin: String,
    pub(super) max_connections: usize,
    pub(super) operation_timeout: Duration,
    pub(super) revalidate_every: Duration,
    pub(super) max_lifetime: Duration,
    pub(super) max_actions: usize,
}

impl LiveRecoveryConfig {
    /// Accepts one exact HTTPS browser origin without credentials, path or query.
    pub fn try_new(origin: impl Into<String>) -> LiveResult<Self> {
        Self::new(origin.into(), false)
    }

    /// Explicit HTTP literal-loopback profile for local development and tests.
    pub fn loopback_for_tests(origin: impl Into<String>) -> LiveResult<Self> {
        Self::new(origin.into(), true)
    }

    fn new(origin: String, loopback: bool) -> LiveResult<Self> {
        let uri: Uri = origin.parse().map_err(|_| LiveRecoveryError::Invalid)?;
        let scheme = uri.scheme_str().ok_or(LiveRecoveryError::Invalid)?;
        let authority = uri.authority().ok_or(LiveRecoveryError::Invalid)?;
        let is_loopback = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback());
        if origin.len() > 512
            || origin != format!("{scheme}://{authority}")
            || authority.as_str().contains('@')
            || authority.port_u16() == Some(0)
            || !(scheme == "https" || (loopback && scheme == "http" && is_loopback))
        {
            return Err(LiveRecoveryError::Invalid);
        }
        Ok(Self {
            origin,
            max_connections: 128,
            operation_timeout: Duration::from_secs(5),
            revalidate_every: Duration::from_secs(5),
            max_lifetime: Duration::from_secs(3600),
            max_actions: 128,
        })
    }

    /// Bounds shared connections (1–1024) and actions per connection (1–1024).
    pub fn with_capacity(mut self, connections: usize, actions: usize) -> LiveResult<Self> {
        if !(1..=1024).contains(&connections) || !(1..=1024).contains(&actions) {
            return Err(LiveRecoveryError::Invalid);
        }
        self.max_connections = connections;
        self.max_actions = actions;
        Ok(self)
    }

    /// Bounds callback/send time (100 ms–10 s), revalidation (100 ms–30 s)
    /// and total connection lifetime (1 s–1 hour).
    pub fn with_timing(
        mut self,
        operation_timeout: Duration,
        revalidate_every: Duration,
        lifetime: Duration,
    ) -> LiveResult<Self> {
        if !(Duration::from_millis(100)..=Duration::from_secs(10)).contains(&operation_timeout)
            || !(Duration::from_millis(100)..=Duration::from_secs(30)).contains(&revalidate_every)
            || !(Duration::from_secs(1)..=Duration::from_secs(3600)).contains(&lifetime)
        {
            return Err(LiveRecoveryError::Invalid);
        }
        self.operation_timeout = operation_timeout;
        self.revalidate_every = revalidate_every;
        self.max_lifetime = lifetime;
        Ok(self)
    }

    pub(super) fn permits(&self, headers: &HeaderMap) -> bool {
        let mut origins = headers.get_all(header::ORIGIN).iter();
        if origins.next().map(|value| value.as_bytes()) != Some(self.origin.as_bytes())
            || origins.next().is_some()
        {
            return false;
        }
        let mut offered = headers.get_all(header::SEC_WEBSOCKET_PROTOCOL).iter();
        offered.next().map(|value| value.as_bytes()) == Some(b"rullst.live.v1".as_slice())
            && offered.next().is_none()
    }
}
