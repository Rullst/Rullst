# Rullst Studio: local development control room

Rullst Studio is a developer-facing Axum dashboard. It can run as a standalone
server bound to `127.0.0.1` (port `5555` by default) or its router can be mounted
explicitly by an application.

Studio's built-in boundary is deliberately limited to debug builds and verified
loopback peers. It is not a shared-environment authentication system; do not
expose raw subrouters publicly without application-level authentication,
authorization, TLS, and network policy.

Generated Blog, Portfolio, LMS, ERP, and SaaS applications start the standalone
server only in debug builds and link to `http://127.0.0.1:5555`. Release builds
do not start it; runtime `RULLST_ENV` or legacy `APP_ENV` values cannot override that compile-time
boundary.

## Running Studio

The CLI can launch the local server:

```bash
cargo rullst studio
```

The library entry point is also available:

```rust,no_run
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    rullst_studio::run_studio(5555_u16).await
}
```

`Studio::new().into_router(LocalStudioAccess::loopback_only())` builds the same
debug-only router for explicit composition. The serving stack must preserve
Axum `ConnectInfo<SocketAddr>` or requests fail closed. Optional OpenAPI and
queue views are enabled with `with_openapi` and `with_horizon`.
`with_cache` opts a supported cache into metadata-only local inspection, while
`with_distributed_traces` supplies the bounded store shared with a separately
mounted push-only ingestion router.

## Current views

- `/studio`: data browser and dashboard shell.
- `/studio/radar`: runtime probes and recorded spans. Process CPU sampling is
  implemented for Linux and Windows and the KPI cards refresh from `/api/radar`
  every two seconds. The first delta-based CPU sample, unsupported platforms,
  and disconnected probes display `Unavailable`; Studio does not synthesize a
  healthy value.
- `/studio/security`: counters and events emitted by the in-process security
  store. Audit-chain integrity displays `Unavailable` until a verifier is
  connected.
- `/studio/capital`: the in-process revenue view; it is not an accounting ledger.
- `/studio/traces`: local spans plus authenticated attribute-free distributed
  records and explicit slow/repeated SQL-label heuristics.
- `/studio/cache`: metadata-only view of an explicitly supplied Memory/Redis
  cache, using opaque identifiers and one-entry invalidation.
- `/studio/migrations`, `/studio/ai`, `/studio/env`, `/studio/features`, and
  `/studio/er`: development tools for their corresponding subsystems.

Some panels poll HTTP JSON endpoints and the request logger uses SSE. The current
crate does not promise a separate WebSocket telemetry transport or zero runtime
overhead.

## Tooling boundaries

- The data browser reads, searches, and paginates allowlisted SQLx identifiers.
  Inside the verified debug-loopback/same-origin boundary, it may edit one
  primitive non-key value or delete one complete-primary-key-selected row.
  Inputs are bounded and parameterized; exact deletion confirmation is
  required, backend-specific types remain read-only, and anything other than
  exactly one affected row fails. SQLite, PostgreSQL, MySQL, and MariaDB run
  separate executable contracts. This does not supply application tenant/RBAC,
  audit history, rollback, or a shared-production database administrator.
- Swagger UI appears only when the application supplies its `OpenApi` document
  with `Studio::with_openapi`; Studio does not reverse-engineer arbitrary Axum
  routes.
- The request SSE records method, URI, status and latency. It deliberately does
  not capture bodies or headers, which commonly contain credentials and PII.
- The jobs view lists the bounded snapshot exposed by a supplied queue. SQLite
  deletes successful rows by default; an application can explicitly select
  `Queue::sqlite_with_completed_history` for bounded, transactionally pruned
  completion history and can purge that history from Studio. Retained payloads
  require host-controlled access and retention policy. Other drivers expose
  only the inspection/history contract they implement.
- The ER view inspects SQLite, PostgreSQL, MySQL, or MariaDB metadata with bound
  lookup values and normalizes Mermaid identifiers. An unconfigured or
  unsupported source remains visibly unavailable.
- The feature-flags page changes the database table used by `DbFeatureDriver`.
  A successful toggle invalidates already-warm drivers in the same process;
  other processes and direct writers converge by TTL unless the host distributes
  an invalidation signal.
- The environment page redacts values by default and adds only a safe projection
  of process-global `RullstConfig`; URLs, filesystem paths, secrets, cookies and
  credentials are omitted.
- Cache inspection returns at most 100 UI rows containing an opaque keyed
  identifier, value byte length and remaining TTL. Values and exact logical
  keys are never rendered, bulk flush is absent, and individual invalidation
  requires the verified local mutation marker. Custom cache drivers remain
  unavailable until they implement Core's bounded metadata contract.

## Telemetry contract

Studio reads runtime state exposed by `RadarSnapshot`, `SpanCollector`, the
security store, queues, and configured database connections. A counter means only
that the corresponding instrumentation path emitted it; it is not proof that all
traffic passed through that control. Missing sources must remain visibly
unavailable.

Remote producers do not connect to the viewer. The application separately
mounts `TraceIngestor::router`, distributes a 32–128-byte
`TraceIngestionKey`, and uses `TraceBatchSigner` for the exact body and four
headers. Each ingestor binds one exact producer name to one key; multiple
producers use separate endpoints/keys over the same store. The route accepts
1–128 closed v1 spans under 128 KiB, verifies
HMAC-SHA256 plus a 60-second timestamp and one-time nonce, then commits to a
bounded process-local store idempotently. It contains no Studio read or admin
route. TLS/network policy, key custody and rotation, producer authorization,
clock synchronization, label redaction, durable storage and OTLP integration
remain deployment work.

## Production boundary

Studio is an optional crate. Its supported `run_studio` and `Studio::into_router`
paths reject credential-free use in release builds, but consumers should still
exclude it from production features unless they are implementing and testing a
separate authenticated administrator boundary. Built-in shared production
access remains roadmap work, not a password environment-variable promise.
