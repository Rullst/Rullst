//! Kubernetes and Cloud-Native Health Probes (Liveness & Readiness)
#![cfg(not(target_arch = "wasm32"))]

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static START_TIME: AtomicU64 = AtomicU64::new(0);

/// Initializes the application boot time tracking for health probe metrics.
pub fn init_health_boot_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    START_TIME.store(now, Ordering::Relaxed);
}

/// Structured response payload for health probes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthProbeResponse {
    /// Service health status ("UP", "DOWN", or "DEGRADED").
    pub status: &'static str,
    /// Framework / App version string.
    pub version: &'static str,
    /// Application uptime in seconds.
    pub uptime_seconds: u64,
    /// System Unix timestamp in seconds.
    pub timestamp: u64,
}

/// Endpoint handler for Kubernetes Liveness probe (`GET /health`).
pub async fn liveness_handler() -> impl IntoResponse {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let boot_time = START_TIME.load(Ordering::Relaxed);
    let uptime = if boot_time > 0 && now >= boot_time {
        now - boot_time
    } else {
        0
    };

    let payload = HealthProbeResponse {
        status: "UP",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        timestamp: now,
    };

    (StatusCode::OK, Json(payload))
}

/// Endpoint handler for Kubernetes Readiness probe (`GET /ready`).
pub async fn readiness_handler() -> impl IntoResponse {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let boot_time = START_TIME.load(Ordering::Relaxed);
    let uptime = if boot_time > 0 && now >= boot_time {
        now - boot_time
    } else {
        0
    };

    let payload = HealthProbeResponse {
        status: "UP",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        timestamp: now,
    };

    (StatusCode::OK, Json(payload))
}

/// Returns an Axum `Router` mounting `/health` and `/ready` health probe routes.
pub fn health_router() -> Router {
    Router::new()
        .route("/health", get(liveness_handler))
        .route("/ready", get(readiness_handler))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_probes_endpoints() {
        init_health_boot_time();
        let app = health_router();

        let liveness_req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(liveness_req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let readiness_req = Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .unwrap();
        let ready_resp = app.oneshot(readiness_req).await.unwrap();
        assert_eq!(ready_resp.status(), StatusCode::OK);
    }
}
