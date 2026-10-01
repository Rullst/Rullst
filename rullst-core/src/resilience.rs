use axum::extract::Request;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MONITORS_IDLE: u8 = 0;
const MONITORS_RUNNING: u8 = 1;
const MONITORS_SHUT_DOWN: u8 = 2;

#[path = "resilience_probe.rs"]
mod db_probe;

#[path = "resilience_rate_limit.rs"]
mod rate_limit;
#[path = "resilience_shield.rs"]
mod shield;

use rate_limit::refill_token_count;
pub use rate_limit::{default_key_extractor, rate_limit_middleware};
pub use shield::backpressure_middleware;

/// Failures that can occur while managing Traffic Shield monitors.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrafficShieldError {
    /// Monitoring was started outside an active Tokio runtime.
    #[error("Traffic Shield monitors require an active Tokio runtime")]
    RuntimeUnavailable,
    /// Monitoring cannot be restarted after explicit shutdown.
    #[error("Traffic Shield monitors have already been shut down")]
    AlreadyShutDown,
}

/// Configures the limits and behavior of the Adaptive Backpressure & Resilient Traffic Shielding.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct TrafficShieldConfig {
    /// Maximum Tokio event-loop lag before load shedding activates. Default: 100ms.
    pub max_event_loop_lag: Duration,
    /// Maximum DB probe round-trip latency before load shedding activates. Default: 500ms.
    /// A probe still running at this latency is abandoned and recorded as taking it.
    pub max_db_latency: Duration,
    /// Maximum number of concurrent in-flight requests before load shedding activates. Default: 1000.
    pub max_active_requests: usize,
    /// If `true`, spawns a background task that probes the DB with `SELECT 1` every second to measure latency.
    pub enable_db_probe: bool,
}

impl Default for TrafficShieldConfig {
    fn default() -> Self {
        Self {
            max_event_loop_lag: Duration::from_millis(100),
            max_db_latency: Duration::from_millis(500),
            max_active_requests: 1000,
            enable_db_probe: true,
        }
    }
}

impl TrafficShieldConfig {
    /// Creates a `TrafficShieldConfig` with sensible production defaults:
    /// 100ms event-loop lag threshold, 500ms DB latency threshold, 1000 max active requests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum event loop latency/lag allowed before shedding load.
    pub fn with_max_event_loop_lag(mut self, lag: Duration) -> Self {
        self.max_event_loop_lag = lag;
        self
    }

    /// Sets the maximum database probe query latency allowed before shedding load.
    pub fn with_max_db_latency(mut self, latency: Duration) -> Self {
        self.max_db_latency = latency;
        self
    }

    /// Sets the maximum simultaneous requests allowed.
    pub fn with_max_active_requests(mut self, limit: usize) -> Self {
        self.max_active_requests = limit;
        self
    }

    /// Configures whether to run a background database probe loop (`SELECT 1`).
    pub fn with_db_probe(mut self, enable: bool) -> Self {
        self.enable_db_probe = enable;
        self
    }
}

/// The core resilience monitor that performs real-time diagnostics on Tokio latency and Database roundtrip speeds.
#[derive(Clone)]
pub struct TrafficShield {
    pub(crate) config: TrafficShieldConfig,
    event_loop_lag_ms: Arc<AtomicU64>,
    db_latency_ms: Arc<AtomicU64>,
    active_requests: Arc<AtomicUsize>,
    monitors: Arc<TrafficShieldMonitors>,
}

struct TrafficShieldMonitors {
    state: AtomicU8,
    shutdown: Arc<tokio::sync::Notify>,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    /// At most one load-shedding line per interval, shared by clones.
    shed_log: shield_log::LogThrottle,
    /// At most one "monitoring unavailable" line per interval.
    unavailable_log: shield_log::LogThrottle,
}

fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Extensible configuration for the Token-Bucket Rate Limiter.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum number of tokens (burst capacity). A request consumes 1 token.
    pub max_tokens: f64,
    /// Token refill rate in **tokens per second**. Use the `per_second`, `per_minute`, `per_hour` factories.
    pub refill_rate: f64,
}

