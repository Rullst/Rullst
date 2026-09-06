use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(feature = "redis-rate-limit")]
mod redis;
#[cfg(feature = "redis-rate-limit")]
pub use redis::{RateLimitDecision, RedisRateLimitMode, RedisRateLimiter};

static RATE_LIMIT_STORE: OnceLock<DashMap<String, (Instant, AtomicU64)>> = OnceLock::new();
static RATE_LIMIT_ADMISSION: OnceLock<Mutex<AdmissionState>> = OnceLock::new();
const MAX_RATE_LIMIT_IDENTITIES: usize = 16_384;
const MAX_RATE_LIMIT_KEY_BYTES: usize = 256;

#[derive(Debug)]
struct AdmissionState {
    last_cleanup: Instant,
    longest_window: Duration,
}

impl Default for AdmissionState {
    fn default() -> Self {
        Self {
            last_cleanup: Instant::now(),
            longest_window: Duration::ZERO,
        }
    }
}

pub fn global_rate_limit_store() -> &'static DashMap<String, (Instant, AtomicU64)> {
    RATE_LIMIT_STORE.get_or_init(DashMap::new)
}

/// Supported rate limiting backend strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateLimitBackend {
    /// Bounded process-local fixed window using DashMap.
    #[default]
    Memory,
    /// Reserved distributed mode. Construction currently returns
    /// [`RateLimitError::DistributedBackendUnsupported`].
    Distributed,
}

/// Unsupported or invalid rate-limit backend configuration.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RateLimitError {
    /// No distributed backend is implemented in this release.
    #[error("distributed rate limiting is not implemented; configure a real shared backend")]
    DistributedBackendUnsupported,
    /// A distributed limiter configuration is invalid.
    #[error("invalid distributed rate-limit configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// A shared backend operation failed.
    #[error("distributed rate-limit backend failed: {0}")]
    Backend(String),
    /// The backend returned a value outside the versioned protocol.
    #[error("distributed rate-limit backend returned an invalid response")]
    InvalidBackendResponse,
    /// A deterministic offline mock was used where a shared backend is required.
    #[error("offline rate-limit mock is process-local and not distributed")]
    OfflineMockIsNotDistributed,
}

/// Configurable builder for application rate limiters.
/// Each shared local store retains at most 16,384 identities with keys up to
/// 256 bytes. Invalid/zero budgets and exhausted admission fail closed.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Maximum accepted requests in one window.
    pub max_requests: u64,
    /// Fixed rate-limit window duration.
    pub window: Duration,
    /// Selected backend mode.
    pub backend: RateLimitBackend,
    store: Arc<DashMap<String, (Instant, AtomicU64)>>,
    admission: Arc<Mutex<AdmissionState>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            max_requests: 120,
            window: Duration::from_secs(60),
            backend: RateLimitBackend::Memory,
            store: Arc::new(DashMap::new()),
            admission: Arc::new(Mutex::new(AdmissionState::default())),
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
            store: Arc::new(DashMap::new()),
            admission: Arc::new(Mutex::new(AdmissionState::default())),
        }
    }

    /// Legacy distributed selection. Requests fail closed because the backend
    /// is not implemented; use [`Self::try_with_distributed`] to detect this at
    /// startup.
    #[deprecated(
        since = "12.0.0",
        note = "use try_with_distributed and handle the error"
    )]
    pub fn with_distributed(mut self) -> Self {
        self.backend = RateLimitBackend::Distributed;
        self
    }

    /// Attempts to configure a distributed backend.
    pub fn try_with_distributed(self) -> Result<Self, RateLimitError> {
        Err(RateLimitError::DistributedBackendUnsupported)
    }

    /// Checks if a client IP or key has exceeded the rate limit.
    pub fn check(&self, key: &str) -> bool {
        if self.backend == RateLimitBackend::Distributed {
            return true;
        }
        is_rate_limited_in(
            &self.store,
            &self.admission,
            key,
            self.max_requests,
            self.window,
        )
    }
}

/// Bounded in-memory fixed-window IP rate limiter checking request rates.
pub fn is_rate_limited(client_ip: &str, max_requests: u64, window_duration: Duration) -> bool {
    is_rate_limited_in(
        global_rate_limit_store(),
        RATE_LIMIT_ADMISSION.get_or_init(|| Mutex::new(AdmissionState::default())),
        client_ip,
        max_requests,
        window_duration,
    )
}

fn is_rate_limited_in(
    store: &DashMap<String, (Instant, AtomicU64)>,
    admission: &Mutex<AdmissionState>,
    client_ip: &str,
    max_requests: u64,
    window_duration: Duration,
) -> bool {
    if max_requests == 0
        || window_duration.is_zero()
        || client_ip.is_empty()
        || client_ip.len() > MAX_RATE_LIMIT_KEY_BYTES
    {
        return record_block();
    }
    // Serialize capacity checks and insertion across clones. A len check
    // outside this lock can over-admit concurrent new identities.
    let Ok(mut admission) = admission.lock() else {
        return record_block();
    };
    let now = Instant::now();
    admission.longest_window = admission.longest_window.max(window_duration);
    if now.saturating_duration_since(admission.last_cleanup)
        >= admission.longest_window.min(Duration::from_secs(1))
    {
        // The legacy global API can serve differing windows; never expire
        // another caller's longer active window using this request's policy.
        store.retain(|_, (start, _)| {
            now.saturating_duration_since(*start) < admission.longest_window
        });
        admission.last_cleanup = now;
    }
    if !store.contains_key(client_ip) && store.len() >= MAX_RATE_LIMIT_IDENTITIES {
        return record_block();
    }

    let mut entry = store
        .entry(client_ip.to_string())
        .or_insert_with(|| (now, AtomicU64::new(0)));

    let (start_time, count) = entry.value_mut();
    if now.saturating_duration_since(*start_time) >= window_duration {
        *start_time = now;
        count.store(0, Ordering::Relaxed);
    }
    let current = count.load(Ordering::Relaxed);
    if current >= max_requests {
        return record_block();
    }
    count.store(current + 1, Ordering::Relaxed);
    false
}

