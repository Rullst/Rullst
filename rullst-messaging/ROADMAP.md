# Rullst Messaging Roadmap

> This roadmap preserves the broker vision without presenting protocol names
> as implemented interoperability. The current public contract is defined by
> `docs/src/spec.md` and tested by `tests/support/mod.rs`.

## Phase 1 — bounded broker contract

- [x] Versioned envelope, validated identifiers, bounded metadata and payload.
- [x] Topic-scoped idempotent publication with exact-replay receipts and
  conflict rejection.
- [x] Consumer groups, fan-out, competing consumers, expiring single-use
  acknowledgement leases, retry, dead-letter, and explicit purge.
- [x] Deterministic in-memory implementation with injectable time.
- [x] Reusable static-dispatch conformance suite and concurrent state tests.
- [ ] Stable remote wire codec, compatibility fixtures, and migration policy.
- [ ] Durable local adapter with crash/restart and corruption-recovery tests.

## Phase 2 — remote adapters

- [ ] NATS Core and JetStream.
- [ ] RabbitMQ/AMQP 0-9-1.
- [ ] Apache Kafka.
- [ ] Redis Streams.
- [ ] AWS SQS and SNS.
- [ ] Google Cloud Pub/Sub.
- [ ] Apache Pulsar.

Every adapter must publish a method-by-method capability matrix. Unsupported
ordering, transactions, delayed delivery, replay, retention, or dead-letter
semantics must fail explicitly rather than being silently approximated.

## Phase 3 — operations and integrations

- [ ] OpenTelemetry propagation with a documented allowlist and secret-safe
  metadata mapping.
- [ ] Studio inspection through real broker telemetry and explicit unavailable
  states.
- [ ] Transactional-outbox relay integration without claiming atomic remote
  effects.
- [ ] Broker-specific live lifecycle matrices, restart/fault evidence, and
  release provenance.
- [ ] Encrypted durable credentials, rotation hooks, and deployment guidance.

Exactly-once external side effects, universal ordering, zero data loss, and
provider availability are not framework guarantees. They require application
idempotency plus broker- and deployment-specific evidence.
