//! Server-level rate limiting and Traffic Shield composition.

use crate::lifecycle::is_health_probe;
use crate::resilience::{RateLimiter, TrafficShield};
use axum::extract::Request;
use axum::middleware::Next;

/// Layers the optional rate limiter and Traffic Shield around `router`.
///
/// Exact `GET`/`HEAD /health` and `/ready` probes bypass both, like lifecycle
/// admission: a saturated database or an exhausted bucket must not make the
/// process-only liveness probe fail and trigger orchestrator restarts. The
/// limiter stays inside the shield, preserving the previous layer order.
pub(crate) fn apply_traffic_controls(
    mut router: axum::Router,
    limiter: Option<RateLimiter>,
    shield: Option<TrafficShield>,
) -> axum::Router {
    if let Some(limiter) = limiter {
        router = router.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let limiter = limiter.clone();
                async move {
                    if is_health_probe(&request) {
                        next.run(request).await
                    } else {
                        crate::resilience::rate_limit_middleware(limiter, request, next).await
                    }
                }
            },
        ));
    }
    if let Some(shield) = shield {
        router = router.layer(axum::middleware::from_fn(
            move |request: Request, next: Next| {
                let shield = shield.clone();
                async move {
                    if is_health_probe(&request) {
                        next.run(request).await
                    } else {
                        crate::resilience::backpressure_middleware(shield, request, next).await
                    }
                }
            },
        ));
    }
    router
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::resilience::{RateLimitConfig, TrafficShieldConfig};
    use axum::http::{Method, StatusCode};
    use tower::ServiceExt;

    fn application() -> axum::Router {
        axum::Router::new()
            .route("/health", axum::routing::get(|| async { "live" }))
            .route("/ready", axum::routing::get(|| async { "ready" }))
            .route("/work", axum::routing::get(|| async { "work" }))
    }

    async fn status(router: &axum::Router, method: Method, uri: &str) -> StatusCode {
        router
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn a_critical_shield_sheds_application_requests_but_not_health_probes() {
        // Zero admitted requests keeps the shield permanently critical.
        let shield = TrafficShield::new(
            TrafficShieldConfig::new()
                .with_db_probe(false)
                .with_max_active_requests(0),
        );
        let router = apply_traffic_controls(application(), None, Some(shield.clone()));

        assert_eq!(
            status(&router, Method::GET, "/work").await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        for _ in 0..3 {
            assert_eq!(
                status(&router, Method::GET, "/health").await,
                StatusCode::OK
            );
            assert_eq!(
                status(&router, Method::HEAD, "/health").await,
                StatusCode::OK
            );
            assert_eq!(status(&router, Method::GET, "/ready").await, StatusCode::OK);
        }
        // Only exact GET/HEAD probe paths are exempt.
        assert_eq!(
            status(&router, Method::GET, "/health/details").await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            status(&router, Method::POST, "/health").await,
            StatusCode::SERVICE_UNAVAILABLE
        );
        shield.shutdown();
    }

    #[tokio::test]
    async fn an_exhausted_rate_limit_bucket_still_admits_health_probes() {
        let limiter = RateLimiter::new(RateLimitConfig::per_hour(1.0));
        let router = apply_traffic_controls(application(), Some(limiter), None);

        assert_eq!(status(&router, Method::GET, "/work").await, StatusCode::OK);
        assert_eq!(
            status(&router, Method::GET, "/work").await,
            StatusCode::TOO_MANY_REQUESTS
        );
        for _ in 0..3 {
            assert_eq!(
                status(&router, Method::GET, "/health").await,
                StatusCode::OK
            );
            assert_eq!(status(&router, Method::GET, "/ready").await, StatusCode::OK);
        }
    }
}
