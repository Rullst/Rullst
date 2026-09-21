# Redis Streams messaging

The v13 candidate adds `RedisBroker` to the existing `rullst-messaging` crate.
Enable `redis-streams` directly, or `messaging-redis` on the `rullst` facade.
The default messaging build remains network-free; the facade feature does not
enable ORM, mail, cache or Core queues. Full hosted source/package admission is
pending. Local acceptance uses an owned, digest-pinned Redis 7.4 service, not an
owner's account or production data.

## Configuration and startup

```toml
[dependencies]
rullst = { version = "13.0.0-alpha.1", default-features = false, features = ["messaging-redis"] }
```

This version identifies the unpublished candidate. Use reviewed source or its
extracted packages until release admission completes.

```rust,no_run
# #[cfg(feature = "messaging-redis")]
# async fn startup() -> Result<(), Box<dyn std::error::Error>> {
use rullst::messaging::{BrokerConfig, RedisBroker, RedisBrokerConfig};

let configuration = RedisBrokerConfig::try_new(
    BrokerConfig::try_new("academy-notifications")?,
    "deployment-2026-09", // Stable deployment generation, shared by all workers.
    std::env::var("MESSAGING_REDIS_ENDPOINT")?, // rediss://host:port/0
    std::env::var("MESSAGING_REDIS_USERNAME")?,
    std::env::var("MESSAGING_REDIS_PASSWORD")?,
)?.require_production()?;

// An operator previously called RedisBroker::provision with this configuration.
let broker = RedisBroker::connect(configuration).await?;
# let _ = broker;
# Ok(())
# }
```

Call `provision` once for an intentionally empty namespace. On ordinary process
startup call `connect`, which checks stored generation and limits and rejects
missing state. Never respond to a startup error by automatically provisioning a
replacement. Keep the deployment generation stable across process restarts;
restoring a backup requires explicit reconciliation, not a new random generation
on every startup. Changing generation or limits for retained state fails.

Credentials are explicit and separate from the URL. Only database zero is
supported. URL credentials, fragments, query options and insecure TLS overrides
are rejected. TLS validates the certificate and hostname. `with_ca_certificate`
accepts an explicit PEM trust bundle up to 64 KiB; it preserves hostname checks.
The normal trust roots come from the maintained Redis client's WebPKI profile.
The host owns credential rotation and secret storage. Diagnostics redact endpoint,
password, deployment generation, payload, header values and ACK capabilities.

An empty or `mock_*` password selects the explicit process-local `InMemoryBroker`
fixture without a network connection. Its clones share state; separate `connect`
calls do not create a distributed fixture. `require_production` rejects mock
credentials and the `allow_loopback_for_tests` transport override. That override
permits plaintext only for a literal loopback IP and cannot be enabled after
sealing a configuration for production. Live connection or protocol errors
never activate the fixture. The mock implements the common broker contract;
Redis-specific byte budgets and remote failures require the actual service tests.

## Supported contract

Redis Streams retains canonical `rullst.messaging.v1` envelopes. Rullst owns the
group, ready-time, attempt, terminal and lease indexes in the same namespace.
These groups implement `MessageBroker`; they are **not native `XREADGROUP`
consumer groups**. Other clients must not mutate these keys or mix native ACK,
trimming or group operations into this layout.

| Operation | Supported behavior |
| --- | --- |
| `publish` | Exact topic/key replay returns the original ID and publication time; changed payload, headers or content metadata conflict. A server response acknowledges the append and index mutations. |
| `subscribe` | Idempotent group registration with earliest-retained or future-only start. Separate groups receive independent copies. |
| `receive` | Non-blocking bounded claims shared between consumers; authoritative Redis time determines lease expiry. A fresh random capability replaces an expired lease. |
| `ack` | The current unexpired capability is consumed once. An expired or replaced capability cannot settle a newer delivery. |
| `retry` | Delays up to seven days; the next receive claims due messages. Reaching the configured attempt ceiling sends the delivery to the group's DLQ. |
| `dead_letter` / `dead_letters` | Explicit current-lease terminal failure and bounded inspection without ACK secrets. Lease-expiry attempt exhaustion is processed on subsequent operations. |
| `purge_terminal` | Removes at most 100 messages per call, even if a larger limit is requested, and only after every current group is terminal or started after the message. With no groups it removes nothing. It also removes publication deduplication records. |

