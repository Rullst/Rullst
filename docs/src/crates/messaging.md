# rullst-messaging

`rullst-messaging` is the broker-neutral event boundary for Rullst. It is kept
separate from `rullst-connect` because OAuth/OIDC identity federation and
message-broker delivery have different security, availability, and retry
semantics.

## Current status

The crate has an **implemented, bounded process-local foundation**:

- a versioned immutable envelope;
- bounded identifiers, headers, payloads, batches, leases, and retention;
- topic-scoped idempotent publication;
- consumer-group fan-out and competing-consumer claims;
- expiring single-use acknowledgement tokens;
- bounded retry, dead-letter views, and explicit purge;
- deterministic time injection and reusable contract tests.

The `InMemoryBroker` is suitable for offline tests, deterministic development,
and explicitly process-local workloads. It is not durable and is not a remote
transport. Kafka, RabbitMQ, Redis Streams, NATS/JetStream, SQS/SNS, Google
Pub/Sub, and Pulsar adapters remain roadmap work.

## Security and correctness boundary

Payloads, idempotency values, acknowledgement tokens, and header values are not
included in debug output. Tracing emits only bounded routing and decision
metadata. Applications must still authorize who may publish or consume each
topic and must make external effects idempotent.

Delivery is at least once. A valid acknowledgement consumes its lease exactly
once, but no local ACK can make an arbitrary remote side effect atomic. Use the
stable envelope ID at the side-effect boundary.

Continue with the [brokered messaging tutorial](../tutorials/49-brokered-messaging.md)
or inspect the [crate roadmap](https://github.com/Rullst/Rullst/blob/main/rullst-messaging/ROADMAP.md).
