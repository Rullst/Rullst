//! Token-bucket rate limiting and its Axum middleware.

use super::{RateLimitConfig, RateLimiter, TokenBucket, buckets};
use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

pub(super) fn refill_token_count(
    current_tokens: f64,
    elapsed_secs: f64,
    config: &RateLimitConfig,
) -> f64 {
    (current_tokens + elapsed_secs * config.refill_rate).min(config.max_tokens)
}

impl RateLimiter {
    /// Creates a new `RateLimiter` from the given config, using the transport
    /// peer address as the default key (IPv4 per address, IPv6 per /64).
    ///
    /// Forwarded headers are untrusted and deliberately ignored. Deployments
    /// behind a trusted proxy can install an explicit key extractor after
    /// validating the proxy chain.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(DashMap::new()),
            key_extractor: Arc::new(default_key_extractor),
            new_keys: Arc::new(AtomicUsize::new(0)),
            max_buckets: buckets::MAX_RATE_LIMIT_BUCKETS,
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
    ///
    /// A key longer than 128 bytes is stored as its SHA-256 digest, so a
    /// custom extractor returning a long header value (an `Authorization`
    /// token, say) cannot make each of the at most 100,000 buckets hold an
    /// attacker-sized key.
    pub fn check_and_consume(&self, key: &str) -> bool {
        let now = Instant::now();
        let key = buckets::bounded_bucket_key(key);
        if !self.buckets.contains_key(key.as_ref()) {
            self.make_room_for_new_key(now);
        }
        let mut entry = self
            .buckets
            .entry(key.into_owned())
            .or_insert_with(|| TokenBucket {
                tokens: self.config.max_tokens,
                last_refill: now,
            });

        let elapsed = now.duration_since(entry.last_refill).as_secs_f64();
        entry.tokens = refill_token_count(entry.tokens, elapsed, &self.config);
        entry.last_refill = now;

        if entry.tokens >= 1.0 {
            entry.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Default key extractor based only on Axum's transport peer address.
///
/// IPv4 peers are keyed per address (`192.0.2.7`) and IPv6 peers per /64
/// prefix (`2001:db8:1:2::/64`); IPv4-mapped IPv6 peers are keyed as IPv4.
pub fn default_key_extractor(req: &Request) -> String {
    if let Some(conn_info) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return buckets::peer_rate_limit_key(conn_info.0.ip());
    }

    "missing-peer-address".to_string()
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
