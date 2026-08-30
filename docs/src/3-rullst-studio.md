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
- `/studio/traces`: recorded application spans.
- `/studio/migrations`, `/studio/ai`, `/studio/env`, `/studio/features`, and
  `/studio/er`: development tools for their corresponding subsystems.

Some panels poll HTTP JSON endpoints and the request logger uses SSE. The current
crate does not promise a separate WebSocket telemetry transport or zero runtime
overhead.

## Tooling boundaries

- The data browser reads, searches, and paginates allowlisted SQLx identifiers;
  it does not edit or delete records.
- Swagger UI appears only when the application supplies its `OpenApi` document
  with `Studio::with_openapi`; Studio does not reverse-engineer arbitrary Axum
  routes.
- The request SSE records method, URI, status and latency. It deliberately does
  not capture bodies or headers, which commonly contain credentials and PII.
- The jobs view lists the bounded snapshot exposed by a supplied queue. SQLite
  deletes successful rows, so it has no durable completion history; a custom
  driver may retain and expose completed records.
- The ER view inspects SQLite, PostgreSQL, MySQL, or MariaDB metadata with bound
  lookup values and normalizes Mermaid identifiers. An unconfigured or
  unsupported source remains visibly unavailable.
- The feature-flags page changes the database table used by `DbFeatureDriver`.
  Already cached evaluations can remain stale until that driver's local TTL
  expires.
- The environment page redacts values by default and adds only a safe projection
  of process-global `RullstConfig`; URLs, filesystem paths, secrets, cookies and
  credentials are omitted.

## Telemetry contract

Studio reads runtime state exposed by `RadarSnapshot`, `SpanCollector`, the
security store, queues, and configured database connections. A counter means only
that the corresponding instrumentation path emitted it; it is not proof that all
traffic passed through that control. Missing sources must remain visibly
unavailable.

## Production boundary

Studio is an optional crate. Its supported `run_studio` and `Studio::into_router`
paths reject credential-free use in release builds, but consumers should still
exclude it from production features unless they are implementing and testing a
separate authenticated administrator boundary. Built-in shared production
access remains roadmap work, not a password environment-variable promise.
