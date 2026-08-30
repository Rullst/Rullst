# Rullst Core ⚙️

`rullst-core` encapsulates the foundational primitives, routing engines, state management, and configuration layers of the Rullst Framework. It acts as the beating heart that orchestrates HTTP handlers, middleware, and backend worker systems.

## ✨ Features

- **Typed Axum Integration:** Wraps Axum routing while retaining access to the underlying router and middleware ecosystem.
- **Runtime Telemetry:** Exposes process/runtime snapshots and tracing-span collection for consumers such as Rullst Studio.
- **Typed Failures:** Server, scheduler, queue, storage, and resilience APIs expose structured errors for fallible paths. The repository's zero-panic policy is CI-scoped, not an absolute runtime guarantee.
- **Dependency Injection:** Type-safe, intuitive global state management across routes and background workers.
- **Environment Management:** Native `dotenv` and TOML configuration loaders for different deployment targets (Staging, Production, Local).
- **Durable Scheduled Queues:** SQLite and Redis persist bounded `dispatch_at`
  timestamps and never claim a job before its millisecond due time. Delivery is
  poll-dependent and at-least-once.
- **Explicit Completion History:** SQLite deletes successful payloads by
  default. `Queue::sqlite_with_completed_history` opts into a bounded retained
  history for Studio/operations, with atomic pruning and an explicit purge API.

## 🚀 Usage

Most developers will not depend on `rullst-core` directly, as it is re-exported seamlessly through the primary `rullst` crate. 

If you are developing a plugin or advanced middleware for the Rullst ecosystem, you can add it explicitly:

```bash
cargo add rullst-core
```

Core is runtime-only by default. Add just the database capabilities the
application needs:

```toml
[dependencies]
rullst-core = { version = "12", features = ["orm", "queue-sqlite"] }
```

Enable `orm`, `queue-sqlite`, `queue-redis`, or `telemetry` only when that
integration is required. The primary `rullst` crate keeps `orm` and
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

For an architectural deep-dive into Rullst Core's event loop and middleware lifecycle, please visit the **[Rullst Book](https://rullst.github.io/Rullst/book/index.html)**.
