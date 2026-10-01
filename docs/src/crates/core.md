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
the historical operation, which actually removed failed jobs. The Redis driver
implements all three: `list_all_jobs` returns at most 1,000 rows (failed jobs
and dead letters newest first, then processing, pending and scheduled jobs, read
without one atomic snapshot; `created_at` is empty because Redis does not record
it), `retry_failed_job` moves a failed job to the tail of the pending list while
keeping its attempt counter, and `purge_failed_jobs` deletes every failed job
and dead letter.

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
The memory cache stores a TTL too large for the monotonic clock (such as
`u64::MAX`) as non-expiring instead of panicking. A read that finds an expired
entry removes the key only while it still holds that expired value, so a
concurrent refill is kept.

SQLite deletes successful jobs by default. Applications that need a real
Studio/operations history can opt in with
`Queue::sqlite_with_completed_history(database_url, retained_jobs)`. The
validated limit is 1–100,000 records; status transition and pruning commit in
one transaction, and `purge_completed_history` removes the retained successes.
Rows still contain the original payload, so access control and retention policy
belong to the host. Redis/custom drivers do not inherit this policy implicitly.
The Redis driver bounds its own failure state instead: failed jobs (with
payloads) and dead letters are each retained up to 10,000 entries by default,
the oldest evicted atomically, and
`RedisDriver::try_with_failure_retention(failed_jobs, dead_letters)` accepts
1–100,000 for each. Failures recorded before that bound existed are not
indexed and are never evicted automatically.

`Queue::dispatch_at` persists a due timestamp for at most 366 days through the
built-in SQLite and Redis drivers. SQLite filters claims by local wall-clock
milliseconds; Redis atomically promotes bounded batches using Redis server time.
Neither backend claims a scheduled job early. Execution starts on the first
worker poll after it becomes due and retains the queue's at-least-once semantics.
`Worker` drives each `pop` to completion instead of racing it against
completions or shutdown, because both built-in claims commit before the future
resolves. A job claimed after graceful shutdown was requested is requeued.
When a handler finishes while its timeout or a graceful shutdown is being
processed, the worker records the handler's own result: only a handler that
was actually cancelled is failed as timed out or requeued, so a success is
never reported as a timeout or run again.
Stalled-lease recovery runs when a worker starts and then every
`min(stalled_after, 60 s)`, and it is queue-wide: it returns every processing
lease in the shared SQLite table or Redis namespace that is older than the
recovering worker's `stalled_after`, including leases of other workers. In the
unpublished v13 source, workers claim through `QueueDriver::pop_with_lease`
with their own `stalled_after`; SQLite and Redis store that lease with the
claim and recovery honours it whatever age the recovering worker uses, so a
pool with a short `stalled_after` no longer requeues a slower pool's running
job. Claims without a lease (older workers, custom drivers or direct `pop`
calls) still stall after the recovering worker's age: while any exist, every
worker that shares a queue must use a `stalled_after` longer than the longest
`job_timeout` of any of them.
A job that crashes, aborts or hangs its worker would otherwise be recovered and
claimed forever, so the SQLite and Redis drivers count stalled leases per job
and fail the job, instead of requeuing it, when its fifth lease stalls. The
failure is listed and retryable like any other failed job, and
`retry_failed_job` restarts the count. In the unpublished v13 source,
`SqliteDriver::try_with_max_stalled_leases` and
`RedisDriver::try_with_max_stalled_leases` change the ceiling (1–1,000, default
`DEFAULT_MAX_STALLED_LEASES` = 5).
Worker transitions are fenced by the claim's attempt number. The SQLite and
Redis drivers complete, fail or requeue a job only while it is still processing
under the attempt that `pop` returned, so a worker whose lease was recovered and
claimed again receives a `StateTransition` error instead of finishing, failing
or deleting the newer claim. The new `QueueDriver::mark_complete_attempt`,
`mark_failed_attempt` and `requeue_attempt` methods default to the unfenced
methods, so custom drivers keep their behaviour until they override them. The
stale handler may still have run its side effects (delivery stays
at-least-once), and SQLite `retry_failed_job` restarts the attempt counter, so
a worker that stays stale across a manual retry and a new claim with the same
attempt number is not fenced.
A worker that claims a job whose name it has no handler for hands the claim
back instead of failing it. SQLite and Redis make the job claimable again after
five seconds, behind jobs that are already due, so a worker that registered the
name (for example a newer version during a rolling deploy) can run it; the
delay keeps the claiming worker out of a hot loop, and it still reports
`HandlerNotFound` each time. A job that no running worker can handle therefore
stays pending and is re-offered every five seconds instead of being failed.
Custom drivers that do not implement `QueueDriver::requeue_attempt_after` keep
the previous behaviour and fail the job.
`ValidatedForm`/`ValidatedJson` failures keep REST status codes (`400`, `413`,
`415` or `422` JSON) for other clients, but an HTMX request receives its
escaped HTML fragment with `200 OK` and an `X-Rullst-Validation-Status` header
carrying that status, because htmx swaps only successful responses by default.
`Scheduler::task` takes a POSIX five-field expression (`minute hour
day-of-month month day-of-week`) evaluated in UTC. Day-of-week accepts 0-7
(0 and 7 are Sunday, 1 is Monday) and names, so `0 9 * * 1-5` runs Monday to
Friday. When both day fields are restricted, a day matching either one runs
the task (`0 0 1 * 1` is the 1st plus every Monday); a field starting with `*`
keeps the intersection. Earlier releases passed the fields to the `cron`
crate unchanged, where 1 was Sunday and 0 was rejected. Messaging's durable
recurring publications keep their documented `cron`-crate projection.
`WorkerHandle` and `SchedulerHandle` buffer at most 256 undrained errors. Once
the buffer is full, newer errors are dropped, counted by `dropped_errors()` and
emitted as `tracing` warnings, so a handle that is kept alive but never drained
does not grow memory. Drain `next_error` (for example from a supervising task)
to observe every failure. A scheduler attached with `Server::schedule` is
drained by the server: each task failure is logged as a `tracing` error on the
`rullst::scheduler` target when reported, and a past task failure no longer
turns a clean shutdown into `Err(ServerError::Scheduler)`; only a failed
scheduler loop does.
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
  process uptime. Unsupported probes return `None`: RSS and CPU have probes
  on Linux and Windows only, so macOS reports neither. On Linux, RSS comes from
  `VmRSS` in `/proc/self/status` (correct on 16/64 KiB page kernels), and CPU
  percent is process CPU time over wall time: the host-wide `/proc/stat` delta
  is scaled by its host CPU count, not by the cgroup-limited
  `available_parallelism`, so a container saturating a 2-CPU quota reports
  about 200%.
