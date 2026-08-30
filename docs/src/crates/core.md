# Rullst Core ⚙️

`rullst-core` contains Rullst's runtime primitives, Axum-compatible routing,
state management, health probes, process telemetry, queues, and configuration
helpers.

Core is runtime-only by default. Enable `orm` for ORM bootstrap/artisan and
database-backed feature flags, and `queue-sqlite` for the SQLite queue driver.
The umbrella `rullst` crate enables both by default, while domain crates opt in
only when they actually use them.

Queue monitoring capabilities are driver-specific. The trait defaults for
listing all jobs, retrying failures and purging failures return
`QueueError::Unsupported`; they never fabricate an empty snapshot or successful
mutation. `purge_failed_jobs` is the canonical facade method. The deprecated
`purge_completed_jobs` name is retained only as a source-compatibility alias for
the historical operation, which actually removed failed jobs.

SQLite deletes successful jobs by default. Applications that need a real
Studio/operations history can opt in with
`Queue::sqlite_with_completed_history(database_url, retained_jobs)`. The
validated limit is 1–100,000 records; status transition and pruning commit in
one transaction, and `purge_completed_history` removes the retained successes.
Rows still contain the original payload, so access control and retention policy
belong to the host. Redis/custom drivers do not inherit this policy implicitly.

`Queue::dispatch_at` persists a due timestamp for at most 366 days through the
built-in SQLite and Redis drivers. SQLite filters claims by local wall-clock
milliseconds; Redis atomically promotes bounded batches using Redis server time.
Neither backend claims a scheduled job early. Execution starts on the first
worker poll after it becomes due and retains the queue's at-least-once semantics.
Custom drivers return `QueueError::Unsupported` for future timestamps unless
they explicitly implement durable scheduling.

## ✨ Core Features & Subsystems

- **Axum-compatible routing:** `rullst::Router` wraps and converts to/from
  `axum::Router`; application latency depends on handlers, middleware, build
  profile, and deployment.
- **Rullst Radar (`rullst::radar`):** Collects process RSS/CPU where an OS probe
  is supported, Tokio task/yield observations when a runtime is available, and
  process uptime. Unsupported probes return `None`.
- **Prometheus `/metrics` Exporter:** Text-format metrics served at `GET /metrics`; formatting and collection have bounded runtime cost.
- **Kubernetes Health Probes (`rullst::health`):** Cloud-Native Liveness (`GET /health`) and Readiness (`GET /ready`) probe endpoints.
- **Interactive Scalar API Docs (`rullst::scalar`):** OpenAPI documentation UI
  mounted at `/docs`, with a pinned CDN asset and a status-only fallback. A
  missing or malformed `openapi.json` returns `503`.
- **Unified Error Handling:** `AppError` standardizes fallible application paths and error-console integration. The repository's zero-panic policy is CI-scoped, not an absolute runtime guarantee.
- **Durable scheduled queues:** SQLite and Redis persist bounded due timestamps;
  the live Redis CI contract proves that an immediate job remains claimable
  while a future job stays unavailable.
- **Opt-in completed-job monitoring:** SQLite can retain and atomically prune a
  configured number of successful jobs; the privacy-safe default remains
  immediate deletion.

---

## 🚀 Usage

Most applications can use the re-exports provided by the umbrella `rullst`
crate instead of depending on `rullst-core` directly.

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

`rullst::Router` provides bidirectional conversion with `axum::Router` and
accepts compatible `tower::Layer` values:

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

// Or wrap an existing Axum router
let rullst_app: Router = axum_app.into();
```

## 🔐 Security Audit & Reliability

Repository workflows exercise Core with unit, integration, fuzz, and Miri jobs within their declared scopes. Consult the exact workflow run and commit for evidence; these tools do not prove the absence of every panic, leak, or vulnerability.