fn record_block() -> bool {
    SecurityStore::global().inc_rate_limit_blocks();
    true
}

/// Axum middleware enforcing a fixed window rate limit (default 120 req / minute per IP).
pub async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    static LIMITER: OnceLock<RateLimiter> = OnceLock::new();
    let Some(client_ip) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|connect_info| connect_info.0.ip().to_string())
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Peer address unavailable; rate limiter is not safely configured",
        )
            .into_response();
    };

    if LIMITER.get_or_init(RateLimiter::default).check(&client_ip) {
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
    use axum::{Router, body::Body, extract::ConnectInfo, http::Request, middleware, routing::get};
    use tower::ServiceExt;

    #[test]
    fn zero_budget_denies_requests_even_after_a_window_reset() {
        let limiter = RateLimiter::new(0, Duration::from_millis(1));
        limiter.store.insert(
            "no-budget".to_string(),
            (Instant::now() - Duration::from_secs(1), AtomicU64::new(99)),
        );
        assert!(limiter.check("no-budget"));
        assert!(RateLimiter::new(1, Duration::ZERO).check("zero-window"));
    }

    #[test]
    fn arbitrary_client_keys_cannot_grow_memory_without_a_bound() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check(&"x".repeat(257)));
        assert!(limiter.check(""));
        for index in 0..16_384 {
            assert!(!limiter.check(&format!("bounded-client-{index}")));
        }
        assert!(limiter.check("over-capacity-client"));
        assert_eq!(limiter.store.len(), 16_384);
    }

    #[test]
    fn counter_saturation_cannot_reopen_the_request_budget() {
        let limiter = RateLimiter::new(u64::MAX, Duration::from_secs(60));
        limiter.store.insert(
            "saturated".to_string(),
            (Instant::now(), AtomicU64::new(u64::MAX)),
        );
        assert!(limiter.check("saturated"));
    }

    #[test]
    fn expired_identities_are_reclaimed_without_a_background_runtime() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let expired = Instant::now() - Duration::from_secs(120);
        for index in 0..MAX_RATE_LIMIT_IDENTITIES {
            limiter
                .store
                .insert(format!("expired-{index}"), (expired, AtomicU64::new(1)));
        }
        limiter.admission.lock().unwrap().last_cleanup = expired;
        assert!(!limiter.check("new-identity"));
        assert_eq!(limiter.store.len(), 1);
    }

    #[test]
    fn concurrent_new_identities_share_one_remaining_slot() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        for index in 1..MAX_RATE_LIMIT_IDENTITIES {
            limiter.store.insert(
                format!("occupied-{index}"),
                (Instant::now(), AtomicU64::new(1)),
            );
        }
        let barrier = std::sync::Barrier::new(16);
        let admitted = AtomicU64::new(0);
        std::thread::scope(|scope| {
            for index in 0..16 {
                let (limiter, barrier, admitted) = (&limiter, &barrier, &admitted);
                scope.spawn(move || {
                    barrier.wait();
                    if !limiter.check(&format!("contender-{index}")) {
                        admitted.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        assert_eq!(admitted.load(Ordering::Relaxed), 1);
        assert_eq!(limiter.store.len(), MAX_RATE_LIMIT_IDENTITIES);
    }

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
        let limiter = RateLimiter::new(5, Duration::from_secs(1));
        assert_eq!(limiter.max_requests, 5);
        let key = "10.0.0.1";
        for _ in 0..5 {
            assert!(!limiter.check(key));
        }
        assert!(limiter.check(key));
        assert!(matches!(
            RateLimiter::new(5, Duration::from_secs(1)).try_with_distributed(),
            Err(RateLimitError::DistributedBackendUnsupported)
        ));
    }

    #[test]
    #[allow(deprecated)]
    fn distributed_compatibility_builder_fails_closed() {
        let limiter = RateLimiter::new(10, Duration::from_secs(1)).with_distributed();
        assert_eq!(limiter.backend, RateLimitBackend::Distributed);
        assert!(limiter.check("distributed-client"));
    }

    #[test]
    fn expired_window_resets_request_count() {
        let limiter = RateLimiter::new(1, Duration::from_millis(1));
        limiter.store.insert(
            "expired-client".to_string(),
            (Instant::now() - Duration::from_secs(1), AtomicU64::new(99)),
        );
        assert!(!limiter.check("expired-client"));
        assert!(limiter.check("expired-client"));
    }

    #[tokio::test]
    async fn middleware_fails_closed_without_peer_address() {
        let app = Router::new()
            .route("/", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(rate_limit_middleware));
        let response = app
            .oneshot(
                Request::get("/")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("middleware request should complete");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn middleware_returns_too_many_requests_after_default_limit() {
        let app = Router::new()
            .route("/", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(rate_limit_middleware));
        let peer = ConnectInfo(
            "192.0.2.217:4123"
                .parse::<std::net::SocketAddr>()
                .expect("socket address should be valid"),
        );

        for expected in std::iter::repeat_n(StatusCode::OK, 120)
            .chain(std::iter::once(StatusCode::TOO_MANY_REQUESTS))
        {
            let mut request = Request::get("/")
                .body(Body::empty())
                .expect("request should be valid");
            request.extensions_mut().insert(peer);
            let response = app
                .clone()
                .oneshot(request)
                .await
                .expect("middleware request should complete");
            assert_eq!(response.status(), expected);
        }
    }
}
