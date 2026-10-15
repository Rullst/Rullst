//! Kubernetes and Cloud-Native Health Probes (Liveness & Readiness)
#![cfg(not(target_arch = "wasm32"))]

use crate::lifecycle::{ApplicationLifecycle, ReadinessSnapshot};
use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static START_TIME: AtomicU64 = AtomicU64::new(0);

/// Initializes the application boot time tracking for health probe metrics.
///
/// `Server` records it when it starts unless this was called earlier.
pub fn init_health_boot_time() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    START_TIME.store(now, Ordering::Relaxed);
}

/// Records the boot time once: an explicit earlier [`init_health_boot_time`]
/// wins.
pub(crate) fn init_health_boot_time_if_unset() {
    crate::radar::record_boot_time_once(&START_TIME);
}

#[cfg(test)]
pub(crate) fn recorded_boot_time() -> u64 {
    START_TIME.load(Ordering::Relaxed)
}

/// Structured response payload for health probes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthProbeResponse {
    /// Service health status ("UP", "DOWN", or "DEGRADED").
    pub status: &'static str,
    /// Version of the Rullst framework (`rullst-core`), not of the
    /// application. [`health_router_with_version`] also reports the
    /// application's own version, as `app_version`.
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
    /// Version of the Rullst framework (`rullst-core`), not of the
    /// application. [`health_router_with_lifecycle_and_version`] also reports
    /// the application's own version, as `app_version`.
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

/// The readiness status and payload of `lifecycle` now.
fn lifecycle_readiness(lifecycle: &ApplicationLifecycle) -> (StatusCode, ReadinessProbeResponse) {
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
    (status, payload)
}

async fn lifecycle_readiness_handler(
    State(lifecycle): State<ApplicationLifecycle>,
) -> impl IntoResponse {
    let (status, payload) = lifecycle_readiness(&lifecycle);
    (status, Json(payload))
}

/// The application's own version, reported as `app_version`.
#[derive(Clone)]
struct AppVersion(Arc<str>);

/// Router state of [`health_router_with_lifecycle_and_version`].
#[derive(Clone)]
struct VersionedLifecycle {
    lifecycle: ApplicationLifecycle,
    app_version: AppVersion,
}

impl FromRef<VersionedLifecycle> for AppVersion {
    fn from_ref(state: &VersionedLifecycle) -> Self {
        state.app_version.clone()
    }
}

/// A probe payload followed by the application's version.
#[derive(Serialize)]
struct WithAppVersion<'a, T> {
    #[serde(flatten)]
    probe: T,
    app_version: &'a str,
}

/// Process-only `/health` and `/ready` that also report `app_version`.
async fn versioned_process_probe(State(app_version): State<AppVersion>) -> Response {
    let (now, uptime) = time_snapshot();
    let probe = HealthProbeResponse {
        status: "UP",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        timestamp: now,
    };
    Json(WithAppVersion {
        probe,
        app_version: &app_version.0,
    })
    .into_response()
}

async fn versioned_lifecycle_readiness(State(state): State<VersionedLifecycle>) -> Response {
    let (status, probe) = lifecycle_readiness(&state.lifecycle);
    let payload = WithAppVersion {
        probe,
        app_version: &state.app_version.0,
    };
    (status, Json(payload)).into_response()
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

/// Returns [`health_router`]'s `/health` and `/ready`, whose JSON also
/// carries the application's own version as `app_version`; `version` stays
/// the framework version. Added in v13.
///
/// ```no_run
/// let probes = rullst_core::health::health_router_with_version(env!("CARGO_PKG_VERSION"));
/// ```
pub fn health_router_with_version(app_version: impl Into<String>) -> Router {
    let app_version = AppVersion(Arc::from(app_version.into()));
    Router::new()
        .route("/health", get(versioned_process_probe))
        .route("/ready", get(versioned_process_probe))
        .with_state(app_version)
}

/// Returns [`health_router_with_lifecycle`]'s `/health` and `/ready`, whose
/// JSON also carries the application's own version as `app_version`;
/// `version` stays the framework version. Added in v13.
pub fn health_router_with_lifecycle_and_version(
    lifecycle: ApplicationLifecycle,
    app_version: impl Into<String>,
) -> Router {
    let app_version = AppVersion(Arc::from(app_version.into()));
    Router::new()
        .route("/health", get(versioned_process_probe))
        .route("/ready", get(versioned_lifecycle_readiness))
        .with_state(VersionedLifecycle {
            lifecycle,
            app_version,
        })
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

        crate::lifecycle::tests::bounded_begin_draining(&lifecycle).unwrap();
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

    /// Status and JSON body of one `GET` on `app`.
    async fn probe(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn probes_report_the_application_version_next_to_the_framework_version() {
        let app = health_router_with_version(String::from("2.4.1+abc123"));
        for path in ["/health", "/ready"] {
            let (status, body) = probe(&app, path).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            assert_eq!(body["status"], "UP", "{path}");
            assert_eq!(body["app_version"], "2.4.1+abc123", "{path}");
            assert_eq!(body["version"], env!("CARGO_PKG_VERSION"), "{path}");
            assert!(body["uptime_seconds"].is_u64(), "{path}");
            assert!(
                body["timestamp"].as_u64().is_some_and(|now| now > 0),
                "{path}"
            );
        }
        // The original routers keep reporting only the framework version.
        let (_, body) = probe(&health_router(), "/health").await;
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body.get("app_version").is_none());
    }

    #[tokio::test]
    async fn lifecycle_probes_report_the_application_version() {
        let lifecycle = ApplicationLifecycle::new();
        let app = health_router_with_lifecycle_and_version(lifecycle.clone(), "7.0.0");

        let (status, body) = probe(&app, "/ready").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "DOWN");
        assert_eq!(body["phase"], "starting");
        assert_eq!(body["app_version"], "7.0.0");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));

        lifecycle.mark_ready().unwrap();
        let (status, body) = probe(&app, "/ready").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "UP");
        assert_eq!(body["app_version"], "7.0.0");

        let (status, body) = probe(&app, "/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "UP");
        assert_eq!(body["app_version"], "7.0.0");
    }
}
