# Rullst Messaging

`rullst-messaging` defines bounded, broker-neutral messaging contracts for
Rullst applications. Its first implementation is a deterministic in-memory
broker for development, tests, and process-local workloads. Remote broker
interoperability is roadmap work.

## Implemented boundary

- versioned `rullst.messaging.v1` envelopes;
- bounded topics, consumer groups, metadata, payloads, batches, leases, and
  retry delays;
- topic-scoped idempotent publishing with conflict detection;
- at-least-once consumer-group delivery and fan-out between groups;
- expiring, single-use acknowledgement leases;
- bounded retry, automatic/manual dead-lettering, and explicit terminal purge;
- injectable clock and a reusable static-dispatch contract suite;
- redacted debug output and low-cardinality tracing.

The in-memory driver is process-local and not durable. It does not claim
Kafka, RabbitMQ, Redis Streams, NATS/JetStream, SQS/SNS, Google Pub/Sub, or
Pulsar protocol compatibility.

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
purge removes both the retained message and its process-local idempotency
record, so retention policy must be chosen deliberately.

See the [crate roadmap](ROADMAP.md) and the
[brokered messaging tutorial](https://github.com/Rullst/Rullst/blob/main/docs/src/tutorials/49-brokered-messaging.md).
