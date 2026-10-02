# Redis, Local Cache & Queue Drivers

Rullst v12 keeps cache and queue backends explicit. Enabling a Cargo feature
only compiles the adapter; it does not inspect `REDIS_URL`, switch drivers, or
silently fall back when Redis is unavailable.

## Cache choices

`Cache::memory()` uses a process-local `DashMap`. Values disappear on restart
and are not shared between replicas. A TTL too large for the monotonic clock
to represent, such as `u64::MAX`, is stored as "never expires" instead of
panicking:

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

# async fn read_profile() -> Result<(), CacheError> {
let cache = Cache::memory();
let profile = load_profile(&cache).await?;
# let _ = profile;
# Ok(())
# }
```

For a shared Redis cache, enable `cache-redis` (or the umbrella `redis`
feature) and construct the adapter explicitly:

```toml
[dependencies]
rullst-core = { version = "12.1.0", features = ["cache-redis"] }
```

```rust,no_run
use rullst_core::cache::Cache;

async fn cache_featured_catalog() -> Result<(), Box<dyn std::error::Error>> {
let redis_url = std::env::var("REDIS_URL")?;
let cache = Cache::redis(redis_url)?;
cache.put("catalog:featured", "[...]", Some(600)).await?;
Ok(())
}
```

A zero TTL removes the key, and a TTL beyond Redis's expiry range is stored
without expiry, as in the memory driver. Constructing the driver validates the
Redis URL but does not establish a connection. The first operation opens one
multiplexed async connection that later operations share; it is not reopened
per command. When that connection breaks, the failing operation returns a typed
`CacheError` and the next one reconnects. Operations also return `CacheError`
if Redis is unavailable. Choose an application-specific policy: fail startup,
retry with bounds, or explicitly select `Cache::memory()` for a documented
single-instance development mode.

The built-in Redis cache prefixes keys with `rullst:cache:`. `flush()` scans and
unlinks keys under that prefix; use dedicated credentials/database boundaries
when multiple applications share a Redis service.

## ORM `.remember(...)` queries

The ORM has a separate opt-in query-cache contract behind its `redis` feature:

```toml
[dependencies]
rullst-orm = { version = "12.1.0", features = ["redis"] }
```

The generated `.remember(...)`, cache invalidation, `orm:events:*` publications
and Redis hash helpers follow this ORM feature (or the facade's `redis` /
`orm-redis`). From 13.0 the application does not declare a `redis` feature of
its own; earlier macro output checked the application's features instead.

```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
struct User {
    id: i32,
    active: bool,
}

async fn load_recent_users() -> Result<(), Box<dyn std::error::Error>> {
let redis_url = std::env::var("REDIS_URL")?;
Orm::init_redis_with_namespace(&redis_url, "academy-production").await?;

let recent = User::query()
    .where_eq("active", true)
    .remember(30)
    .get()
    .await?;
let _ = recent;
Ok(())
}
```

Generated `save_to_redis`/`get_from_redis`/`increment_redis_field` model
hashes use the same namespace, and tenant models bind the active tenant into
their key and require `with_tenant(...)`. Hashes that 12.0 and 12.1 wrote under
`orm:<table>:<id>` are handled as described in
[Model hashes stored by 12.0 and 12.1](#model-hashes-stored-by-120-and-121).

Use a stable, unique namespace for every application that shares a Redis
database. Query keys bind that namespace, the table, generated SQL and typed
bindings and, for a model with a `tenant_column`, an opaque digest of the
active tenant; other models share one global entry. They do not expose raw
tenant identifiers. Entries live under `rullst:orm:cache:v4:<namespace>:` and
model hashes under `rullst:orm:hash:v1:<namespace>:`. Cache entries written by
earlier versions are never read again and expire through their TTL. The older
`Orm::init_redis(url)` API remains available and uses `default`; only use it
with a dedicated Redis database.

The failure and consistency rules are explicit:

- `remember(0)` is rejected.
- Missing Redis initialization is a configuration error for a remembered query
  outside a transaction.
- Redis command failures or corrupt JSON fall back to the authoritative
  database; a successful read is returned even if cache population fails.
- Explicit and task-scoped ORM transactions always bypass query cache.
- Generated model saves/deletes invalidate the table's global entries and, for
  a tenant-scoped model, those of the tenant active at the write, only after
  commit; rollback keeps existing cache entries. Each write follows the
  table's key index (at most 10,000 live keys), never a keyspace `SCAN`. Raw
  SQL, bulk builders and writes from another process cannot be inferred. Keep
  defensive TTLs and do not cache authorization or reads that require a
  stronger distributed consistency contract.

The Core `Cache` facade and ORM query cache use different keyspaces and APIs;
initializing one does not initialize the other.

### Model hashes stored by 12.0 and 12.1

Releases up to 12.1 stored every generated model hash under the shared key
`orm:<table>:<id>`. For a model without a tenant scope, `get_from_redis` reads
that key while the namespaced hash is missing, and the next `save_to_redis` or
`increment_redis_field` moves it to the namespaced key in one Redis script
(every field is kept; an existing namespaced hash wins and the stale legacy
hash is removed). Nothing has to be run. Applications that share one Redis
database also shared these keys, so the first one to write a hash takes it
over. A 12.x hash of a model with `#[orm(encrypted)]` fields holds them in
plaintext, so reading it fails closed until `save_to_redis()` rewrites it.

