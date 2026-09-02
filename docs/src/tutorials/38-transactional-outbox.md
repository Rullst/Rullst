# 38. Transactional Outbox & Durable Effects

Use a transactional outbox when a committed database change must eventually
produce an external effect even if the application process stops between the
two operations. Typical examples are a webhook, an e-mail request, a search
projection or a message published to another service.

`after_commit` is the simpler choice for a best-effort, process-local effect.
`Outbox` is the durable choice. It stores an event in the same relational
transaction as the domain mutation, then lets an independent worker claim and
deliver it later.

## 1. Register the schema as a migration

Applications using the built-in SQLx migration runner can register the
versioned migration supplied by the ORM:

```rust,no_run
use rullst_orm::OutboxMigration;
use rullst_orm::schema::migration::Migration;

fn migrations() -> Vec<Box<dyn Migration>> {
    vec![Box::new(OutboxMigration)]
}
```

`Outbox::install()` applies the same idempotent DDL directly and is useful in
tests or explicit local setup. It never runs automatically. A production team
should review and track the migration through the same deployment process as
its domain schema; do not create tables opportunistically while serving a
request.

The built-in contract supports the SQLx relational backends: SQLite,
PostgreSQL, MySQL and MariaDB. It does not span MongoDB, DuckDB, Turso or
SurrealDB transactions.

## 2. Commit domain state and event together

Choose a stable `stream` boundary and a deterministic event key. A stream can
represent an application or tenant, but it is only a namespace: the framework
does not infer authorization from it.

```rust,no_run
use rullst_orm::{Error, Orm, Outbox};
use serde_json::json;

# async fn create_invoice() -> Result<(), Error> {
Orm::transaction(|_| Box::pin(async move {
    let insert = sqlx::query(
        "INSERT INTO invoices (id, status) VALUES (?, ?)"
    )
    .bind(123_i64)
    .bind("issued");
    rullst_orm::execute_query!(insert, execute, pool)?;

    Outbox::enqueue(
        "tenant-42",
        "invoice:123:issued:v1",
        "invoice.issued",
        &json!({"invoice_id": 123}),
    ).await?;

    Ok::<(), Error>(())
})).await?;
# Ok(())
# }
```

If the transaction rolls back, neither row survives. Calling `enqueue` outside
`Orm::transaction` fails instead of silently opening an unrelated transaction.
When the application already owns a raw SQLx transaction, use
`enqueue_with_tx(&mut transaction, ...)`.

`(stream, event_key)` is unique. Repeating the same kind and serialized JSON
payload returns the existing event ID with `inserted == false`. Reusing that
key for different content is an error; it does not overwrite the original
event.

## 3. Claim, deliver and acknowledge

Each worker supplies a stable identifier, lease duration and maximum number of
claims:

```rust,no_run
use rullst_orm::{Error, Outbox};

# async fn deliver() -> Result<(), Error> {
if let Some(event) = Outbox::claim_next(
    "tenant-42",
    "webhook-worker-1",
    30, // lease seconds
    8,  // maximum claims
).await? {
    let payload = event.payload()?;

    // Perform the external effect. Pass event.event_key to a provider that
    // supports idempotency, or deduplicate it in the receiving service.
    let delivered = payload.get("invoice_id").is_some();

    if delivered {
        let owned = Outbox::acknowledge(event.id, event.claim_key).await?;
        if !owned {
            // The lease expired or another worker reclaimed the event.
            // Do not assume ownership or mutate the newer claim.
        }
    } else {
        Outbox::fail(
            event.id,
            event.claim_key,
            "provider temporarily unavailable",
            8,
            15,
        ).await?;
    }
}
# Ok(())
# }
```

An ACK or failure transition succeeds only for the exact, unexpired random
claim token. A failed delivery becomes pending after the bounded delay, or
`dead_letter` when its attempt limit is reached. If a worker dies during its
final claim, the next claim sweep moves that expired event to dead-letter
instead of retrying forever.

## 4. Understand the guarantee

The delivery guarantee is **at least once**:

1. the worker claims an event;
2. the external provider accepts the effect;
3. the process stops before the database ACK;
4. the lease expires and the event is delivered again.

No local database design can atomically commit an arbitrary remote HTTP side
effect. Make the consumer idempotent using the stable stream/event key. Do not
use a random key on every retry.

Other explicit limits:

- key fields are 1–128 characters from a bounded ASCII grammar;
- JSON payloads are at most one MiB;
- leases are 1–3,600 seconds and attempts are 1–100;
- ordering is not guaranteed across retries or concurrent workers;
- generated observers are not automatically converted into outbox events;
- cleanup, retention, tenant authorization and the worker supervision loop
  belong to the application;
- monitor retry and dead-letter state through your normal database operations.

This separation keeps model saves predictable while giving applications a
durable primitive where the event schema and operational policy are explicit.

## 5. Relay into `rullst-messaging`

With the umbrella `messaging-orm-outbox` feature, `OrmOutboxRelay<B>` maps one
exact outbox stream to one topic on any static `MessageBroker`. It validates the
claimed JSON, uses `event_key` as the broker idempotency key, publishes and then
acknowledges the exact ORM claim. Its executable crash-window test stops after
the first publish, reclaims the expired event and observes an exact broker
replay with only one retained message.

That bridge improves composition; it does not change the guarantee. ORM commit,
broker publish and ORM ACK are not one distributed atomic transaction. Operate
the claim loop, retry/dead-letter policy, cleanup, authorization and final
consumer deduplication explicitly. See
[Bounded Brokered Messaging](49-brokered-messaging.md#6-relay-a-relational-outbox-after-commit)
for the relay code.
