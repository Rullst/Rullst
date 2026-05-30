use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Configures the limits and behavior of the Adaptive Backpressure & Resilient Traffic Shielding.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct TrafficShieldConfig {
    pub max_event_loop_lag: Duration,
    pub max_db_latency: Duration,
    pub max_active_requests: usize,
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
                    let pool_opt = std::panic::catch_unwind(rullst_orm::Orm::pool);
                    if let Ok(pool) = pool_opt {
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

    pub fn event_loop_lag(&self) -> Duration {
        Duration::from_millis(self.event_loop_lag_ms.load(Ordering::Relaxed))
    }

    pub fn db_latency(&self) -> Duration {
        Duration::from_millis(self.db_latency_ms.load(Ordering::Relaxed))
    }

    pub fn active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }
}

/// Router-level protection middleware that tracks load timing and drops requests under critical saturation.
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

        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(axum::http::header::RETRY_AFTER, "5")
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(axum::body::Body::from(
                "Service Temporarily Saturated. Please try again soon.",
            ))
            .unwrap();
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
    pub max_tokens: f64,
    pub refill_rate: f64,
}

impl RateLimitConfig {
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
        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(axum::body::Body::from(
                "Rate limit exceeded. Please try again later.",
            ))
            .unwrap()
    }
}