Tenant models never read, move or delete the 12.x key, which every tenant
shared, so until migrated `get_from_redis` returns `None` and
`increment_redis_field` starts from zero. Migrate them once after deploying
13:

1. List the keys of each tenant model table:
   `redis-cli --scan --pattern 'orm:<table>:*'`.
2. Read each hash with `HGETALL`; every value is the JSON of one field.
3. Check its tenant column against the tenant that owns row `<id>` in the
   database, and skip mismatches: another tenant may have overwritten it.
4. Decode the hash into the model (for example with `serde_json` from the
   parsed values), or reload the row when the hash only cached it, and call
   `with_tenant(tenant, model.save_to_redis())`.
5. Remove the 12.x key with `UNLINK`.

## Queue choices

Rullst provides explicit SQLite and Redis queue constructors:

```toml
[dependencies]
rullst-core = { version = "12.1.0", features = ["queue-sqlite"] }
serde_json = "1"
```

```rust,no_run
use rullst_core::queue::Queue;
use serde_json::json;

async fn enqueue_receipt() -> Result<(), Box<dyn std::error::Error>> {
let queue = Queue::sqlite("sqlite://jobs.sqlite?mode=rwc").await?;
let job_id = queue
    .dispatch("send_receipt", json!({ "invoice_id": 42 }))
    .await?;
println!("queued {job_id}");
Ok(())
}
```

With `queue-redis`, construct `Queue::redis(redis_url)` instead. The Redis
driver uses atomic Lua transitions for pending, processing, failed, and
dead-letter state. Failed jobs (with their payloads) and dead letters are each
retained up to 10,000 entries; recording one more evicts the oldest in the same
script. `RedisDriver::try_with_failure_retention(failed_jobs, dead_letters)`
accepts 1–100,000 for each (pass the configured driver to `Queue::custom`).
Failed jobs recorded before this bound was introduced (for example by 12.x
workers) are not indexed, so they are neither counted, listed nor evicted;
`retry_failed_job` still finds them by ID and `purge_failed_jobs` removes them.
`list_all_jobs` returns at most 1,000 rows, failures and dead letters first,
and `list_job_previews` (unpublished v13) returns the same rows with each
payload and error cut to a byte budget inside one Lua script;
`retry_failed_job` moves a failed job back to the pending list with its attempt
counter intact; `purge_failed_jobs` deletes every failed job and dead letter. Like the cache, each driver shares one lazily opened
multiplexed connection and reconnects after a failed operation. Production validation must still cover Redis persistence,
eviction policy, credentials/TLS, failover, monitoring, and worker recovery in
the target topology.

There is no automatic interchange between the SQLite and Redis queues: they
store independent state. Switching a live deployment requires an explicit
drain/migration plan.

## Real-time boundary

Core's current WebSocket broadcast/presence helpers are process-local. They
release channels without subscribers and empty presence rooms, so the registry
tracks live rooms rather than every name ever used. Redis Pub/Sub, Kafka, and
RabbitMQ real-time transports remain roadmap work; do not describe the cache or
queue adapter as cross-instance real-time sync. The unpublished v13
[Redis Streams messaging](redis-messaging.md) candidate in `rullst-messaging`
is at-least-once brokered delivery, not WebSocket fan-out.

## Deployment checklist

- Choose the backend in application configuration and make fallback policy
  explicit.
- Never commit Redis credentials; prefer TLS and least-privilege network access.
- Namespace application/tenant keys above the built-in driver prefix where
  isolation is required. `TenantCache` supplies validated tenant namespaces as
  `tenants:<tenant>:<key>`, where `%` and `:` in the tenant segment are written
  as `%25` and `%3A` so a key containing `:` cannot reach another tenant.
- Test disconnects, timeouts, retries, eviction, restart, and worker recovery.
- Benchmark the deployed service. Rullst does not claim universal cache latency,
  memory usage, or infrastructure cost.
