# Durable recurring publications

The unpublished v13 candidate adds Messaging `schedules-postgres` and facade
`messaging-schedules-postgres`. Independent application instances coordinate
through one authoritative PostgreSQL database, then relay frozen occurrences
into the existing `MessageBroker` contract. Core's process-local `Scheduler`
keeps its existing behavior. This feature alone selects neither SQLite nor ORM.

The host owns authenticated tenant membership, schedule authoring, inspection,
cancellation, operator retry and selection of the destination broker. Schedule
names and namespace strings received from a request are not authorization.
Apply the normal authenticated, ownership-checked, CSRF/WAF/header-protected
management endpoints. Payloads must contain only necessary application data.

## Configure and run

Load a 32-byte key from your secret manager and retain previous decryption keys
while any retained record uses them. `initialize` creates the fixed schema and
namespace once during deployment. `connect` requires existing state and never
repairs missing tables or changes namespace quotas. Both take an explicit clock;
use `SystemClock` in production. No database failure falls back to memory.

```rust,no_run
# #[cfg(feature = "messaging-schedules-postgres")]
# async fn schedule<B: rullst::messaging::MessageBroker>(database_url: String,
#     key_bytes: [u8; 32], first_after_ms: i64, payload: Vec<u8>, broker: B)
#     -> Result<(), Box<dyn std::error::Error>> {
use rullst::messaging::{MessagingKeyring, MessagingStorageKey, SystemClock};
use rullst::messaging::schedules::{
    MissedRunPolicy, PostgresRecurringStore, RecurringConfig,
    RecurringDefinition, ScheduledMessage,
};

let keys = MessagingKeyring::new(MessagingStorageKey::try_new("2026-09", key_bytes)?);
let store = PostgresRecurringStore::connect(
    database_url, RecurringConfig::new("school-events", 100, 10_000)?, keys, SystemClock,
).await?;
store.create(RecurringDefinition::new(
    "daily-reminder-v1", "0 9 * * *", first_after_ms,
    MissedRunPolicy::Coalesce,
    ScheduledMessage::new("reminders", "reminder.requested", payload)?,
)?).await?;
store.tick(20).await?;
for lease in store.claim(20).await? {
    store.relay(&lease, &broker).await?;
}
# Ok(())
# }
```

The application supplies the worker loop and its shutdown/backoff policy. Calls
have a 10-second storage deadline; pools have at most four connections, with
5-second acquisition/SQL/lock deadlines. Broker publication is bounded by the
remaining lease and at most 10 seconds. Database and broker work do not share a
transaction. Calling `close` closes that pool and its clones, not other instances.

## Calendar and delivery contract

Definitions are immutable. Identical creation replays the original metadata;
conflicting reuse fails. Cancellation is permanent. Edit by cancelling and
creating a new name. Retained definitions reserve names and count against the
quota, even after cancellation; retiring a namespace is an explicit deployment
operation. A random generation and scheduled UTC instant identify each occurrence.

The five fields use the Rust `cron` crate's calendar semantics: minute, hour,
day-of-month, month, weekday; seconds are zero, years span 1970 through 2100.
Weekdays are **1=Sunday through 7=Saturday**, or names such as `Mon`. Day-of-month
and weekday restrictions intersect. This is not POSIX crontab syntax: weekday
zero and seconds/year fields are rejected. There is no implicit local timezone
or daylight-saving conversion. `first_after_ms` is exclusive. Exhausted calendars
stop producing occurrences.

`CatchUp` preserves every missed due time; a tick emits the oldest due times
first, up to its 1–100 occurrence budget. `Coalesce` emits the oldest outstanding
due time once, then advances past observed current time. Materialization and
advancement commit atomically. Capacity failure rolls back the **whole tick**;
reduce the batch or purge terminal occurrences, then retry. Definitions are
limited to 10,000, occurrences to 100,000 and raw message payloads to 16 KiB.

Every materialized occurrence has a fixed delivery window starting at creation,
including caught-up work. The default is one day, with a seven-day maximum.
Leases default to 60 seconds, configurable from 1–300 seconds; the delivery
window must accommodate two configured leases. A claim fences earlier workers
with a fresh random capability and revision. Automatic publication failures
back off exponentially and stop after ten attempts or delivery-window expiry.
`retry_failed` starts a new explicit operator attempt budget within the **original**
window. It cannot revive published, cancelled, expired or purged occurrences.

A relay checks the live lease immediately before publication. Repeated attempts
use exactly the same message and purpose-separated idempotency key. Broker
acceptance followed by an uncertain acknowledgement returns
`RecurringRelayError::Acknowledgement`, retaining the broker receipt. A broker
error or timeout may also follow remote acceptance; recovery repeats the same
key. An invalidated lease cannot acknowledge/retry the new worker's occurrence.
A cancellation after the final check cannot recall an in-flight publication.

Broker deduplication must remain available for at least the delivery/retry
window. Early broker purging can remove that protection. Consumers must check
current authorization and deduplicate external effects at the destination.
A successful relay proves broker acceptance, not handler completion or exactly
once external delivery. Select the same broker namespace on every retry.

## Storage, roles and retention

Definitions and occurrence content use AES-256-GCM with explicit rotation keys,
bound to namespace, purpose and generation/occurrence identity. Configuration
has an authenticated encrypted binding; key/configuration drift fails closed.
Retain old keys as long as records need them; automatic re-encryption is not
included. Debug output omits content, headers and lease credentials. Metadata
names are server-owned configuration and should not contain personal data.

PostgreSQL must use permanent tables, fsync, full-page writes, synchronous commits
and a writable primary. Remote connections require verified TLS. All operations
serialize through the namespace control row and recheck persisted server time
after lock waits and around commit. The host remains responsible for trusted
clock discipline, database credentials, backup encryption, anti-rollback policy,
failover and capacity monitoring. These checks do not certify a deployment's
backup/failover design.

The runtime role needs `USAGE` on `public` and these table privileges:

| Table | Runtime privileges |
| --- | --- |
| `rullst_recurring_control` | SELECT, UPDATE |
| `rullst_recurring_definitions` | SELECT, INSERT, UPDATE |
| `rullst_recurring_occurrences` | SELECT, INSERT, UPDATE, DELETE |

`initialize` uses a separate deployment role with schema privileges. Ordinary
workers use `connect` without DDL rights. Do not let other application roles
mutate this subsystem's tables directly.

`schedules` and `occurrences` return metadata pages of 1–100 records with exclusive
cursors, without payloads or credentials. `occurrences`, `claim` and terminal
purge classify expired work and erase expired retained content. Publication and
cancellation erase occurrence ciphertext immediately; retryable dead letters
retain it until expiry or explicit purge. `purge_terminal` removes only terminal
occurrences older than the caller's cutoff, up to 100 per call. Run maintenance
regularly: data is not removed while every worker is stopped.

## Acceptance status

Local native contracts cover concurrent initializers/instances, immutable replay,
catch-up/coalescing, real SQL rollback and lock deadlines, cancelled futures,
lease expiry, cross-namespace denial, encryption binding, restricted roles,
non-durable tables, quotas, retry limits, cancellation, fresh processes and actual
PostgreSQL restart. An encrypted SQLite broker journey verifies acceptance before
lost ACK, reopen/replay deduplication and real consumer delivery. No provider
account is used. The same native journey passed through an extracted facade
consumer, with Messaging source bytes matched against its archive. Full hosted
workspace, coverage/security and source admission remain required before
declaring the feature complete.
