#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use axum::{Router, routing::get};
use tower::ServiceExt;

fn request() -> Request {
    Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .expect("valid request")
}

#[tokio::test]
async fn stopped_shield_middleware_fails_closed_without_counting_a_request() {
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(false));
    shield.shutdown();
    let observed = shield.clone();
    let app = Router::new()
        .route("/", get(|| async { "unreachable" }))
        .layer(axum::middleware::from_fn(move |request, next| {
            let shield = shield.clone();
            async move { backpressure_middleware(shield, request, next).await }
        }));

    let response = app.oneshot(request()).await.expect("infallible router");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(observed.active_requests(), 0);
}

#[tokio::test]
async fn custom_rate_limit_key_and_critical_telemetry_paths_are_enforced() {
    let limiter = RateLimiter::new(RateLimitConfig::per_hour(1.0))
        .with_key_extractor(|_| "authenticated-user".to_string());
    let observed = limiter.clone();
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(move |request, next| {
            let limiter = limiter.clone();
            async move { rate_limit_middleware(limiter, request, next).await }
        }));
    assert_eq!(
        app.clone()
            .oneshot(request())
            .await
            .expect("first request")
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.oneshot(request())
            .await
            .expect("second request")
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(observed.buckets.contains_key("authenticated-user"));

    for db_is_critical in [false, true] {
        let shield = TrafficShield::new(
            TrafficShieldConfig::new()
                .with_db_probe(db_is_critical)
                .with_max_event_loop_lag(Duration::from_millis(10))
                .with_max_db_latency(Duration::from_millis(10)),
        );
        if db_is_critical {
            shield.db_latency_ms.store(10, Ordering::Relaxed);
        } else {
            shield.event_loop_lag_ms.store(10, Ordering::Relaxed);
        }
        let app = Router::new()
            .route("/", get(|| async { "unreachable" }))
            .layer(axum::middleware::from_fn(move |request, next| {
                let shield = shield.clone();
                async move { backpressure_middleware(shield, request, next).await }
            }));
        assert_eq!(
            app.oneshot(request())
                .await
                .expect("infallible router")
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}

#[tokio::test]
async fn poisoned_monitor_lock_recovers_and_optional_db_probe_stops_cleanly() {
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(true));
    let monitors = Arc::clone(&shield.monitors);
    let poisoner = std::thread::spawn(move || {
        let _guard = monitors.tasks.lock().expect("initial lock");
        panic!("poison monitor fixture");
    });
    assert!(poisoner.join().is_err());

    shield.start().expect("poisoned task list is recovered");
    assert!(shield.is_running());
    tokio::time::sleep(Duration::from_millis(1_050)).await;
    assert_eq!(shield.db_latency(), Duration::ZERO);
    shield.shutdown();
    assert_eq!(shield.start(), Err(TrafficShieldError::AlreadyShutDown));
}

#[test]
fn resilience_duration_conversion_saturates() {
    assert_eq!(duration_millis_u64(Duration::MAX), u64::MAX);
}
