use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Configures the limits and behavior of the Adaptive Backpressure & Resilient Traffic Shielding.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct TrafficShieldConfig {
    /// Maximum Tokio event-loop lag before load shedding activates. Default: 100ms.
    pub max_event_loop_lag: Duration,
    /// Maximum DB probe round-trip latency before load shedding activates. Default: 500ms.
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
}

impl TrafficShield {
    /// Creates a new `TrafficShield`, starts background monitoring goroutines for event-loop lag
    /// and (optionally) database latency, and returns the ready-to-use shield instance.
    pub fn new(config: TrafficShieldConfig) -> Self {
        let shield = Self {
            config,
            event_loop_lag_ms: Arc::new(AtomicU64::new(0)),
            db_latency_ms: Arc::new(AtomicU64::new(0)),
            active_requests: Arc::new(AtomicUsize::new(0)),
        };

        shield.spawn_monitors();
        shield
    }

    #[cfg_attr(mutants, mutants::skip)]
    fn spawn_monitors(&self) {
        let lag_ms = self.event_loop_lag_ms.clone();
        // 1. Event Loop Lag Diagnostic Task
        tokio::spawn(async move {
            let interval = Duration::from_millis(100);
            loop {
                let start = Instant::now();
                tokio::time::sleep(interval).await;
                let elapsed = start.elapsed();
                let lag = if elapsed > interval {
                    elapsed - interval
                } else {
                    Duration::from_millis(0)
                };
                lag_ms.store(lag.as_millis() as u64, Ordering::Relaxed);
            }
        });

        // 2. Database Probe Diagnostic Task
        if self.config.enable_db_probe {
            let db_lat_ms = self.db_latency_ms.clone();
            tokio::spawn(async move {
                let interval = Duration::from_millis(1000);
                loop {
                    tokio::time::sleep(interval).await;
                    if let Some(pool) = crate::db::safe_pool() {
                        let start = Instant::now();
                        let res = sqlx::query("SELECT 1").execute(pool).await;
                        match res {
                            Ok(_) => {
                                let latency = start.elapsed();
                                db_lat_ms.store(latency.as_millis() as u64, Ordering::Relaxed);
                            }
                            Err(_) => {
                                db_lat_ms.store(9999, Ordering::Relaxed);
                            }
                        }
                    } else {
                        db_lat_ms.store(0, Ordering::Relaxed);
                    }
                }
            });
        }
    }

    /// Returns the most recently measured Tokio event-loop lag as a `Duration`.
    /// Updated every 100ms by the background monitor task.
    pub fn event_loop_lag(&self) -> Duration {
        Duration::from_millis(self.event_loop_lag_ms.load(Ordering::Relaxed))
    }

    /// Returns the most recently measured database probe round-trip latency as a `Duration`.
    /// Returns `Duration::ZERO` if `enable_db_probe` is `false` or the pool is uninitialized.
    pub fn db_latency(&self) -> Duration {
        Duration::from_millis(self.db_latency_ms.load(Ordering::Relaxed))
    }

    /// Returns the current count of in-flight HTTP requests being tracked by this shield.
    pub fn active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }
}

/// Router-level protection middleware that tracks load timing and drops requests under critical saturation.
#[cfg_attr(mutants, mutants::skip)]
pub async fn backpressure_middleware(shield: TrafficShield, req: Request, next: Next) -> Response {
    let active = shield.active_requests.fetch_add(1, Ordering::SeqCst);

    let max_active = shield.config.max_active_requests;
    let lag = shield.event_loop_lag();
    let db_lat = shield.db_latency();

    let is_critical_cpu = lag >= shield.config.max_event_loop_lag;
    let is_critical_db = shield.config.enable_db_probe && db_lat >= shield.config.max_db_latency;
    let is_critical_active = active >= max_active;

    if is_critical_cpu || is_critical_db || is_critical_active {
        shield.active_requests.fetch_sub(1, Ordering::SeqCst);

        eprintln!(
            "⚠️ [Rullst Backpressure] Load shedding active! CPU lag: {:?}, DB latency: {:?}, Active requests: {}",
            lag, db_lat, active
        );

        match Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(axum::http::header::RETRY_AFTER, "5")
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(axum::body::Body::from(
                "Service Temporarily Saturated. Please try again soon.",
            )) {
            Ok(res) => return res,
            Err(_) => {
                let mut res = Response::new(axum::body::Body::empty());
                *res.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                return res;
            }
        }
    }

    let is_moderate_cpu = lag >= shield.config.max_event_loop_lag / 2;
    let is_moderate_db =
        shield.config.enable_db_probe && db_lat >= shield.config.max_db_latency / 2;
    let is_moderate_active = active >= max_active / 2;

    if is_moderate_cpu || is_moderate_db || is_moderate_active {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let response = next.run(req).await;
    shield.active_requests.fetch_sub(1, Ordering::SeqCst);
    response
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
    /// For convenience, prefer the factory methods: [`per_second`], [`per_minute`], [`per_hour`].
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
#[derive(Clone)]
pub struct RateLimiter {
    pub(crate) config: RateLimitConfig,
    buckets: Arc<DashMap<String, TokenBucket>>,
    key_extractor: Arc<dyn Fn(&Request) -> String + Send + Sync>,
}

impl RateLimiter {
    /// Creates a new `RateLimiter` from the given config, using IP address as the default key.
    /// The key extractor resolves `X-Forwarded-For`, `X-Real-IP`, or the peer `SocketAddr` in that order.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(DashMap::new()),
            key_extractor: Arc::new(default_key_extractor),
        }
    }

    /// Overrides the key extraction method (e.g. to limit by username or auth token).
    pub fn with_key_extractor<F>(mut self, extractor: F) -> Self
    where
        F: Fn(&Request) -> String + Send + Sync + 'static,
    {
        self.key_extractor = Arc::new(extractor);
        self
    }

    /// Evaluates if the bucket for `key` can consume 1 token, refilling dynamic tokens incrementally.
    pub fn check_and_consume(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entry = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket {
                tokens: self.config.max_tokens,
                last_refill: now,
            });

        let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
        let new_tokens = entry.tokens + elapsed * self.config.refill_rate;
        entry.tokens = new_tokens.min(self.config.max_tokens);
        entry.last_refill = now;

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Default key extractor: checks X-Forwarded-For, X-Real-IP, and peer SocketAddr.
pub fn default_key_extractor(req: &Request) -> String {
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(first_ip) = s.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return s.trim().to_string();
        }
    }

    if let Some(conn_info) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return conn_info.0.ip().to_string();
    }

    "anonymous".to_string()
}

