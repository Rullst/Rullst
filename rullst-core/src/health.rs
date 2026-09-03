//! Kubernetes and Cloud-Native Health Probes (Liveness & Readiness)
#![cfg(not(target_arch = "wasm32"))]

use crate::lifecycle::{ApplicationLifecycle, ReadinessSnapshot};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
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

/// Secret-minimized response payload for lifecycle-aware readiness probes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReadinessProbeResponse {
    /// Readiness status (`UP` only while every bounded gate is ready).
    pub status: &'static str,
    /// Framework / App version string.
    pub version: &'static str,
    /// Application uptime in seconds.
    pub uptime_seconds: u64,
    /// System Unix timestamp in seconds.
    pub timestamp: u64,
    /// Process phase and aggregate readiness counters. Component labels and
    /// dependency error messages are intentionally not returned.
    #[serde(flatten)]
    pub lifecycle: ReadinessSnapshot,
}

fn time_snapshot() -> (u64, u64) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let boot_time = START_TIME.load(Ordering::Relaxed);
    let uptime = if boot_time > 0 && now >= boot_time {
        now - boot_time
    } else {
        0
    };
    (now, uptime)
}

/// Endpoint handler for Kubernetes Liveness probe (`GET /health`).
pub async fn liveness_handler() -> impl IntoResponse {
    let (now, uptime) = time_snapshot();

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
    let (now, uptime) = time_snapshot();

    let payload = HealthProbeResponse {
        status: "UP",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        timestamp: now,
    };

    (StatusCode::OK, Json(payload))
}

async fn lifecycle_readiness_handler(
    State(lifecycle): State<ApplicationLifecycle>,
) -> impl IntoResponse {
    let (now, uptime) = time_snapshot();
    let snapshot = lifecycle.snapshot();
    let status = if snapshot.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let payload = ReadinessProbeResponse {
        status: if snapshot.ready { "UP" } else { "DOWN" },
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        timestamp: now,
        lifecycle: snapshot,
    };
    (status, Json(payload))
}

/// Returns an Axum `Router` mounting `/health` and `/ready` health probe routes.
pub fn health_router() -> Router {
    Router::new()
        .route("/health", get(liveness_handler))
        .route("/ready", get(readiness_handler))
}

/// Returns `/health` and lifecycle-aware `/ready` routes.
///
/// Liveness stays process-only. Readiness fails with `503` during startup,
/// dependency unavailability, lock corruption, draining, and after stop. The
/// payload exposes aggregate counts, never component labels or error details.
pub fn health_router_with_lifecycle(lifecycle: ApplicationLifecycle) -> Router {
    Router::new()
        .route("/health", get(liveness_handler))
        .route("/ready", get(lifecycle_readiness_handler))
        .with_state(lifecycle)
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

    #[tokio::test]
    async fn lifecycle_readiness_is_bounded_and_fails_closed() {
        // TM-CORE-01: startup/dependency/drain state must fail closed without
        // publishing component labels or dependency errors.
        use axum::body::to_bytes;

        let lifecycle =
            ApplicationLifecycle::with_required_components(["private-database"]).unwrap();
        let app = health_router_with_lifecycle(lifecycle.clone());

        let starting = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(starting.status(), StatusCode::SERVICE_UNAVAILABLE);
        let bytes = to_bytes(starting.into_body(), 4096).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("\"phase\":\"starting\""));
        assert!(body.contains("\"required_components\":1"));
        assert!(!body.contains("private-database"));

        lifecycle.mark_ready().unwrap();
        lifecycle
            .set_component_ready("private-database", true)
            .unwrap();
        let ready = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ready.status(), StatusCode::OK);

        lifecycle.begin_draining().unwrap();
        let draining = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(draining.status(), StatusCode::SERVICE_UNAVAILABLE);

        let liveness = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(liveness.status(), StatusCode::OK);
    }
}
