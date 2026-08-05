# Rullst Core ⚙️

`rullst-core` encapsulates the foundational primitives, routing engines, state management, Cloud-Native health probes, Kernel-level telemetry, and configuration layers of the Rullst Framework. It acts as the beating heart that orchestrates HTTP handlers, middleware, and backend worker systems.

## ✨ Core Features & Subsystems

- **Zero-Cost Routing:** Extends `axum` routing for sub-millisecond response times without sacrificing safety.
- **Rullst Radar (`rullst::radar`):** Kernel-level telemetry collector tracking Tokio runtime tick latency, active async tasks, CPU utilization, and RSS memory consumption.
- **Prometheus `/metrics` Exporter:** Zero-allocation text-based metric formatter served at `GET /metrics` for native Prometheus, Grafana, and Datadog scraping.
- **Kubernetes Health Probes (`rullst::health`):** Cloud-Native Liveness (`GET /health`) and Readiness (`GET /ready`) probe endpoints.
- **Interactive Scalar API Docs (`rullst::scalar`):** High-performance OpenAPI documentation UI mounted at `/docs` with CDN loading and static offline fallback.
- **Unified Error Handling:** The `AppError` enum standardizes error propagation, guaranteeing a "Zero-Panic" runtime environment and Ignition Error Console integration.

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

## 🔐 Security Audit & Reliability

`rullst-core` is the most audited crate in the framework. It undergoes continuous fuzzing against malformed routing requests and is structurally verified against memory leaks using Miri. All functions returning `Result` strictly avoid panicking on corrupted payloads.