/// Native Axum middleware enforcing rate limiting.
pub async fn rate_limit_middleware(limiter: RateLimiter, req: Request, next: Next) -> Response {
    let key = (limiter.key_extractor)(&req);
    if limiter.check_and_consume(&key) {
        next.run(req).await
    } else {
        match Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(axum::body::Body::from(
                "Rate limit exceeded. Please try again later.",
            )) {
            Ok(res) => res,
            Err(_) => {
                let mut res = Response::new(axum::body::Body::empty());
                *res.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                res
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::http::Request;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn test_default_key_extractor() {
        let req1 = Request::builder()
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(default_key_extractor(&req1), "192.168.1.1");

        let req2 = Request::builder()
            .header("x-real-ip", "10.0.0.2")
            .body(axum::body::Body::empty())
            .unwrap();
        assert_eq!(default_key_extractor(&req2), "10.0.0.2");

        let mut req3 = Request::builder().body(axum::body::Body::empty()).unwrap();
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        req3.extensions_mut()
            .insert(axum::extract::ConnectInfo(socket));
        assert_eq!(default_key_extractor(&req3), "127.0.0.1");

        let req4 = Request::builder().body(axum::body::Body::empty()).unwrap();
        assert_eq!(default_key_extractor(&req4), "anonymous");
    }

    #[tokio::test]
    async fn test_traffic_shield_active_requests() {
        let config = TrafficShieldConfig::new().with_db_probe(false);
        let shield = TrafficShield::new(config);

        assert_eq!(shield.active_requests(), 0);
        shield
            .active_requests
            .fetch_add(5, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(shield.active_requests(), 5);
    }

    #[test]
    fn test_rate_limit_config_per_minute() {
        let config = RateLimitConfig::per_minute(60.0);
        assert_eq!(config.max_tokens, 60.0);
        assert_eq!(config.refill_rate, 1.0);
    }

    #[test]
    fn test_rate_limit_config_per_second() {
        let config = RateLimitConfig::per_second(10.0);
        assert_eq!(config.max_tokens, 10.0);
        assert_eq!(config.refill_rate, 10.0);
    }

    #[test]
    fn test_rate_limit_config_per_hour() {
        let config = RateLimitConfig::per_hour(3600.0);
        assert_eq!(config.max_tokens, 3600.0);
        assert_eq!(config.refill_rate, 1.0);
    }

    #[tokio::test]
    async fn test_traffic_shield_db_latency() {
        let config = TrafficShieldConfig::new().with_db_probe(false);
        let shield = TrafficShield::new(config);

        assert_eq!(shield.db_latency().as_millis(), 0);
        shield
            .db_latency_ms
            .store(50, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(shield.db_latency().as_millis(), 50);
    }

    #[tokio::test]
    async fn test_traffic_shield_event_loop_lag() {
        let config = TrafficShieldConfig::new().with_db_probe(false);
        let shield = TrafficShield::new(config);

        assert_eq!(shield.event_loop_lag().as_millis(), 0);
        shield
            .event_loop_lag_ms
            .store(100, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(shield.event_loop_lag().as_millis(), 100);
    }

    #[test]
    fn test_check_and_consume() {
        let config = RateLimitConfig::per_second(2.0); // 2 tokens per second
        let limiter = RateLimiter::new(config);

        // Consume 1st token
        assert!(limiter.check_and_consume("test_key"));
        // Consume 2nd token
        assert!(limiter.check_and_consume("test_key"));
        // 3rd token should fail (out of tokens)
        assert!(!limiter.check_and_consume("test_key"));
    }
    #[tokio::test]
    async fn test_traffic_shield_config_builders() {
        let config = TrafficShieldConfig::new()
            .with_max_event_loop_lag(Duration::from_millis(999))
            .with_max_db_latency(Duration::from_millis(888))
            .with_max_active_requests(777);

        assert_eq!(config.max_event_loop_lag, Duration::from_millis(999));
        assert_eq!(config.max_db_latency, Duration::from_millis(888));
        assert_eq!(config.max_active_requests, 777);
    }

    #[tokio::test]
    async fn test_check_and_consume_refill() {
        let config = RateLimitConfig::per_second(2.0); // max 2, refill 2/s
        let limiter = RateLimiter::new(config);

        assert!(limiter.check_and_consume("test_key"));
        assert!(limiter.check_and_consume("test_key"));
        assert!(!limiter.check_and_consume("test_key")); // 0 tokens left

        // wait for refill
        tokio::time::sleep(Duration::from_millis(600)).await; // 0.6s * 2 = 1.2 tokens
        assert!(limiter.check_and_consume("test_key"));
        assert!(!limiter.check_and_consume("test_key")); // Should only be 1 token restored
    }

    #[tokio::test]
    async fn test_backpressure_middleware_critical() {
        use axum::{Router, routing::get};
        use tower::ServiceExt;

        let config = TrafficShieldConfig::new().with_max_active_requests(0); // instant shed
        let shield = TrafficShield::new(config);

        let app =
            Router::new()
                .route("/", get(|| async { "OK" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let s = shield.clone();
                    async move { backpressure_middleware(s, req, next).await }
                }));

        let req = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_backpressure_middleware_moderate() {
        use axum::{Router, routing::get};
        use std::time::Instant;
        use tower::ServiceExt;

        let config = TrafficShieldConfig::new().with_max_active_requests(100);
        let shield = TrafficShield::new(config);
        shield
            .active_requests
            .store(50, std::sync::atomic::Ordering::SeqCst); // moderate (>= max/2)

        let app =
            Router::new()
                .route("/", get(|| async { "OK" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let s = shield.clone();
                    async move { backpressure_middleware(s, req, next).await }
                }));

        let req = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let start = Instant::now();
        let res = app.oneshot(req).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(res.status(), axum::http::StatusCode::OK);
        assert!(elapsed >= Duration::from_millis(25)); // moderate load causes 25ms sleep
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_rejection() {
        use axum::{Router, routing::get};
        use tower::ServiceExt;

        let config = RateLimitConfig::per_second(1.0); // 1 request per second capacity
        let limiter = RateLimiter::new(config);

        let app =
            Router::new()
                .route("/", get(|| async { "OK" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let l = limiter.clone();
                    async move { rate_limit_middleware(l, req, next).await }
                }));

        // 1st request should succeed
        let req1 = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(res1.status(), axum::http::StatusCode::OK);

        // 2nd request should be rejected by rate_limit_middleware
        let req2 = axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res2 = app.oneshot(req2).await.unwrap();
        assert_eq!(res2.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);

        let body_bytes = axum::body::to_bytes(res2.into_body(), 1024).await.unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body_bytes),
            "Rate limit exceeded. Please try again later."
        );
    }
}

#[cfg(kani)]
mod kani_proofs {
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

        let new_tokens = current_tokens + elapsed_secs * refill_rate;
        let final_tokens = new_tokens.min(max_tokens);

        // Prove that the math never yields NaN or Infinity under normal constraints
        assert!(!final_tokens.is_nan());
        assert!(!final_tokens.is_infinite());
        assert!(final_tokens >= 0.0);
        assert!(final_tokens <= max_tokens);
    }

    #[kani::proof]
    fn verify_traffic_shield_thresholds() {
        let max_event_loop_lag: u64 = kani::any();
        let max_db_latency: u64 = kani::any();
        let max_active_requests: usize = kani::any();

        let lag: u64 = kani::any();
        let db_lat: u64 = kani::any();
        let active: usize = kani::any();

        // Proves that divisions by 2 in backpressure middleware never panic
        let moderate_cpu_thresh = max_event_loop_lag / 2;
        let moderate_db_thresh = max_db_latency / 2;
        let moderate_active_thresh = max_active_requests / 2;

        let is_moderate_cpu = lag >= moderate_cpu_thresh;
        let is_moderate_db = db_lat >= moderate_db_thresh;
        let is_moderate_active = active >= moderate_active_thresh;

        // Basic sanity assertions
        if active == 0 {
            assert!(!is_moderate_active);
        }
    }
}
