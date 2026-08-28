//! Atomic Redis rate limiting with an explicit deterministic offline mode.

use super::RateLimitError;
use crate::telemetry::SecurityStore;
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_CLIENT_KEY_BYTES: usize = 1024;
const REDIS_FIXED_WINDOW_SCRIPT: &str = r#"
local current = redis.call('INCR', KEYS[1])
if current == 1 then
    redis.call('PEXPIRE', KEYS[1], ARGV[1])
end
local ttl = redis.call('PTTL', KEYS[1])
return { current, ttl }
"#;

/// Runtime mode selected by [`RedisRateLimiter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisRateLimitMode {
    /// Atomic shared Redis counter with a server-side expiry.
    Distributed,
    /// Deterministic process-local fallback for empty or `mock_*` URLs.
    OfflineMock,
}

/// Result of consuming one request from a bounded window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitDecision {
    /// Whether the request is within the configured budget.
    pub allowed: bool,
    /// Requests left in the current window.
    pub remaining: u64,
    /// Time until the current window expires.
    pub retry_after: Duration,
}

#[derive(Clone)]
enum Backend {
    Redis(redis::Client),
    OfflineMock(Arc<DashMap<String, (Instant, u64)>>),
}

/// Atomic fixed-window limiter shared through Redis.
///
/// Empty and `mock_*` URLs select an explicit process-local fallback so offline
/// tests remain deterministic. Production startup should call
/// [`Self::require_distributed`] to reject that mode.
#[derive(Clone)]
pub struct RedisRateLimiter {
    max_requests: u64,
    window: Duration,
    window_ms: i64,
    key_prefix: String,
    backend: Backend,
}

impl RedisRateLimiter {
    /// Builds a limiter without opening a network connection.
    pub fn new(
        redis_url: impl Into<String>,
        key_prefix: impl Into<String>,
        max_requests: u64,
        window: Duration,
    ) -> Result<Self, RateLimitError> {
        if max_requests == 0 || max_requests > i64::MAX as u64 {
            return Err(RateLimitError::InvalidConfiguration("max_requests"));
        }
        if window.is_zero() || window > MAX_WINDOW {
            return Err(RateLimitError::InvalidConfiguration("window"));
        }
        let window_ms = i64::try_from(window.as_millis())
            .map_err(|_| RateLimitError::InvalidConfiguration("window"))?;
        let key_prefix = key_prefix.into();
        if key_prefix.is_empty()
            || key_prefix.len() > 128
            || !key_prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
        {
            return Err(RateLimitError::InvalidConfiguration("key_prefix"));
        }

        let redis_url = redis_url.into();
        let backend = if redis_url.is_empty() || redis_url.starts_with("mock_") {
            Backend::OfflineMock(Arc::new(DashMap::new()))
        } else {
            Backend::Redis(
                redis::Client::open(redis_url)
                    .map_err(|error| RateLimitError::Backend(error.to_string()))?,
            )
        };
        Ok(Self {
            max_requests,
            window,
            window_ms,
            key_prefix,
            backend,
        })
    }

    /// Reports whether checks use Redis or the explicit offline fallback.
    pub fn mode(&self) -> RedisRateLimitMode {
        match self.backend {
            Backend::Redis(_) => RedisRateLimitMode::Distributed,
            Backend::OfflineMock(_) => RedisRateLimitMode::OfflineMock,
        }
    }

    /// Fails startup when this instance is not backed by shared Redis state.
    pub fn require_distributed(&self) -> Result<(), RateLimitError> {
        if self.mode() == RedisRateLimitMode::Distributed {
            Ok(())
        } else {
            Err(RateLimitError::OfflineMockIsNotDistributed)
        }
    }