- **Prometheus `/metrics` Exporter:** Text-format metrics served at `GET /metrics`; formatting and collection have bounded runtime cost.
- **Kubernetes probe routes (`rullst::health`):** the simple `health_router`
  reports process availability and uptime. `Server` records the health and
  Radar uptime origin when it starts, unless `init_health_boot_time` or
  `init_radar` was called earlier; without a `Server`, call them yourself. The opt-in
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
- **Redacted configuration `Debug`:** `DatabaseConfig` (and therefore
  `RullstConfig`) prints only the database URL scheme, such as
  `postgres://<redacted>`; `db::ReplicationConfig` redacts `auth_token` and
  prints only the `sync_url` scheme. Fields stay public and unchanged.
- **Durable scheduled queues:** SQLite and Redis persist bounded due timestamps;
  the live Redis CI contract proves that an immediate job remains claimable
  while a future job stays unavailable.
- **Opt-in completed-job monitoring:** SQLite can retain and atomically prune a
  configured number of successful jobs; the privacy-safe default remains
  immediate deletion.
- **Bounded token-bucket rate limiter:** `RateLimiter` keys IPv4 peers per
  address and IPv6 peers per /64 by default. It tracks at most 100,000 keys,
  drops fully refilled buckets and evicts the least recently used ones beyond
  that cap; a key longer than 128 bytes (for example from a custom extractor
  returning a token header) is stored as its SHA-256 digest, so the map stays
  bounded in bytes too. State is process-local, not a distributed limit. When attached to
  `Server`, the limiter and the Traffic Shield let exact `GET`/`HEAD /health`
  and `/ready` probes through, so load shedding or an exhausted bucket cannot
  fail a liveness probe. In a debug Development server with `cargo rullst dev`
  reloading, its `/_rullst/dev-generation` poll and `/_rullst/dev-reload.js`
  also bypass both and are not access-logged.
