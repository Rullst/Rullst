# Rullst Studio 📊

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io.

`rullst-studio` is the built-in, local-first administration and monitoring
dashboard for Rullst. It exposes bounded database, queue and in-process
telemetry views from the sources explicitly supplied by the application.

## ✨ Features

- **Database inspector:** Read and filter configured SQLx tables, edit bounded
  primitive non-key values, delete one primary-key-selected row with explicit
  confirmation, and inspect a live ER diagram. Mutation contracts run against
  SQLite, PostgreSQL, MySQL, and MariaDB.
- **API playground:** Mount interactive Swagger UI from an `OpenApi` document
  explicitly supplied by the application; Studio does not infer arbitrary Axum
  routes.
- **Worker queue monitoring:** Inspect up to 50 records exposed by a supplied
  Rullst queue and request retries. SQLite removes successful jobs, so the view
  is not durable completion history.
- **Safe configuration view:** Environment values are deny-by-default redacted;
  typed runtime configuration is projected without URLs, paths, or secrets.
- **Feature flags manager:** Toggle database-backed flags and immediately
  invalidate already-warm `DbFeatureDriver` caches in the same process.
- **Distributed diagnostics:** Visualize local spans plus bounded,
  attribute-free v1 spans submitted through a separately mounted
  HMAC-authenticated push endpoint. The profiler reports slow SQL labels and a
  possible N+1 heuristic without accepting SQL text, bindings, headers, bodies,
  attributes, or error strings.
- **Cache inspector:** An explicitly supplied memory or Redis `Cache` exposes
  at most 100 metadata records. The UI renders opaque HMAC identifiers, value
  sizes and TTLs and can invalidate one entry; it never renders logical keys or
  values and does not offer bulk flush.
- **Local-first security:** The supported launcher binds to loopback, verifies
  the direct peer and local `Host` authority, and requires same-origin `Origin`
  on mutations.

## 🚀 Quickstart

Add `rullst-studio` to your project:

After that RC is published, install its exact train with
`cargo add rullst-studio@12.0.0-rc.1`.

### Launching the Studio

The supported v12 mode is a standalone debug server. `run_studio` and
`Studio::into_router(LocalStudioAccess::loopback_only())` reject release builds
and requests whose direct peer is not verified as loopback. Servers composing
the router manually must preserve Axum `ConnectInfo<SocketAddr>`. Non-local
`Host`, cross-origin requests, and unsafe requests without `Origin` fail closed.
Data-browser writes additionally require a crate-private marker created only by
that verified access middleware, so importing the raw browser router cannot
turn its mutation handlers into an unprotected database API.

The earlier `StudioLayer` embedded-production idea was never implemented.
Keeping an authenticated shared Studio is worthwhile, but it needs its own
explicit identity/RBAC/TLS policy before it can become a supported mode.

**CLI Launch:**

If you don't want to embed it, you can launch it statelessly via the Rullst CLI:

```bash
cargo rullst studio
```

### Distributed trace ingestion

Create one bounded store, give it to the local viewer, and separately mount the
push-only ingestion router on the application endpoint reachable by trusted
producers:

```rust,no_run
use rullst_studio::distributed_traces::{
    DistributedTraceStore, TraceIngestionKey, TraceIngestor,
};
use rullst_studio::{LocalStudioAccess, Studio};

# fn app() -> Result<(), Box<dyn std::error::Error>> {
let store = DistributedTraceStore::new(2_048)?;
let key = TraceIngestionKey::new(std::env::var("RULLST_TRACE_INGESTION_KEY")?)?;
let ingestion = TraceIngestor::new(store.clone(), "api-1", key)?;

// Mount this push-only router in the application. TLS, network policy, key
// distribution, rotation and endpoint availability remain deployment work.
let _application_ingestion = ingestion.router();

// The viewer still requires the debug/loopback boundary.
let _studio = Studio::new()
    .with_distributed_traces(store)
    .into_router(LocalStudioAccess::loopback_only())?;
# Ok(())
# }
```

Each `TraceIngestor` binds one exact producer name to one key; use separate
endpoints/keys over the same store for multiple producers. Producers use
`TraceBatchSigner::new("api-1", key)` to obtain the exact body and four headers.
The receiver accepts 1–128 spans in at most 128 KiB, verifies HMAC-SHA256, a
60-second clock window and a one-time nonce, validates the complete batch, and
then commits idempotently. The store is process-local and bounded; it is not an
OTLP collector, durable trace backend, key manager, or production Studio login.

## 🔐 Security Audit

`rullst-studio` currently supports verified-loopback development access. It does
not provide a built-in shared-intranet or production authentication mode. Do not
expose raw subrouters publicly; a future shared mode must fail closed behind
application-owned authentication, administrator authorization, TLS, and network
policy.

Migration actions remain CLI-driven because standalone Studio has no migration
registry. Queue, revenue, security, AI, and telemetry views expose only supplied
process-local sources; unsupported operations or disconnected integrations are
reported explicitly.

The SSE request view records method, URI, status and latency only. It does not
capture bodies or headers by default because those can contain authentication,
session, payment, and personal data. A successful Studio toggle invalidates all
already-warm `DbFeatureDriver` entries in the same process. Other processes and
direct database writers remain visible through the configured TTL unless the
application supplies distributed invalidation.

The trace ingestion endpoint is a distinct push-only router and contains no
Studio read or mutation routes. Keep the HMAC key in a secret manager, rotate
it through application deployment, synchronize producer clocks, use TLS and
network policy, and do not put personal data or secrets in operation labels.
The cache inspector uses the same verified local request marker as database
mutations for individual invalidation. Its HTML contains neither cache values
nor exact logical keys.

Data-browser mutation forms use database-inspected tables, columns and complete
primary keys. SQL values are parameterized; only text, signed integer, finite
float and Boolean codecs are writable. Primary keys and backend-specific types
remain read-only, a mutation must affect exactly one row, request bodies are
limited to 64 KiB, and deletion requires typing `DELETE <table>`. This is a
local developer database tool, not application authorization, tenant policy,
audit history, rollback, or a supported shared-production admin surface.

## 📚 Documentation

For supported usage and security boundaries, see the
**[Rullst Book](https://rullst.github.io/Rullst/book/)** and its
capability ledger. A production/shared Studio mode is not currently supplied.
