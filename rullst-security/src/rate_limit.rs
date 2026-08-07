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
    RATE_LIMIT_STORE.get_or_init(DashMap::new)
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
}
