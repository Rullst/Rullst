//! Unit tests for the Traffic Shield and the rate limiter middleware.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::http::Request;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[test]
fn traffic_shield_construction_is_runtime_independent() {
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(false));

    assert!(!shield.is_running());
    assert_eq!(shield.start(), Err(TrafficShieldError::RuntimeUnavailable));
    assert!(!shield.is_running());
}

#[tokio::test]
async fn traffic_shield_has_explicit_shared_shutdown() {
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(false));
    let clone = shield.clone();

    shield.start().unwrap();
    assert!(clone.is_running());
    clone.shutdown();
    assert!(!shield.is_running());
    assert_eq!(shield.start(), Err(TrafficShieldError::AlreadyShutDown));
}

#[tokio::test]
async fn dropping_final_shield_aborts_monitor_tasks() {
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(false));
    let monitored_value = Arc::downgrade(&shield.event_loop_lag_ms);
    shield.start().unwrap();
    drop(shield);

    tokio::time::timeout(Duration::from_millis(100), async {
        while monitored_value.upgrade().is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn load_shedding_reports_once_per_interval() {
    use tower::ServiceExt;

    // Zero admitted requests keeps the shield permanently critical.
    let shield = TrafficShield::new(
        TrafficShieldConfig::new()
            .with_db_probe(false)
            .with_max_active_requests(0),
    );
    let layer_shield = shield.clone();
    let router = axum::Router::new()
        .route("/work", axum::routing::get(|| async { "work" }))
        .layer(axum::middleware::from_fn(move |request, next| {
            backpressure_middleware(layer_shield.clone(), request, next)
        }));
    for _ in 0..3 {
        let response = router
            .clone()
            .oneshot(
                Request::get("/work")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    // One report; the other two rejections in the interval are counted.
    assert_eq!(shield.monitors.shed_log.suppressed(), 2);
    shield.shutdown();
}

#[test]
fn test_default_key_extractor() {
    let req1 = Request::builder()
        .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(default_key_extractor(&req1), "missing-peer-address");

    let req2 = Request::builder()
        .header("x-real-ip", "10.0.0.2")
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(default_key_extractor(&req2), "missing-peer-address");

    let mut req3 = Request::builder().body(axum::body::Body::empty()).unwrap();
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    req3.extensions_mut()
        .insert(axum::extract::ConnectInfo(socket));
    assert_eq!(default_key_extractor(&req3), "127.0.0.1");

    let req4 = Request::builder().body(axum::body::Body::empty()).unwrap();
    assert_eq!(default_key_extractor(&req4), "missing-peer-address");
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
fn test_traffic_pressure_classification() {
    let config = TrafficShieldConfig::new().with_db_probe(false);

    assert_eq!(
        classify_traffic_pressure(&config, Duration::ZERO, Duration::ZERO, 0),
        TrafficPressure::Normal
    );
    assert_eq!(
        classify_traffic_pressure(&config, config.max_event_loop_lag / 2, Duration::ZERO, 0,),
        TrafficPressure::Moderate
    );
    assert_eq!(
        classify_traffic_pressure(&config, config.max_event_loop_lag, Duration::ZERO, 0,),
        TrafficPressure::Critical
    );
    assert_eq!(
        classify_traffic_pressure(&config, Duration::ZERO, config.max_db_latency, 0,),
        TrafficPressure::Normal
    );
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

    let app = Router::new()
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

    let app = Router::new()
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

    let app = Router::new()
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
