# Rullst Core ⚙️

`rullst-core` encapsulates the foundational primitives, routing engines, state management, Cloud-Native health probes, Kernel-level telemetry, and configuration layers of the Rullst Framework. It acts as the beating heart that orchestrates HTTP handlers, middleware, and backend worker systems.

Core is runtime-only by default. Enable `orm` for ORM bootstrap/artisan and
database-backed feature flags, and `queue-sqlite` for the SQLite queue driver.
The umbrella `rullst` crate enables both by default, while domain crates opt in
only when they actually use them.

## ✨ Core Features & Subsystems

- **Zero-Cost Routing:** Extends `axum` routing for sub-millisecond response times without sacrificing safety.
- **Rullst Radar (`rullst::radar`):** Kernel-level telemetry collector tracking Tokio runtime tick latency, active async tasks, CPU utilization, and RSS memory consumption.
- **Prometheus `/metrics` Exporter:** Text-format metrics served at `GET /metrics`; formatting and collection have bounded runtime cost.
- **Kubernetes Health Probes (`rullst::health`):** Cloud-Native Liveness (`GET /health`) and Readiness (`GET /ready`) probe endpoints.
- **Interactive Scalar API Docs (`rullst::scalar`):** High-performance OpenAPI documentation UI mounted at `/docs` with CDN loading and static offline fallback.
- **Unified Error Handling:** `AppError` standardizes fallible application paths and error-console integration. The repository's zero-panic policy is CI-scoped, not an absolute runtime guarantee.

---

## 🚀 Usage

Most developers will not depend on `rullst-core` directly, as it is re-exported seamlessly through the primary `rullst` crate.

### Mounting Health Probes & Prometheus Metrics

```rust
use axum::Router;
use rullst_core::{
    health::health_router,
    radar::radar_metrics_router,
    scalar::scalar_docs_router,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .merge(health_router())         // GET /health & GET /ready
        .merge(radar_metrics_router())   // GET /metrics (Prometheus)
        .merge(scalar_docs_router("/openapi.json")); // GET /docs
}
```

### Axum First-Class Escape Hatches & Tower Interoperability

Rullst does not lock developers in a "walled garden". `rullst::Router` provides seamless, bidirectional interoperability with `axum::Router` and `tower::Layer`:

```rust
use rullst::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;

async fn handler() -> &'static str { "ok" }

let mut router = Router::new()
    .route("/hello", get(handler))
    .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "not found") })
    .layer(CorsLayer::permissive());

// Direct conversion to raw axum::Router
let axum_app: axum::Router = router.into();

// Or wrap existing Axum routers seamlessly
let rullst_app: Router = axum_app.into();
```

## 🔐 Security Audit & Reliability

Repository workflows exercise Core with unit, integration, fuzz, and Miri jobs within their declared scopes. Consult the exact workflow run and commit for evidence; these tools do not prove the absence of every panic, leak, or vulnerability.
