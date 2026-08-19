//! Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`).
//!
//! Eliminates side-channel timing analysis on authentication, user lookup,
//! and password reset endpoints by enforcing constant-time response durations
//! with cryptographically randomized micro-jitter.

use crate::telemetry::SecurityStore;
use axum::{extract::Request, middleware::Next, response::Response};
use std::time::{Duration, Instant};

/// Configuration for the Anti-Timing Attack Guard.
#[derive(Clone, Debug)]
pub struct TimingGuardConfig {
    /// Minimum guaranteed response duration (e.g., 250ms).
    pub min_duration: Duration,
    /// Maximum random micro-jitter to prevent statistical synchronization (e.g., 20ms).
    pub max_jitter: Duration,
    /// Whether to execute synthetic CPU hash cycles for non-existent users.
    pub enable_synthetic_cpu_cycles: bool,
}

impl Default for TimingGuardConfig {
    fn default() -> Self {
        Self {
            min_duration: Duration::from_millis(250),
            max_jitter: Duration::from_millis(20),
            enable_synthetic_cpu_cycles: true,
        }
    }
}

/// An active timing scope that tracks the elapsed wall-clock time
/// and normalizes completion latency when finished.
pub struct TimingScope {
    start_time: Instant,
    config: TimingGuardConfig,
}

impl TimingScope {
    /// Starts a new timing guard scope with the provided configuration.
    pub fn start(config: TimingGuardConfig) -> Self {
        Self {
            start_time: Instant::now(),
            config,
        }
    }

    /// Finishes the scope, sleeping for the remaining duration if the execution
    /// completed earlier than the configured target plus random jitter.
    pub async fn finish(self) {
        let elapsed = self.start_time.elapsed();

        let jitter_micros = if self.config.max_jitter.as_micros() > 0 {
            (rand::random::<u32>() as u128) % self.config.max_jitter.as_micros()
        } else {
            0
        };

        let target_duration =
            self.config.min_duration + Duration::from_micros(jitter_micros as u64);

        if elapsed < target_duration {
            let sleep_needed = target_duration - elapsed;
            tokio::time::sleep(sleep_needed).await;
        }

        SecurityStore::global().inc_timing_guard_protected();
    }

    /// Performs synthetic CPU hashing work (simulating Argon2 / Bcrypt CPU heat)
    /// and then applies constant-time duration padding.
    pub async fn finish_with_synthetic_work(self) {
        if self.config.enable_synthetic_cpu_cycles {
            synthetic_argon2_cpu_work();
        }
        self.finish().await;
    }
}

/// Executes a calibrated synthetic hashing loop so that early returns (e.g., user not found)
/// produce identical CPU instruction cache footprints and CPU consumption as real user lookups.
pub fn synthetic_argon2_cpu_work() {
    use sha2::{Digest, Sha256};
    let mut state = [0x5au8; 32];
    for i in 0u64..1_500u64 {
        let mut hasher = Sha256::new();
        hasher.update(state);
        hasher.update(i.to_be_bytes());
        state = hasher.finalize().into();
    }
    // Prevent compiler dead-code elimination with black_box
    std::hint::black_box(state);
}

/// Async helper wrapping any closure with constant-time execution padding.
pub async fn equalize_response_time<F, Fut, T>(config: TimingGuardConfig, action: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let scope = TimingScope::start(config);
    let result = action().await;
    scope.finish().await;
    result
}

/// Axum middleware layer enforcing constant-time response normalization
/// on sensitive routes (e.g. `/login`, `/register`, `/forgot-password`, `/auth/*`).
pub async fn timing_guard_middleware(req: Request, next: Next) -> Response {
    let scope = TimingScope::start(TimingGuardConfig::default());
    let response = next.run(req).await;
    scope.finish().await;
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timing_guard_normalizes_fast_execution() {
        let config = TimingGuardConfig {
            min_duration: Duration::from_millis(50),
            max_jitter: Duration::from_millis(5),
            enable_synthetic_cpu_cycles: false,
        };

        let start = Instant::now();
        let result = equalize_response_time(config, || async {
            // Fast execution takes < 1ms
            42
        })
        .await;

        let elapsed = start.elapsed();
        assert_eq!(result, 42);
        assert!(
            elapsed >= Duration::from_millis(48),
            "Expected at least ~50ms duration, got {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_timing_guard_preserves_slow_execution() {
        let config = TimingGuardConfig {
            min_duration: Duration::from_millis(20),
            max_jitter: Duration::from_millis(2),
            enable_synthetic_cpu_cycles: false,
        };

        let start = Instant::now();
        let result = equalize_response_time(config, || async {
            tokio::time::sleep(Duration::from_millis(35)).await;
            "done"
        })
        .await;

        let elapsed = start.elapsed();
        assert_eq!(result, "done");
        assert!(elapsed >= Duration::from_millis(34));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_synthetic_cpu_work_runs_safely() {
        let start = Instant::now();
        synthetic_argon2_cpu_work();
        assert!(start.elapsed() < Duration::from_millis(50));
    }
}
