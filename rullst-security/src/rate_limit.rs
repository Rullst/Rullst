use crate::telemetry::SecurityStore;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::net::IpAddr;
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

/// Process-global store used by [`is_rate_limited`].
///
/// Entries are keyed by client key and policy (see [`is_rate_limited`]), not
/// by the client key alone.
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
            StoreKey::Client,
        )
    }
}

/// Bounded in-memory fixed-window IP rate limiter checking request rates.
///
/// This legacy helper shares one process-global store across all callers.
/// Each `(client_ip, max_requests, window_duration)` combination has its own
/// budget: callers that use different policies for the same key neither share
/// a count nor reset each other's window. All policies share the global
/// 16,384-entry capacity, so prefer one [`RateLimiter`] instance per policy.
pub fn is_rate_limited(client_ip: &str, max_requests: u64, window_duration: Duration) -> bool {
    is_rate_limited_in(
        global_rate_limit_store(),
        RATE_LIMIT_ADMISSION.get_or_init(|| Mutex::new(AdmissionState::default())),
        client_ip,
        max_requests,
        window_duration,
        StoreKey::ClientAndPolicy,
    )
}

/// How a validated client key maps to a store entry.
#[derive(Clone, Copy)]
enum StoreKey {
    /// One policy owns the store, so the client key is sufficient.
    Client,
    /// Several policies share the store; scope the entry to the policy.
    ClientAndPolicy,
}

impl StoreKey {
    fn entry_key(self, client_ip: &str, max_requests: u64, window_duration: Duration) -> String {
        match self {
            Self::Client => client_ip.to_string(),
            // The numeric prefix is unambiguous for any client key.
            Self::ClientAndPolicy => format!(
                "{max_requests}/{}ns/{client_ip}",
                window_duration.as_nanos()
            ),
        }
    }
}

fn is_rate_limited_in(
    store: &DashMap<String, (Instant, AtomicU64)>,
    admission: &Mutex<AdmissionState>,
    client_ip: &str,
    max_requests: u64,
    window_duration: Duration,
    store_key: StoreKey,
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
    let entry_key = store_key.entry_key(client_ip, max_requests, window_duration);
    if !store.contains_key(&entry_key) && store.len() >= MAX_RATE_LIMIT_IDENTITIES {
        return record_block();
    }

    let mut entry = store
        .entry(entry_key)
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

/// Rate-limit key for a verified transport peer.
///
/// IPv4 peers are keyed per address. IPv6 peers are keyed per /64, the
/// smallest prefix normally delegated to one subscriber, so rotating source
/// addresses inside it neither yields fresh budgets nor fills the bounded
/// identity table. IPv4-mapped IPv6 addresses are keyed as IPv4.
fn peer_rate_limit_key(ip: IpAddr) -> String {
    match ip.to_canonical() {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => {
            let [a, b, c, d, ..] = address.segments();
            format!("{a:x}:{b:x}:{c:x}:{d:x}::/64")
        }
    }
}

/// Axum middleware enforcing a fixed window rate limit: by default 120
/// requests per minute for each IPv4 address or IPv6 /64 prefix of the
/// verified socket peer.
///
/// The process-wide table tracks at most 16,384 peer keys; while it is full of
/// unexpired windows, requests from a key it does not hold fail closed.
pub async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    static LIMITER: OnceLock<RateLimiter> = OnceLock::new();
    let Some(client_ip) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|connect_info| peer_rate_limit_key(connect_info.0.ip()))
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
mod tests;
