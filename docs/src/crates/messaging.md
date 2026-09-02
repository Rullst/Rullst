# rullst-messaging

`rullst-messaging` is the broker-neutral event boundary for Rullst. It is kept
separate from `rullst-connect` because OAuth/OIDC identity federation and
message-broker delivery have different security, availability, and retry
semantics.

## Current status

The crate has an **implemented, bounded broker foundation**:

- a versioned immutable envelope;
- bounded identifiers, headers, payloads, batches, leases, and retention;
- topic-scoped idempotent publication;
- consumer-group fan-out and competing-consumer claims;
- expiring single-use acknowledgement tokens;
- bounded retry, dead-letter views, and explicit purge;
- deterministic time injection and reusable contract tests;
- a canonical bounded v1 envelope wire codec with a deterministic byte fixture;
- allowlisted W3C `traceparent`/`tracestate` propagation without baggage;
- a feature-gated SQLite adapter that transactionally retains publications,
  subscriptions, claims, ACK/retry/DLQ state and idempotency across restart.

The `InMemoryBroker` is suitable for offline tests, deterministic development,
and explicitly process-local workloads. `SqliteBroker` uses a fixed schema and
serialized SQLite write transactions; restart, two-instance contention,
configuration drift and corrupt-row repair are tested. It is a durable local
adapter, not a remote transport. Kafka, RabbitMQ, Redis Streams,
NATS/JetStream, SQS/SNS, Google Pub/Sub, and Pulsar adapters remain roadmap
work.

The wire codec is not a remote adapter: it neither opens broker connections nor
maps a provider's publish/ACK/retention semantics. Trace sampling, exporting,
retention and tenant-aware correlation also remain host policy.

## Security and correctness boundary

Payloads, idempotency values, acknowledgement tokens, and header values are not
included in debug output. Tracing emits only bounded routing and decision
metadata. Applications must still authorize who may publish or consume each
topic and must make external effects idempotent.

Delivery is at least once. A valid acknowledgement consumes its lease exactly
once, but no local ACK can make an arbitrary remote side effect atomic. Use the
stable envelope ID at the side-effect boundary.

SQLite payloads and headers are plaintext. The deployment owns database-file
permissions, encryption at rest, backups, retention, disk monitoring and topic/
tenant authorization. Reopening a namespace with different limits fails closed
instead of silently changing retained delivery semantics.

Continue with the [brokered messaging tutorial](../tutorials/49-brokered-messaging.md)
or inspect the [crate roadmap](https://github.com/Rullst/Rullst/blob/main/rullst-messaging/ROADMAP.md).
