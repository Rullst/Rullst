# Rullst Messaging

> **v12 development notice:** This README documents the unreleased v12 source.
> Use a path dependency from this checkout until an immutable v12 RC exists on
> crates.io.

`rullst-messaging` defines bounded, broker-neutral messaging contracts for
Rullst applications. It provides a deterministic in-memory broker and an
opt-in durable SQLite adapter. Remote broker interoperability is roadmap work.

## Implemented boundary

- versioned `rullst.messaging.v1` envelopes;
- bounded topics, consumer groups, metadata, payloads, batches, leases, and
  retry delays;
- topic-scoped idempotent publishing with conflict detection;
- at-least-once consumer-group delivery and fan-out between groups;
- expiring, single-use acknowledgement leases;
- bounded retry, automatic/manual dead-lettering, and explicit terminal purge;
- injectable clock and a reusable static-dispatch contract suite;
- redacted debug output and low-cardinality tracing;
- a canonical bounded v1 envelope wire codec with a fixed compatibility
  fixture, strict version rejection, and fail-closed decoding;
- validated W3C `traceparent` plus a conservative `tracestate` subset, carried
  only through those two allowlisted headers without arbitrary baggage;
- opt-in SQLite durability for publications, group cursors, leases, retries,
  dead letters, acknowledgements, idempotency and explicit purge.

The in-memory driver is process-local and not durable. It does not claim
Kafka, RabbitMQ, Redis Streams, NATS/JetStream, SQS/SNS, Google Pub/Sub, or
Pulsar protocol compatibility.

`WireEnvelopeCodec` is an envelope interoperability primitive, not a remote
transport. It does not establish a connection, persist the caller's publish
idempotency key, map broker-specific acknowledgements, or certify any named
broker. Applications also own trace sampling, export, retention and
tenant-aware correlation policy.

## Durable local profile

Enable `features = ["sqlite"]` on `rullst-messaging`, or
`features = ["messaging-sqlite"]` on the umbrella `rullst` crate:

```rust,no_run
use rullst_messaging::{BrokerConfig, SqliteBroker};

# async fn durable() -> Result<(), rullst_messaging::MessagingError> {
let config = BrokerConfig::try_new("checkout")?;
let broker = SqliteBroker::connect("sqlite://storage/messages.sqlite", config).await?;
# let _ = broker;
# Ok(())
# }
```

The adapter uses a fixed schema and `BEGIN IMMEDIATE` for mutations, so separate
instances sharing a file serialize publication and claims. Reopening the same
namespace requires the exact persisted limits. SQLite commits survive process
restart; expired leases are requeued or dead-lettered on the next operation.
Malformed persisted envelopes fail closed. `connect` selects an immutable
plaintext-v1 profile. For protected message contents, start a namespace with
the explicit AES-256-GCM profile:

```rust,no_run
use rullst_messaging::{
    BrokerConfig, MessagingKeyring, MessagingStorageKey, SqliteBroker,
};

# async fn encrypted(key_bytes: [u8; 32]) -> Result<(), rullst_messaging::MessagingError> {
let primary = MessagingStorageKey::try_new("messages-2026-09", key_bytes)?;
let keyring = MessagingKeyring::new(primary);
let broker = SqliteBroker::connect_encrypted(
    "sqlite://storage/messages.sqlite",
    BrokerConfig::try_new("checkout")?,
    keyring,
).await?;
# let _ = broker;
# Ok(())
# }
```

This profile encrypts and authenticates header values plus payload bytes with a
fresh AES-256-GCM nonce. AAD binds namespace, topic, sequence, message ID,
event/content type, publication timestamp and rotation key ID. Topic, message
and event metadata, timestamps, idempotency keys, fingerprints, key IDs and
delivery state remain visible. It is content encryption, not full-database or
metadata encryption.

The first key encrypts new records; add at most seven prior decryption keys with
`with_decryption_key`. Startup rejects a missing/wrong key, tampered profile or
removal of a prior key while any retained record still references it. Rotation
does not rewrite old messages: ACK and purge them under the complete keyring
before retiring that key. Plaintext and encrypted profiles never mix silently;
migration requires an explicit new namespace/database and application-owned
republishing. Key custody, file permissions, protected backups, rollback
detection, retention, disk operations and tenant/topic authorization remain
deployment/application duties.

## Transactional outbox relay

Enable `orm-outbox` directly, or `messaging-orm-outbox` on `rullst`, to bridge
the existing relational `rullst-orm::Outbox` to any concrete `MessageBroker`.
The application first commits its domain mutation and outbox row in one ORM
transaction. A supervised worker claims that row and passes it to an
`OrmOutboxRelay` bound to one exact stream and broker topic.

`publish_claim` validates the claim and publishes its JSON using the outbox
`event_key` as the broker idempotency key. `relay_and_ack` then acknowledges the
exact ORM lease. These are necessarily two operations: if the process stops
after publication, the lease expires and a later claim republishes the same
content. The broker returns the original message as an exact replay, after
which the new claim can be acknowledged. The executable integration test
proves this crash window produces one broker message.

The relay does not turn a relational and remote system into one atomic
transaction. Worker supervision, retry/dead-letter policy, outbox cleanup,
topic/tenant authorization and destination-side idempotency remain application
responsibilities. Outbox payload, event/idempotency keys and claim tokens are
redacted from claim/relay diagnostics.

## Example

```rust
use rullst_messaging::{
    BrokerConfig, InMemoryBroker, MessageBroker, PublishRequest, ReceiveRequest,
    StartPosition, SubscriptionRequest,
};
use std::time::Duration;

# async fn example() -> Result<(), rullst_messaging::MessagingError> {
let broker = InMemoryBroker::new(BrokerConfig::try_new("checkout")?);
broker
    .subscribe(SubscriptionRequest::try_new(
        "orders",
        "email-workers",
        StartPosition::Earliest,
    )?)
    .await?;

let receipt = broker
    .publish(PublishRequest::try_new(
        "orders",
        "order.created",
        "order/42/v1",
        br#"{"order_id":42}"#.to_vec(),
    )?)
    .await?;

let deliveries = broker
    .receive(ReceiveRequest::try_new(
        "orders",
        "email-workers",
        "worker-1",
        10,
        Duration::from_secs(30),
    )?)
    .await?;
for delivery in deliveries {
    // Make the external effect idempotent with delivery.envelope().id().
    broker.ack(delivery.ack_token()).await?;
}

assert!(!receipt.is_duplicate());
# Ok(())
# }
```

## Delivery semantics

Delivery is **at least once**, not exactly once. A consumer can complete an
external side effect and stop before acknowledging its lease. The application
must deduplicate side effects using the stable envelope ID or an application
key at the destination boundary.

An exact publish replay returns the original message ID. Reusing the same
topic/idempotency key with different content fails closed. Explicit terminal
purge removes both the retained message and its idempotency
record, so retention policy must be chosen deliberately.

See the [crate roadmap](ROADMAP.md) and the
[brokered messaging tutorial](https://github.com/Rullst/Rullst/blob/main/docs/src/tutorials/49-brokered-messaging.md).