- **Trusted-proxy client resolution (v13):** `Server::trusted_proxies`
  mounts `security::TrustedProxyLayer` outside every other framework layer.
  Only a socket peer inside the listed networks may report the client through
  `X-Forwarded-For` or RFC 7239 `Forwarded`; the resolved address replaces
  `ConnectInfo`, so existing rate limiters and lockouts use it unchanged. See
  [Running behind a reverse proxy](#running-behind-a-reverse-proxy).
- **Bounded database flag cache:** `DbFeatureDriver` caches a found flag, a
  flag without a row and a failed or timed-out lookup (missing table,
  unavailable database) for its TTL, so an undefined flag does not query the
  database on every evaluation. A failed refresh keeps serving the last value
  read; one lookup waits at most two seconds and each driver caches at most
  4,096 flag names.
- **Feature flag buckets:** percentage rollouts and A/B variants in the Env,
  TOML, Memory and DB drivers use `calculate_hash_bucket`, a versioned
  SHA-256 hash over a domain tag, the length-prefixed flag and the identifier.
  It gives every toolchain, platform and replica the same assignment.
  Earlier releases used `std`'s unspecified `DefaultHasher`, so upgrading
  reassigns users to buckets once; percentages and variant weights are kept.
  `TomlFeatureDriver::reload` parses into a new map and swaps it in at once,
  so concurrent evaluations never see a flag as unset mid-reload. It reads
  `[features]` with a TOML parser, so quoted flag names (`"checkout.v2"`),
  `# ` inside strings and any valid header spelling work, and dotted keys or
  `[features.<group>]` tables become dotted flag names; a file that is not
  valid TOML falls back to the earlier line reader. When an A/B split's weights
  sum to less than 100, an identifier outside them gets the variant
  `"disabled"` from the driver that defines the flag; it no longer falls
  through to a lower-priority driver's split. The unpublished v13
  `FeatureManager::overrides()` returns the first-priority
  `MemoryFeatureDriver` of `FeatureManager::default()` (and of the global
  `feature::manager()` when it uses the default pipeline), so programmatic
  overrides reach it.
- **Shared project settings (internal, v13):** `server::ProjectSettings` and
  `server::read_project_setting` resolve a setting from the process
  environment first and then the project's `.env`, which never overrides the
  environment, and `ProjectSettings::environment` applies the `Server`
  precedence for `RULLST_ENV`/`APP_ENV`/`[app].env`. Errors never contain
  `.env` content. They are `#[doc(hidden)]` support for first-party crates such
  as `rullst-mail`, not a stable extension point.
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

### Running behind a reverse proxy

Behind a load balancer or TLS terminator every connection comes from the proxy,
so limits and lockouts would treat all clients as one. List the networks your
own proxies connect from, and nothing broader:

```rust,no_run
use rullst_core::{Router, Server, security::TrustedProxyConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proxies = TrustedProxyConfig::new(["10.0.0.0/8", "fd00::/8"])?
        .trust_forwarded_proto(true);
    Server::new(Router::new())
        .trusted_proxies(proxies)
        .run(3000)
        .await?;
    Ok(())
}
```

Or configure the same policy in `Rullst.toml`; a builder policy replaces it:

```toml
[security]
trusted_proxies = ["10.0.0.0/8", "fd00::/8"]
trusted_proxy_header = "x-forwarded-for" # or "forwarded" (RFC 7239)
trust_forwarded_proto = true
```

Forwarding headers from any other peer are ignored. The chain is read right to
left, skipping trusted hops, and the first untrusted address becomes the
client: `ConnectInfo<SocketAddr>` is replaced by that IP with port 0, and a
`ClientAddr` extension records the client, the original socket peer and whether
a proxy supplied it. A missing or malformed header from a trusted proxy keeps the
proxy address and logs one `tracing` event without header contents. Enable
`trust_forwarded_proto` only when your proxies overwrite `X-Forwarded-Proto`
(or set `proto=`); `ClientAddr::forwarded_proto` then reports the scheme, and
Nexus accepts an HTTPS report as its TLS evidence. Any host inside a listed
network can choose the client address, so never list client-reachable ranges.

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
