//! Anti-Timing Attack User Enumeration Guard (`rullst-security::timing_guard`).
//!
//! Reduces coarse timing differences on authentication, user lookup, and
//! password-reset endpoints by padding responses to a configured target with
//! random jitter. It is not a constant-time or side-channel-elimination proof.

use crate::telemetry::SecurityStore;
use axum::{extract::Request, middleware::Next, response::Response};
use std::time::{Duration, Instant};

/// Configuration for the Anti-Timing Attack Guard.
#[derive(Clone, Debug)]
pub struct TimingGuardConfig {
    /// Target minimum response duration before scheduler and transport effects.
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

    /// Runs [`synthetic_argon2_cpu_work`] when enabled, then pads to the
    /// configured minimum duration like [`Self::finish`].
    pub async fn finish_with_synthetic_work(self) {
        if self.config.enable_synthetic_cpu_cycles {
            synthetic_argon2_cpu_work();
        }
        self.finish().await;
    }
}

/// Runs 1,500 SHA-256 iterations so that an early return (for example, user
/// not found) still spends some CPU time.
///
/// This does not reproduce the time, memory or cache profile of a real
/// Argon2 or bcrypt verification, and it runs synchronously on the calling
/// thread. To hide whether an account exists, verify the submitted password
/// against a dummy hash with the same password hasher instead.
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

/// Runs `action` and pads its completion to the configured minimum duration
/// plus random jitter. Slower executions are not padded.
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

/// Axum middleware that pads every response it wraps to at least 250 ms plus
/// up to 20 ms of jitter (the default [`TimingGuardConfig`]).
///
/// Mount it on sensitive routes such as login, registration and password
/// reset. Responses slower than the target are not padded, so a slow code path
/// can still be distinguished; this reduces coarse timing differences only.
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
        // Wall-clock ceilings belong in benchmarks: sanitizer instrumentation
        // deliberately changes execution time by a large, host-dependent factor.
        synthetic_argon2_cpu_work();
    }

    #[test]
    fn default_configuration_is_security_conservative() {
        let config = TimingGuardConfig::default();
        assert_eq!(config.min_duration, Duration::from_millis(250));
        assert_eq!(config.max_jitter, Duration::from_millis(20));
        assert!(config.enable_synthetic_cpu_cycles);
    }

    #[tokio::test]
    async fn synthetic_finish_supports_enabled_and_disabled_modes() {
        for enabled in [false, true] {
            TimingScope::start(TimingGuardConfig {
                min_duration: Duration::ZERO,
                max_jitter: Duration::ZERO,
                enable_synthetic_cpu_cycles: enabled,
            })
            .finish_with_synthetic_work()
            .await;
        }
    }

    #[tokio::test]
    async fn middleware_returns_the_inner_response_after_padding() {
        use axum::{Router, body::Body, http::Request, middleware, routing::get};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/", get(|| async { "protected" }))
            .layer(middleware::from_fn(timing_guard_middleware));
        let started = Instant::now();
        let response = app
            .oneshot(
                Request::get("/")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("middleware request should complete");
        assert!(started.elapsed() >= Duration::from_millis(245));
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
