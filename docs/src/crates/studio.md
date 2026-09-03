# Rullst Studio 📊

> [!IMPORTANT]
> This page documents the unreleased v12 source. Use a path dependency from
> this checkout until the planned `12.0.0-rc.1` is published.

`rullst-studio` is the built-in, local-first administration and monitoring
dashboard for Rullst. It exposes bounded database, queue, cache and telemetry
views from the sources explicitly supplied by the application.

## ✨ Features

- **Database inspector:** Read and filter configured SQLx tables, edit bounded
  primitive non-key values, delete one complete-primary-key-selected row with
  exact confirmation, and inspect a live ER diagram. SQLite, PostgreSQL, MySQL
  and MariaDB run executable mutation contracts.
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
- **Distributed diagnostics:** Visualize in-process sources plus bounded,
  attribute-free v1 spans from a separately mounted HMAC-authenticated push
  endpoint. Slow-query and repeated-label findings are heuristics; no SQL text,
  bindings, attributes, headers, bodies or error details are accepted.
- **Cache inspector:** An explicitly supplied memory or Redis `Cache` exposes
  bounded metadata and individual invalidation through opaque process-bound
  tokens. Values, exact keys and bulk flush remain unavailable in the UI.
- **Local-first security:** The supported launcher binds to loopback, verifies
  the direct peer and local `Host` authority on every request, and requires a
  same-origin `Origin` header for mutations.

## 🚀 Quickstart

After the RC is published, add its exact train with
`cargo add rullst-studio@12.0.0-rc.1`.

### Launching the Studio

The supported v12 mode is a standalone debug server. `run_studio` and
`Studio::into_router(LocalStudioAccess::loopback_only())` reject release builds
and requests whose direct peer is not verified as loopback. Servers composing
the router manually must preserve Axum `ConnectInfo<SocketAddr>`. The access
capability also rejects DNS-rebinding-style non-local `Host` values,
cross-origin requests, and unsafe requests without an `Origin` header.

The earlier `StudioLayer` embedded-production idea was never implemented.
Keeping an authenticated shared Studio is worthwhile, but it needs its own
explicit identity/RBAC/TLS policy before it can become a supported mode.

**CLI Launch:**

If you don't want to embed it, you can launch it statelessly via the Rullst CLI:

```bash
cargo rullst studio
```

### Authenticated trace producers

```rust,no_run
use rullst_studio::distributed_traces::{
    DistributedTraceStore, TraceIngestionKey, TraceIngestor,
};
use rullst_studio::{LocalStudioAccess, Studio};

# fn build() -> Result<(), Box<dyn std::error::Error>> {
let store = DistributedTraceStore::new(2_048)?;
let key = TraceIngestionKey::new(std::env::var("RULLST_TRACE_INGESTION_KEY")?)?;
let ingestion = TraceIngestor::new(store.clone(), "api-1", key)?;
let _push_only_application_router = ingestion.router();
let _local_viewer = Studio::new()
    .with_distributed_traces(store)
    .into_router(LocalStudioAccess::loopback_only())?;
# Ok(())
# }
```

Each ingestor binds one exact producer name to one key; mount separate producer
endpoints over the same store when needed. `TraceBatchSigner::new` binds the
same pair and produces the byte-identical JSON body plus source,
timestamp, nonce and signature headers. Mount the push-only router under an
application path; it exposes no Studio reads or administrative mutations. The
application/deployment still owns TLS, network policy, key distribution and
rotation, clock synchronization, availability and label redaction. The store
is bounded process memory, not OTLP or durable trace storage.

## 🔐 Security Audit

`rullst-studio` currently supports verified-loopback development access. It does
not provide a built-in shared-intranet or production authentication mode. Do not
expose raw subrouters publicly; a future shared mode must fail closed behind
application-owned authentication, administrator authorization, TLS, and network
policy.

The built-in migration page intentionally links to `cargo rullst db:*` commands.
The compatibility HTTP mutation handlers return `501 Not Implemented` because
the standalone Studio has no configured migration or seeder registry. Queue and
revenue panels likewise show only data supplied by the selected driver or
application; unsupported operations return errors instead of simulated success.

The SSE request view records method, URI, status and latency only. It does not
capture bodies or headers by default because those can contain authentication,
session, payment, and personal data. A successful Studio toggle invalidates all
already-warm `DbFeatureDriver` entries in the same process. Other processes and
direct database writers remain visible through the configured TTL unless the
application supplies distributed invalidation.

`Studio::with_cache` opts one `Cache` into metadata-only inspection. The page
shows a keyed opaque identifier, UTF-8 value byte length and remaining TTL for
at most 100 entries. It never returns the value or logical key, offers no bulk
flush, and requires the verified local mutation marker to invalidate one
entry. Memory and Redis implement this contract; custom drivers return an
explicit unsupported state unless they implement bounded inspection.

## 📚 Documentation

For supported usage and security boundaries, see this book and the
[capability ledger](../capability-ledger.md#security-authentication-studio-and-nexus).