impl RateLimitConfig {
    /// Creates a new `RateLimitConfig` with explicit burst capacity and refill rate.
    /// For convenience, prefer the factory methods: `per_second`, `per_minute`, `per_hour`.
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            max_tokens,
            refill_rate,
        }
    }

    /// Creates a limit of N requests per second.
    pub fn per_second(limit: f64) -> Self {
        Self::new(limit, limit)
    }

    /// Creates a limit of N requests per minute.
    pub fn per_minute(limit: f64) -> Self {
        Self::new(limit, limit / 60.0)
    }

    /// Creates a limit of N requests per hour.
    pub fn per_hour(limit: f64) -> Self {
        Self::new(limit, limit / 3600.0)
    }
}

#[derive(Clone, Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

/// Thread-safe Token-Bucket rate limiter powered by Shared-Memory DashMap.
///
/// Clones share one bucket map. It tracks at most 100,000 keys: buckets that
/// have refilled completely are dropped (a new bucket behaves identically),
/// and beyond the cap the least recently used buckets are evicted, which gives
/// those clients a fresh burst. Keys longer than 128 bytes are stored as a
/// SHA-256 digest, so each bucket's key takes at most 128 bytes.
#[derive(Clone)]
pub struct RateLimiter {
    pub(crate) config: RateLimitConfig,
    buckets: Arc<DashMap<String, TokenBucket>>,
    key_extractor: Arc<dyn Fn(&Request) -> String + Send + Sync>,
    new_keys: Arc<AtomicUsize>,
    max_buckets: usize,
}

#[cfg(test)]
#[path = "resilience_tests.rs"]
mod tests;

#[path = "resilience_log.rs"]
mod shield_log;

#[path = "resilience_buckets.rs"]
mod buckets;

#[cfg(test)]
#[path = "resilience_contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "resilience_rate_limit_tests.rs"]
mod rate_limit_tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::shield::{TrafficPressure, classify_traffic_pressure};
    use super::*;

    #[kani::proof]
    fn verify_token_bucket_math_safety() {
        let max_tokens: f64 = kani::any();
        let refill_rate: f64 = kani::any();
        let current_tokens: f64 = kani::any();
        let elapsed_secs: f64 = kani::any();

        // Constrain to reasonable limits to avoid trivial infinity
        kani::assume(max_tokens > 0.0 && max_tokens < 1_000_000.0);
        kani::assume(refill_rate >= 0.0 && refill_rate < 100_000.0);
        kani::assume(current_tokens >= 0.0 && current_tokens <= max_tokens);
        kani::assume(elapsed_secs >= 0.0 && elapsed_secs < 31_536_000.0); // Up to 1 year

        let config = RateLimitConfig::new(max_tokens, refill_rate);
        let final_tokens = refill_token_count(current_tokens, elapsed_secs, &config);

        // Prove that the math never yields NaN or Infinity under normal constraints
        assert!(!final_tokens.is_nan());
        assert!(!final_tokens.is_infinite());
        assert!(final_tokens >= 0.0);
        assert!(final_tokens <= max_tokens);
    }

    #[kani::proof]
    fn verify_traffic_shield_thresholds() {
        let max_event_loop_lag_ns: u64 = kani::any();
        let max_db_latency_ns: u64 = kani::any();
        let max_active_requests: usize = kani::any();
        let enable_db_probe: bool = kani::any();
        let lag_ns: u64 = kani::any();
        let db_latency_ns: u64 = kani::any();
        let active: usize = kani::any();

        let config = TrafficShieldConfig {
            max_event_loop_lag: Duration::from_nanos(max_event_loop_lag_ns),
            max_db_latency: Duration::from_nanos(max_db_latency_ns),
            max_active_requests,
            enable_db_probe,
        };
        let lag = Duration::from_nanos(lag_ns);
        let db_latency = Duration::from_nanos(db_latency_ns);
        let pressure = classify_traffic_pressure(&config, lag, db_latency, active);

        let critical = lag >= config.max_event_loop_lag
            || (enable_db_probe && db_latency >= config.max_db_latency)
            || active >= max_active_requests;
        let moderate = lag >= config.max_event_loop_lag / 2
            || (enable_db_probe && db_latency >= config.max_db_latency / 2)
            || active >= max_active_requests / 2;

        match pressure {
            TrafficPressure::Critical => {
                assert!(critical);
            }
            TrafficPressure::Moderate => {
                assert!(!critical);
                assert!(moderate);
            }
            TrafficPressure::Normal => {
                assert!(!critical);
                assert!(!moderate);
            }
        }
    }
}
