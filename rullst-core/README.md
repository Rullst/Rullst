# Rullst Core ⚙️

`rullst-core` encapsulates the foundational primitives, routing engines, state management, and configuration layers of the Rullst Framework. It acts as the beating heart that orchestrates HTTP handlers, middleware, and backend worker systems.

## ✨ Features

- **Typed Axum Integration:** Wraps Axum routing while retaining access to the underlying router and middleware ecosystem.
- **Bounded Server Functions:** `#[server_function]` shares the versioned
  `rullst.client` v1 envelope between native Axum routes and Wasm callers, with
  a 256 KiB body ceiling, correlation checks, same-origin paths, CSRF header
  forwarding, and message-free typed failures. Identity and domain policy stay
  server-owned.
- **Runtime Telemetry:** Exposes process/runtime snapshots and tracing-span collection for consumers such as Rullst Studio.
- **Distributed Tracing Candidate (v13):** Optional `telemetry::distributed`
  adds explicit parent trust, approved operation labels, minimized OTLP export
  and an owned lifecycle. Independent Messaging processes and a standard TLS
  collector passed local acceptance; hosted source/package admission is pending.
  See [the profile](https://github.com/Rullst/Rullst/blob/main/docs/src/distributed-tracing.md).
- **Recoverable Live UI Candidate (v13):** Explicitly mounted typed views add
  current authorization, complete snapshots, transactional revision conflicts
  and bounded WebSocket recovery. The included browser module never replays
  uncertain mutations. Local protocol/Chromium restart acceptance passed;
  hosted source/package admission is pending. See [adoption and limits](https://github.com/Rullst/Rullst/blob/main/docs/src/live-recovery.md).
- **Lifecycle-aware Readiness:** An opt-in process lifecycle gates new requests
  during startup, dependency unavailability, and graceful drain. It accepts at
  most 32 immutable component labels and exposes counts—not labels or errors—on
  `/ready`; dependency checks and multi-replica coordination remain host work.
- **Typed Failures:** Server, scheduler, queue, storage, and resilience APIs expose structured errors for fallible paths. The repository's zero-panic policy is CI-scoped, not an absolute runtime guarantee.
- **Dependency Injection:** Type-safe, intuitive global state management across routes and background workers.
- **Environment Management:** Native `dotenv` and TOML configuration loaders for different deployment targets (Staging, Production, Local).
- **Durable Scheduled Queues:** SQLite and Redis persist bounded `dispatch_at`
  timestamps and never claim a job before its millisecond due time. Delivery is
  poll-dependent and at-least-once.
- **Explicit Completion History:** SQLite deletes successful payloads by
  default. `Queue::sqlite_with_completed_history` opts into a bounded retained
  history for Studio/operations, with atomic pruning and an explicit purge API.
- **Metadata-only Cache Inspection:** Memory and Redis drivers can return a
  sorted snapshot of at most 200 logical keys, UTF-8 value lengths and TTLs
  without returning values. Custom drivers fail explicitly unless they opt in;
  authorization and safe rendering of logical keys remain caller policy.
- **Offline Sync Foundation:** The optional native `offline-sync` feature
  provides bounded replay-safe mutations, explicit conflict resolution, and
  account-bound AES-256-GCM snapshots. Its static-dispatch coordinator bounds
  requests, times them out, and detects stalled cursors. Applications still own
  Keychain/Keystore access, atomic platform persistence, authenticated HTTP,
  retry, and background scheduling.
- **Private Object Storage Candidate (v13):** The optional `storage-s3` feature
  adds bounded S3/R2 upload, download, metadata, deletion and short-lived signed
  GET URLs to the existing storage and tenant APIs. Configuration is explicit;
  offline credentials select a bounded deterministic store. Source/package
  admission passed in PR #236; release admission and owner-account interoperability
  remain outstanding.
  See the [candidate contract](https://github.com/Rullst/Rullst/blob/main/docs/src/private-object-storage.md).
- **Resumable Private Upload Candidate (v13):** Separate `storage-multipart`
  adds exact-size parts, SHA-256 checks, encrypted tenant/object-bound recovery
  records, completion reconciliation and abort. Native disposable S3 restart and
  adversarial protocol tests passed locally; full hosted admission remains required.
  See [private multipart uploads](https://github.com/Rullst/Rullst/blob/main/docs/src/private-multipart-uploads.md).

## 🚀 Usage

Most developers will not depend on `rullst-core` directly, as it is re-exported seamlessly through the primary `rullst` crate. 

If you are developing a plugin or advanced middleware for the Rullst ecosystem, you can add it explicitly:

Install the exact stable train with
`cargo add rullst-core@12.1.0`.

Core is runtime-only by default. Add just the database capabilities the
application needs:

```toml
[dependencies]
rullst-core = { version = "12.1.0", features = ["orm", "queue-sqlite", "offline-sync"] }
```

Enable `orm`, `queue-sqlite`, `queue-redis`, `offline-sync`, or `telemetry` only
when that integration is required. The primary `rullst` crate keeps `orm` and
`queue-sqlite` in its default feature set for application compatibility.

Both built-in queue drivers implement `Queue::dispatch_at` for schedules up to
366 days ahead. SQLite performs an automatic additive schema migration; Redis
uses a sorted set and server time, with a digest-pinned live CI contract. Custom
drivers fail with `QueueError::Unsupported` for future jobs until they implement
the scheduling method explicitly.

Successful SQLite jobs are removed unless the application explicitly calls
`Queue::sqlite_with_completed_history(database_url, retained_jobs)`. The limit
must be between 1 and 100,000; completion and pruning share one transaction.
Retained rows include the original payload, so the application must restrict
Studio/inspection access and choose an appropriate retention policy. Use
`Queue::purge_completed_history` to remove them.

`Cache::inspect(limit)` is opt-in diagnostic data, not an administration
endpoint. The limit must be 1–200. `CacheEntryMetadata` deliberately redacts
the logical key from `Debug` and never carries the cached value, but
`logical_key()` still returns application data to an authorized caller. Rullst
Studio converts it into a process-bound opaque token before rendering it.

For orchestrated deployments, construct `ApplicationLifecycle`, mount
`health_router_with_lifecycle(lifecycle.clone())`, then pass the same value to
`Server::with_lifecycle`. The server marks startup complete after binding,
begins drain before Axum waits for accepted requests, and marks startup failure
or termination as stopped. `run_with_shutdown` permits a caller-owned trigger.
Admission follows the ordinary response body through completion, error or drop,
including streamed data/trailers. It does not prove client receipt or track
upgraded connections. `wait_for_drain` is bounded; supervisors separately own
process kill deadlines and application WebSocket/background-task termination.
The process-local registry does not probe dependencies, authorize application
requests, coordinate replicas, or guarantee load-balancer propagation.

### Minimal HTTP Server

```rust
use rullst_core::{Router, Server};
use rullst_core::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/health", get(|| async { "ok" }));
    Server::new(app).run(3000).await?;
    Ok(())
}
```

## 🔐 Security Audit

Repository workflows exercise Core with unit, integration, fuzz, and Miri jobs within their declared scopes. Consult the exact workflow run and commit for evidence; these tools do not prove the absence of every panic, leak, or vulnerability.

## 📚 Documentation

For an architectural deep-dive into Rullst Core's event loop and middleware lifecycle, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/)**.
