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

Cache diagnostics are driver-specific too. `Cache::inspect(limit)` accepts
1–200 and returns sorted logical-key, UTF-8 value-length and remaining-TTL
metadata for Memory and Redis without the value. Exact keys are still
application data and `CacheEntryMetadata::logical_key()` belongs only inside an
authorized diagnostic boundary; its `Debug` output redacts the key. Custom
drivers return `CacheError::InspectionUnsupported` unless they implement the
bounded method. The live Redis CI/release contract checks metadata, TTL and
non-disclosure; it does not prove cluster/failover or operator authorization.
The Redis cache and queue drivers each keep one lazily opened multiplexed
connection for all operations and replace it after a connection-level failure.

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
`Worker` drives each `pop` to completion instead of racing it against
completions or shutdown, because both built-in claims commit before the future
resolves. A job claimed after graceful shutdown was requested is requeued.
Custom drivers return `QueueError::Unsupported` for future timestamps unless
they explicitly implement durable scheduling.

`ApplicationLifecycle` supplies an opt-in process-local startup/readiness/drain
contract. Up to 32 immutable validated component labels can gate readiness and
application admission; `/ready` publishes only aggregate counts, not labels or
dependency errors. `Server::with_lifecycle` marks the phase ready after binding,
begins draining before Axum's graceful wait, and marks it stopped on completion
or startup failure. `run_with_shutdown` accepts a caller-owned trigger for
embedded supervisors and deterministic tests. The lifecycle does not run
dependency probes, coordinate replicas, authorize users, or guarantee load
balancer propagation.

The v13 drain candidate retains admission through the response body, including
streaming/trailers, error and cancellation. It does not track upgraded connections
or prove client receipt. The [two-replica deployment contract](../deployment-acceptance.md)
exercises this boundary through a real proxy; full hosted admission remains pending.

## ✨ Core Features & Subsystems

- **Axum-compatible routing:** `rullst::Router` wraps and converts to/from
  `axum::Router`; application latency depends on handlers, middleware, build
  profile, and deployment.
- **Typed server functions:** concrete async `#[server_function]` items share
  owned Serde arguments/results through the versioned `rullst.client` v1
  envelope. The generated native router and Wasm caller enforce same-origin
  paths, bounded bodies, correlation and redacted errors; hosts still own
  identity, tenant, authorization, idempotency and rate-limit policy.
- **Rullst Radar (`rullst::radar`):** Collects process RSS/CPU where an OS probe
  is supported, Tokio task/yield observations when a runtime is available, and
  process uptime. Unsupported probes return `None`. On Linux, RSS comes from
  `VmRSS` in `/proc/self/status` (correct on 16/64 KiB page kernels), and CPU
  percent is process CPU time over wall time: the host-wide `/proc/stat` delta
  is scaled by its host CPU count, not by the cgroup-limited
  `available_parallelism`, so a container saturating a 2-CPU quota reports
  about 200%.
- **Prometheus `/metrics` Exporter:** Text-format metrics served at `GET /metrics`; formatting and collection have bounded runtime cost.
- **Kubernetes probe routes (`rullst::health`):** the simple `health_router`
  reports process availability and uptime. The opt-in
  `health_router_with_lifecycle` returns readiness from the same bounded state
  that gates Server request admission; the application still performs and
  times out its own dependency checks.
- **Interactive Scalar API Docs (`rullst::scalar`):** OpenAPI documentation UI
  mounted at `/docs`, with a pinned CDN asset and a status-only fallback. A
  missing or malformed `openapi.json` returns `503`.
- **Typed framework errors:** startup, queues, validation, scheduling, storage,
  and other subsystems expose their own typed errors. Applications may compose
  those into an application-owned `AppError`; Core does not define one global
  application error type.
- **Durable scheduled queues:** SQLite and Redis persist bounded due timestamps;
  the live Redis CI contract proves that an immediate job remains claimable
  while a future job stays unavailable.
- **Opt-in completed-job monitoring:** SQLite can retain and atomically prune a
  configured number of successful jobs; the privacy-safe default remains
  immediate deletion.
- **Bounded token-bucket rate limiter:** `RateLimiter` keys IPv4 peers per
  address and IPv6 peers per /64 by default. It tracks at most 100,000 keys,
  drops fully refilled buckets and evicts the least recently used ones beyond
  that cap; state is process-local, not a distributed limit.
- **Feature flag buckets:** percentage rollouts and A/B variants in the Env,
  TOML, Memory and DB drivers use `calculate_hash_bucket`, a versioned
  SHA-256 hash over a domain tag, the length-prefixed flag and the identifier.
  It gives every toolchain, platform and replica the same assignment.
  Earlier releases used `std`'s unspecified `DefaultHasher`, so upgrading
  reassigns users to buckets once; percentages and variant weights are kept.
  `TomlFeatureDriver::reload` parses into a new map and swaps it in at once,
  so concurrent evaluations never see a flag as unset mid-reload, and a
  `[features] # comment` header is recognized.
- **Bounded cache metadata:** Memory and Redis expose value length and TTL for
  at most 200 sorted entries, never cached values. Rullst Studio renders keyed
  opaque identifiers and one-entry invalidation rather than exact keys or bulk
  flush.

---

## 🚀 Usage

Most applications can use the re-exports provided by the umbrella `rullst`
crate instead of depending on `rullst-core` directly.

### Mounting lifecycle-aware Health Probes & Prometheus Metrics

These optional surfaces are application-owned and must be mounted explicitly.
Protect or isolate `/metrics` when its operational data should not be public.

```rust,no_run
use rullst_core::{
    ApplicationLifecycle, Router, Server,
    health::health_router_with_lifecycle,
    radar::radar_metrics_router,
    scalar::scalar_docs_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lifecycle = ApplicationLifecycle::new();
    let app = Router::new()
        .merge_axum(health_router_with_lifecycle(lifecycle.clone()))
        .merge_axum(radar_metrics_router())   // GET /metrics (Prometheus)
        .merge_axum(scalar_docs_router("/openapi.json")); // GET /docs
    Server::new(app)
        .with_lifecycle(lifecycle)
        .run(3000)
        .await?;
    Ok(())
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
