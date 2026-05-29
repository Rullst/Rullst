use axum::response::IntoResponse;
use rullst::testing::TestApp;
use rullst::{RateLimitConfig, RateLimiter, TrafficShield, TrafficShieldConfig};
use std::time::Duration;

async fn test_handler() -> impl IntoResponse {
    "ok"
}

#[tokio::test]
async fn test_rate_limiter_consumption() {
    let limiter = RateLimiter::new(RateLimitConfig::new(2.0, 1.0)); // 2 tokens max, refills 1 token/sec
    let key = "127.0.0.1";

    // Consume first token
    assert!(limiter.check_and_consume(key));
    // Consume second token
    assert!(limiter.check_and_consume(key));
    // Third token should fail immediately (bucket empty)
    assert!(!limiter.check_and_consume(key));

    // Wait 1.1s for a token to refill
    tokio::time::sleep(Duration::from_millis(1100)).await;
    // Should be able to consume one token now
    assert!(limiter.check_and_consume(key));
    // Next one fails
    assert!(!limiter.check_and_consume(key));
}

#[tokio::test]
async fn test_rate_limiting_middleware() {
    let limiter = RateLimiter::new(RateLimitConfig::new(1.0, 1.0)); // 1 token max

    let lim = limiter.clone();
    let app = axum::Router::new()
        .route("/", axum::routing::get(test_handler))
        .layer(axum::middleware::from_fn(move |req, next| {
            rullst::resilience::rate_limit_middleware(lim.clone(), req, next)
        }));

    let test_app = TestApp::new(app);

    // First request is allowed (returns 200)
    let res = test_app.get("/").await;
    res.assert_status(200);

    // Second request is rate limited (returns 429)
    let res = test_app.get("/").await;
    res.assert_status(429);
    assert!(res.body_string().contains("Rate limit exceeded"));
}

#[tokio::test]
async fn test_traffic_shield_backpressure() {
    // Set low limits to easily trigger traffic shedding manually
    let config = TrafficShieldConfig::new()
        .with_max_active_requests(1)
        .with_db_probe(false);
    let shield = TrafficShield::new(config);

    let sh = shield.clone();
    let app = axum::Router::new()
        .route("/", axum::routing::get(test_handler))
        .layer(axum::middleware::from_fn(move |req, next| {
            rullst::resilience::backpressure_middleware(sh.clone(), req, next)
        }));

    let test_app = TestApp::new(app);

    // First request works
    let res = test_app.get("/").await;
    res.assert_status(200);

    // To simulate critical load in a test, let's configure max_active_requests = 0
    let config_saturated = TrafficShieldConfig::new()
        .with_max_active_requests(0)
        .with_db_probe(false);
    let shield_saturated = TrafficShield::new(config_saturated);

    let sh_sat = shield_saturated.clone();
    let app_saturated = axum::Router::new()
        .route("/", axum::routing::get(test_handler))
        .layer(axum::middleware::from_fn(move |req, next| {
            rullst::resilience::backpressure_middleware(sh_sat.clone(), req, next)
        }));

    let test_app_saturated = TestApp::new(app_saturated);

    // Under simulated critical active request saturation, request gets shed (returns 503)
    let res = test_app_saturated.get("/").await;
    res.assert_status(503);
    assert!(res.body_string().contains("Service Temporarily Saturated"));
}

#[tokio::test]
async fn test_resilience_module_exists() {

}