Server-side scripts serialize mutations, so independently connected consumers
cannot claim the same live lease. Delivery remains **at least once**: an external
effect can succeed before its ACK fails. Deduplicate effects using the stable
message ID or a domain key. Retain deduplication state long enough for the
application's retry/recovery horizon; republishing after terminal purge creates
a new message. Delayed retries and competing consumers do not promise global or
strict FIFO processing order.

## Limits and failure handling

The profile accepts at most **10,000 retained messages**, **128 subscriptions**,
**100 attempts** and **1 MiB per payload**, with the lower `BrokerConfig` limits
also enforced. Canonical retained envelope bytes have a separate **64 MiB**
ceiling. Header/routing overhead counts toward that ceiling. Delivery and DLQ
responses stop at **4 MiB** of encoded envelopes, so a batch can contain fewer
than the requested maximum. These bounds are not a total Redis or process memory
quota: indexes, protocol buffers, concurrent calls and separate namespaces also
consume memory. The server is a trusted authenticated dependency, not an
untrusted RESP parser sandbox.

At most **eight operations per cloned broker family** are admitted concurrently;
excess calls fail immediately. The default total operation deadline is **ten
seconds**; `with_timeout` permits 100 milliseconds–30 seconds. There is no
automatic write retry. A timeout or lost response does not establish whether a
write committed. Replay a publication with the same key/content; let uncertain
delivery claims expire and preserve destination idempotency. A later operation
may establish a fresh connection to the same endpoint and must recheck persisted
namespace state.

Redis scripts provide isolation, not rollback after an execution error. Every
mutation writes a persistent in-progress marker first and clears it only after
all commands succeed. A partial failure leaves the namespace quarantined;
subsequent operations and startup reject it. There is deliberately no public
“clear dirty” repair helper. Recover to a reviewed consistent snapshot or rebuild
in a separately provisioned namespace from authoritative source events, checking
already-completed effects. Keep the original evidence for diagnosis. The marker
cannot detect arbitrary administrator edits or a coherent old backup rollback.

Publication fans out over registered groups. Creating an earliest subscription
indexes retained messages; purge examines current groups. Those operations have
bounded work but can briefly block the dedicated Redis server. Subscription
creation belongs in deployment/configuration work, not per-request handlers.

## Deployment responsibilities

Use a dedicated **standalone Redis 7.4+ database**, protected network and narrowly
scoped ACL identity. The key prefix is
`rullst:messaging:v1:<SHA-256(namespace)>:`; namespace hashing avoids delimiter
collisions but is not authorization or encryption. The SDK needs `EVAL`, `TIME`
and the hash, set, sorted-set and stream commands used by its fixed scripts.
It does not need configuration, ACL-management, `FLUSHDB` or arbitrary key
administration permissions. The application must authorize namespace/topic/group
access before invoking broker operations; never hand the raw Redis credentials
to an untrusted client. Lua source is included in the crate archive for audit.

Configure **AOF persistence**, an appropriate fsync policy and **no eviction**.
The acceptance fixture uses `appendfsync always` and abrupt process restart.
The SDK does not change or certify server configuration. Persistence guarantees
still depend on storage and server operation; acknowledgements are not universal
zero-data-loss guarantees. Monitor disk/memory, server time and failed operations.
Clock regression relative to persisted operation time rejects mutations until
the server's time catches up or a reviewed recovery occurs. Redis contents,
headers, deduplication metadata and leases are plaintext at rest; protect AOF,
volumes and backups separately. The encrypted SQLite profile is not automatically
applied to this remote adapter.

Replication/failover, Sentinel, Cluster, managed-provider certification, native
consumer-group interoperability, automatic trimming/repair, unbounded replay,
multi-region ordering and exactly-once effects are outside this profile.

## Outbox and executable evidence

Add `messaging-orm-outbox` to relay committed relational events through the
existing `OrmOutboxRelay<RedisBroker>`. Commit the domain write and event in the
same ORM transaction. A worker publishes the durable event key, then ACKs its
exact outbox claim. Crashing between these operations produces an exact broker
replay when reclaimed. This still does not create a distributed transaction.

The [disposable service runner](../../.github/check-messaging-redis.py) executes
the common broker contract, competing clients, exact/conflicting replay,
expired/replaced leases, retry/DLQ, retention/reply limits, partial-write
quarantine, missing state, timeout reconciliation, trusted/untrusted TLS,
hostname checks, an actual ORM outbox and an AOF server restart. Test fixtures
use only temporary data, local certificates and public test credentials.
The [facade consumer](../../.github/fixtures/messaging-redis-facade.rs) is also
compiled from extracted package sources. Hosted full-workspace, package and
release checks remain separate admission requirements.
