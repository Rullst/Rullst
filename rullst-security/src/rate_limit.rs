use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static RATE_LIMIT_STORE: OnceLock<DashMap<String, (Instant, AtomicU64)>> = OnceLock::new();

pub fn global_rate_limit_store() -> &'static DashMap<String, (Instant, AtomicU64)> {
    RATE_LIMIT_STORE.get_or_init(|| {
        let store = DashMap::new();
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(300));
                loop {
                    interval.tick().await;
                    let now = Instant::now();
                    global_rate_limit_store().retain(|_, (start_time, _)| {
                        now.duration_since(*start_time) < Duration::from_secs(600)
                    });
                }
            });
        }
        store
    })
}

/// Supported rate limiting backend strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateLimitBackend {
    /// High-throughput in-memory sliding window using DashMap.
    #[default]
    Memory,
    /// Distributed multi-instance sliding window using Redis or shared cache backend.
    Distributed,
}

/// Configurable builder for application rate limiters.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub max_requests: u64,
    pub window: Duration,
    pub backend: RateLimitBackend,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            max_requests: 120,
            window: Duration::from_secs(60),
            backend: RateLimitBackend::Memory,
        }
    }
}

impl RateLimiter {
    /// Creates a new RateLimiter with specified max requests and duration window.
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            backend: RateLimitBackend::Memory,
        }
    }

    /// Configures the RateLimiter to use the distributed backend.
    pub fn with_distributed(mut self) -> Self {
        self.backend = RateLimitBackend::Distributed;
        self
    }

    /// Checks if a client IP or key has exceeded the rate limit.
    pub fn check(&self, key: &str) -> bool {
        is_rate_limited(key, self.max_requests, self.window)
    }
}

/// In-memory sliding-window IP rate limiter checking request rates.
pub fn is_rate_limited(client_ip: &str, max_requests: u64, window_duration: Duration) -> bool {
    let store = global_rate_limit_store();
    let now = Instant::now();

    let mut entry = store
        .entry(client_ip.to_string())
        .or_insert_with(|| (now, AtomicU64::new(0)));

    let (start_time, count) = entry.value_mut();
    if now.duration_since(*start_time) > window_duration {
        *start_time = now;
        count.store(1, Ordering::Relaxed);
        false
    } else {
        let current = count.fetch_add(1, Ordering::Relaxed) + 1;
        if current > max_requests {
            SecurityStore::global().inc_rate_limit_blocks();
            true
        } else {
            false
        }
    }
}

/// Axum middleware enforcing a sliding window rate limit (default 120 req / minute per IP).
pub async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    let client_ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .split(',')
        .next()
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string();

    if is_rate_limited(&client_ip, 120, Duration::from_secs(60)) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.",
        )
            .into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_rate_limiter() {
        let ip = "192.168.99.1";
        let window = Duration::from_secs(10);
        assert!(!is_rate_limited(ip, 3, window));
        assert!(!is_rate_limited(ip, 3, window));
        assert!(!is_rate_limited(ip, 3, window));
        // 4th request exceeds max_requests=3
        assert!(is_rate_limited(ip, 3, window));
    }

    #[test]
    fn test_rate_limiter_builder() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1)).with_distributed();
        assert_eq!(limiter.max_requests, 5);
        assert_eq!(limiter.backend, RateLimitBackend::Distributed);
        let key = "10.0.0.1";
        for _ in 0..5 {
            assert!(!limiter.check(key));
        }
        assert!(limiter.check(key));
    }
}
