#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rullst_security::deception::global_deception_routes;
use rullst_security::log_redactor::redact_secrets;
use rullst_security::rate_limit::{RateLimitBackend, RateLimiter, rate_limit_middleware};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn test_rate_limiter_builder_and_middleware() {
    let limiter = RateLimiter::new(5, Duration::from_secs(1));

    assert_eq!(limiter.backend, RateLimitBackend::Memory);
    assert_eq!(limiter.max_requests, 5);

    assert!(limiter.clone().try_with_distributed().is_err());

    // Middleware integration
    let app = axum::Router::new()
        .route("/api/ping", axum::routing::get(|| async { "pong" }))
        .layer(axum::middleware::from_fn(rate_limit_middleware));

    for _ in 0..3 {
        let req = Request::builder()
            .uri("/api/ping")
            .header("X-Forwarded-For", "192.168.1.100")
            .body(Body::empty())
            .unwrap();
        let mut req = req;
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(192, 0, 2, 100)),
                443,
            )));
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[test]
fn test_log_redactor_and_honeypot_deception() {
    // 1. Redact sensitive patterns (Bearer tokens, API keys, passwords)
    let secret_log = "User logged in with Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9 and password=SuperSecret123";
    let redacted = redact_secrets(secret_log);
    assert!(!redacted.contains("SuperSecret123"));

    // 2. Honeypot paths
    let deception_routes = global_deception_routes();
    assert!(deception_routes.contains("/.env"));
    assert!(deception_routes.contains("/admin.php"));
    assert!(deception_routes.contains("/wp-login.php"));
    assert!(!deception_routes.contains("/api/v1/users"));
}
