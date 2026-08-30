# Redis, Local Cache & Queue Drivers

Rullst v12 keeps cache and queue backends explicit. Enabling a Cargo feature
only compiles the adapter; it does not inspect `REDIS_URL`, switch drivers, or
silently fall back when Redis is unavailable.

## Cache choices

`Cache::memory()` uses a process-local `DashMap`. Values disappear on restart
and are not shared between replicas:

```rust
use rullst_core::cache::{Cache, CacheError};
use std::sync::Arc;

async fn load_profile(cache: &Cache) -> Result<Arc<String>, CacheError> {
    cache
        .remember("profile:42", 300, || async {
            Ok("serialized profile".to_string())
        })
        .await
}

let cache = Cache::memory();
let profile = load_profile(&cache).await?;
```

For a shared Redis cache, enable `cache-redis` (or the umbrella `redis`
feature) and construct the adapter explicitly:

```toml
[dependencies]
rullst-core = { version = "12.0.0-rc.1", features = ["cache-redis"] }
```

```rust
use rullst_core::cache::Cache;

let redis_url = std::env::var("REDIS_URL")?;
let cache = Cache::redis(redis_url)?;
cache.put("catalog:featured", "[...]", Some(600)).await?;
```

Constructing the driver validates the Redis URL but does not establish a
connection. Operations open a multiplexed async connection and return a typed
`CacheError` if Redis is unavailable. Choose an application-specific policy:
fail startup, retry with bounds, or explicitly select `Cache::memory()` for a
documented single-instance development mode.

The built-in Redis cache prefixes keys with `rullst:cache:`. `flush()` scans and
unlinks keys under that prefix; use dedicated credentials/database boundaries
when multiple applications share a Redis service.

## ORM `.remember(...)` queries

The ORM has a separate opt-in query-cache contract behind its `redis` feature:

```toml
[dependencies]
rullst-orm = { version = "12.0.0-rc.1", features = ["redis"] }
```

```rust
use rullst_orm::Orm;

let redis_url = std::env::var("REDIS_URL")?;
Orm::init_redis_with_namespace(&redis_url, "academy-production").await?;

let recent = User::query()
    .where_eq("active", true)
    .remember(30)
    .get()
    .await?;
```

Use a stable, unique namespace for every application that shares a Redis
database. Query keys bind that namespace, an opaque digest of the active tenant
scope, table, generated SQL and typed bindings. They do not expose raw tenant
identifiers. The older `Orm::init_redis(url)` API remains available and uses
`default`; only use it with a dedicated Redis database.

The failure and consistency rules are explicit:

- `remember(0)` is rejected.
- Missing Redis initialization is a configuration error for a remembered query
  outside a transaction.
- Redis command failures or corrupt JSON fall back to the authoritative
  database; a successful read is returned even if cache population fails.
- Explicit and task-scoped ORM transactions always bypass query cache.
- ORM writes do not automatically invalidate remembered results. Keep TTLs
  short enough for the domain, and do not cache authorization or other reads
  that require immediate freshness.

The Core `Cache` facade and ORM query cache use different keyspaces and APIs;
initializing one does not initialize the other.

## Queue choices

Rullst provides explicit SQLite and Redis queue constructors:

```toml
[dependencies]
rullst-core = { version = "12.0.0-rc.1", features = ["queue-sqlite"] }
serde_json = "1"
```

```rust
use rullst_core::queue::Queue;
use serde_json::json;

let queue = Queue::sqlite("sqlite://jobs.sqlite?mode=rwc").await?;
let job_id = queue
    .dispatch("send_receipt", json!({ "invoice_id": 42 }))
    .await?;
println!("queued {job_id}");
```

With `queue-redis`, construct `Queue::redis(redis_url)` instead. The Redis
driver uses atomic Lua transitions for pending, processing, failed, and
dead-letter state. Production validation must still cover Redis persistence,
eviction policy, credentials/TLS, failover, monitoring, and worker recovery in
the target topology.

There is no automatic interchange between the SQLite and Redis queues: they
store independent state. Switching a live deployment requires an explicit
drain/migration plan.

## Real-time boundary

Core's current WebSocket broadcast/presence helpers are process-local. Redis
Streams, Redis Pub/Sub, Kafka, and RabbitMQ transports remain roadmap work; do
not describe the cache or queue adapter as cross-instance real-time sync.

## Deployment checklist

- Choose the backend in application configuration and make fallback policy
  explicit.
- Never commit Redis credentials; prefer TLS and least-privilege network access.
- Namespace application/tenant keys above the built-in driver prefix where
  isolation is required. `TenantCache` supplies validated tenant namespaces.
- Test disconnects, timeouts, retries, eviction, restart, and worker recovery.
- Benchmark the deployed service. Rullst does not claim universal cache latency,
  memory usage, or infrastructure cost.
