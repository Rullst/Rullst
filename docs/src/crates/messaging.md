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
- an explicit SQLite AES-256-GCM profile for header values and payloads, with
  immutable profile selection and bounded primary/prior-key rotation.
- an opt-in static relational ORM outbox relay with exact stream/topic binding
  and publish-before-ACK crash/replay evidence.

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

`SqliteBroker::connect` keeps the compatible plaintext profile.
`connect_encrypted` uses randomized AES-256-GCM and authenticates immutable row
metadata. Raw-storage, restart, wrong-key, tamper, row-swap, rotation, symlink
and two-instance regressions are executable. It protects header values and
payloads, not routing/idempotency/delivery metadata or the complete database.
The deployment still owns keys, database-file permissions, protected backups,
rollback detection, retention, disk monitoring and topic/tenant authorization.
Reopening a namespace with different limits or a different storage profile
fails closed instead of silently changing retained semantics.

The outbox relay does not make ORM and broker state one atomic transaction. It
uses the committed event key for exact broker replay, publishes first and ACKs
the exact ORM lease second. The application still supervises workers, retries,
dead letters, cleanup, tenant/topic authorization and destination idempotency.

Continue with the [brokered messaging tutorial](../tutorials/49-brokered-messaging.md)
or inspect the [crate roadmap](https://github.com/Rullst/Rullst/blob/main/rullst-messaging/ROADMAP.md).