    /// Atomically consumes one request budget for a client-derived key.
    pub async fn check(&self, client_key: &str) -> Result<RateLimitDecision, RateLimitError> {
        if client_key.is_empty() || client_key.len() > MAX_CLIENT_KEY_BYTES {
            return Err(RateLimitError::InvalidConfiguration("client_key"));
        }
        let redis_key = self.redis_key(client_key);
        let decision = match &self.backend {
            Backend::Redis(client) => self.check_redis(client, &redis_key).await?,
            Backend::OfflineMock(store) => self.check_offline(store, &redis_key),
        };
        if !decision.allowed {
            SecurityStore::global().inc_rate_limit_blocks();
        }
        Ok(decision)
    }

    fn redis_key(&self, client_key: &str) -> String {
        let digest = Sha256::digest(client_key.as_bytes());
        format!("{}:{}", self.key_prefix, hex::encode(digest))
    }

    async fn check_redis(
        &self,
        client: &redis::Client,
        redis_key: &str,
    ) -> Result<RateLimitDecision, RateLimitError> {
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|error| RateLimitError::Backend(error.to_string()))?;
        let (current, ttl_ms): (i64, i64) = redis::cmd("EVAL")
            .arg(REDIS_FIXED_WINDOW_SCRIPT)
            .arg(1)
            .arg(redis_key)
            .arg(self.window_ms)
            .query_async(&mut connection)
            .await
            .map_err(|error| RateLimitError::Backend(error.to_string()))?;
        if current <= 0 || ttl_ms < 0 {
            return Err(RateLimitError::InvalidBackendResponse);
        }
        Ok(self.decision(current as u64, Duration::from_millis(ttl_ms as u64)))
    }

    fn check_offline(
        &self,
        store: &DashMap<String, (Instant, u64)>,
        redis_key: &str,
    ) -> RateLimitDecision {
        let now = Instant::now();
        let mut entry = store.entry(redis_key.to_string()).or_insert((now, 0));
        if now.saturating_duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 = entry.1.saturating_add(1);
        let retry_after = self
            .window
            .saturating_sub(now.saturating_duration_since(entry.0));
        self.decision(entry.1, retry_after)
    }

    fn decision(&self, current: u64, retry_after: Duration) -> RateLimitDecision {
        RateLimitDecision {
            allowed: current <= self.max_requests,
            remaining: self.max_requests.saturating_sub(current),
            retry_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TM-ACADEMY-12: offline credentials remain explicit and cannot be
    // mistaken for the distributed abuse-control boundary required in production.
    #[tokio::test]
    async fn offline_mode_is_explicit_deterministic_and_shared_across_clones() {
        let first =
            RedisRateLimiter::new("mock_rate_limit", "rullst:test", 2, Duration::from_secs(60))
                .expect("mock limiter");
        let second = first.clone();
        assert_eq!(first.mode(), RedisRateLimitMode::OfflineMock);
        assert!(matches!(
            first.require_distributed(),
            Err(RateLimitError::OfflineMockIsNotDistributed)
        ));
        assert!(first.check("learner-7").await.expect("first check").allowed);
        assert!(
            second
                .check("learner-7")
                .await
                .expect("second check")
                .allowed
        );
        assert!(!first.check("learner-7").await.expect("third check").allowed);
    }

    #[test]
    fn configuration_and_redis_key_material_fail_closed() {
        assert!(
            RedisRateLimiter::new(
                "mock_rate_limit",
                "unsafe prefix",
                1,
                Duration::from_secs(1)
            )
            .is_err()
        );
        let limiter =
            RedisRateLimiter::new("mock_rate_limit", "rullst:test", 1, Duration::from_secs(1))
                .expect("mock limiter");
        let key = limiter.redis_key("sensitive@example.com");
        assert!(key.starts_with("rullst:test:"));
        assert!(!key.contains("sensitive@example.com"));
        assert!(REDIS_FIXED_WINDOW_SCRIPT.contains("redis.call('INCR'"));
        assert!(REDIS_FIXED_WINDOW_SCRIPT.contains("redis.call('PEXPIRE'"));
    }
}
