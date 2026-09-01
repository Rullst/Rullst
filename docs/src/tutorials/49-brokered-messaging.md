# 49. Bounded Brokered Messaging

> [!IMPORTANT]
> Dependency examples use `12.0.0-rc.1`, the planned first v12 RC. Do not
> request it from crates.io before it is published; use path dependencies from
> this source checkout during development.

This tutorial starts with `rullst-messaging`'s deterministic in-memory broker
and then switches the same trait contract to the durable local SQLite adapter.
Remote broker protocols remain separate adapters.

## 1. Enable the umbrella feature

```toml
[dependencies]
rullst = { version = "12.0.0-rc.1", features = ["messaging"] }
```

Use `messaging-sqlite` instead of `messaging` for durable local state. A direct
dependency uses `rullst-messaging = { version = "12.0.0-rc.1", features =
["sqlite"] }`.

## 2. Create a bounded broker

```rust
use rullst::messaging::{BrokerConfig, InMemoryBroker};

# fn create_broker() -> Result<(), rullst::messaging::MessagingError> {
let config = BrokerConfig::try_new("billing")?
    .with_limits(
        10_000,        // retained messages
        128,           // consumer-group subscriptions
        5,             // delivery attempts
        1024 * 1024,   // payload bytes
    )?;
let broker = InMemoryBroker::new(config);
# let _ = broker;
# Ok(())
# }
```

The hard ceilings prevent an accidental configuration from turning the local
broker into unbounded memory state. Capacity exhaustion fails closed until
terminal messages are explicitly purged.

### Persist the same contract locally

```rust,no_run
use rullst::messaging::{BrokerConfig, SqliteBroker};

# async fn open() -> Result<(), rullst::messaging::MessagingError> {
let config = BrokerConfig::try_new("billing")?
    .with_limits(10_000, 128, 5, 1024 * 1024)?;
let broker = SqliteBroker::connect("sqlite://storage/messages.sqlite", config).await?;
# let _ = broker;
# Ok(())
# }
```

All mutations use serialized SQLite write transactions. A committed publish or
ACK survives restart, while an uncommitted operation rolls back. In-flight
leases also survive; after expiry the next broker operation requeues or
dead-letters them. Multiple processes may share one file and namespace, but
reopening it with different limits is rejected. This is local durability, not
network replication, automatic failover or exactly-once side effects.

## 3. Register a consumer group and publish

```rust
use rullst::messaging::{
    MessageBroker, PublishRequest, StartPosition, SubscriptionRequest,
};

# async fn example(
#     broker: &rullst::messaging::InMemoryBroker,
# ) -> Result<(), rullst::messaging::MessagingError> {
broker
    .subscribe(SubscriptionRequest::try_new(
        "invoices",
        "receipt-mailers",
        StartPosition::Earliest,
    )?)
    .await?;

let request = PublishRequest::try_new(
    "invoices",
    "invoice.paid",
    "invoice/2026-0042/paid/v1",
    br#"{"invoice_id":"2026-0042"}"#.to_vec(),
)?
.with_content_type("application/json")?
.with_header("trace-id", "01-example")?;

let first = broker.publish(request.clone()).await?;
let replay = broker.publish(request).await?;
assert_eq!(first.id(), replay.id());
assert!(replay.is_duplicate());
# Ok(())
# }
```

Idempotency is scoped to the topic. The same topic/key with different content
returns `MessagingError::IdempotencyConflict`; it never silently overwrites the
original publication.

## 4. Receive, acknowledge, retry, or dead-letter

```rust
use rullst::messaging::{
    FailureCode, MessageBroker, ReceiveRequest, RetryDisposition,
};
use std::time::Duration;

# async fn consume(
#     broker: &rullst::messaging::InMemoryBroker,
# ) -> Result<(), rullst::messaging::MessagingError> {
let request = ReceiveRequest::try_new(
    "invoices",
    "receipt-mailers",
    "worker-1",
    10,
    Duration::from_secs(30),
)?;

for delivery in broker.receive(request).await? {
    let effect_succeeded = true;
    if effect_succeeded {
        broker.ack(delivery.ack_token()).await?;
    } else {
        let disposition = broker
            .retry(
                delivery.ack_token(),
                Duration::from_secs(15),
                FailureCode::try_new("mail.transient")?,
            )
            .await?;
        if disposition == RetryDisposition::DeadLettered {
            // Alert through an application-owned operational channel.
        }
    }
}
# Ok(())
# }
```

An ACK token is an opaque, single-use capability bound to one group and one
lease. Once it expires, the message is requeued or dead-lettered at the attempt
ceiling. Do not serialize tokens into logs or application records.

## 5. Design for at-least-once delivery

The consumer may complete the mail/payment/webhook effect and stop before the
ACK reaches the broker. Therefore:

1. use `delivery.envelope().id()` as a stable deduplication key;
2. claim that key transactionally in the side-effect system where possible;
3. ACK only after the effect is durably accepted;
4. treat retries and dead letters as normal operational states;
5. authorize topics and tenant scope in the host application.

The in-memory broker loses state on process exit. The SQLite adapter retains
state but stores payloads and headers in plaintext; the host owns file
permissions, encryption at rest, backup/restore, retention, disk monitoring and
topic/tenant authorization. A future remote adapter is supported only after it
passes the shared contract plus its own protocol, restart, and fault matrix; an
adapter name alone is not durability evidence.
