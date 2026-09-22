# Shared PostgreSQL mail suppression (v13 candidate)

Mail's optional `postgres` feature exposes `PostgresSuppressionStore`,
`PostgresSuppressionConfig` and `SuppressionKey`; the facade feature is
`mail-postgres`. It implements the existing `SuppressionStore` and
`MutableSuppressionStore` contracts across hosts sharing one authoritative
writable PostgreSQL database. It does not enable SQLite or change default mail
delivery. Full hosted/source/package admission for this increment is pending.

## Configure and compose the store

Choose a server-owned namespace and immutable independent quotas of 1–1,000,000
recipient records and replay identities. Use a separate random 32-byte HMAC key
from the deployment secret store. `SuppressionKey::new` rejects trivially weak
inputs; this check cannot make a human-chosen key cryptographically random.
Namespace/key selection must come from trusted application configuration, never
from arbitrary request data. Use opaque namespaces rather than email addresses
or other personal identifiers.

Run `PostgresSuppressionStore::initialize(database_url, key, config)` explicitly
as a deployment task. Applications use `connect` with the same configuration;
missing namespaces, changed keys and quota drift fail closed. Runtime does not
create tables or reconstruct deleted state. Protect database credentials and
disable SQL statement/parameter logging outside the adapter as well.

Wrap the configured mail transport in `SuppressionGuard::new(driver, store)`.
For multiple tenants, register one correctly scoped guard per authenticated
tenant in `TenantMailResolver`; avoid a fallback that silently selects another
tenant's store. Install the composed driver through `Mail::set_driver` in every
web process and queue worker. Apply the guard to every physical provider attempt
when composing failover/retry transports, so a retry does not reuse an earlier
suppression decision.

The guard checks authoritative SQL state immediately before `send`,
`send_for_tenant` and `send_with_delivery_id`. The normal Mail pipeline still
validates messages. `Mail::enqueue_for_tenant` preserves the trusted tenant
context; `register_mail_handler` resolves the current driver when the worker
dispatches. Feedback received after enqueue can therefore block delivery.
Plain drivers are not silently wrapped; applications must install this policy.

## Verified events and minimized storage

Authenticate provider webhook bodies/signatures and freshness before producing
a `SuppressionEvent`. The existing Resend verifier can turn verified permanent
bounces/complaints into such events. Other providers require their corresponding
verified adapter; this store does not authenticate arbitrary JSON or establish
consent. Manual suppression requires authorized application/operator policy.
Mount ingestion behind the appropriate signature verifier, bounded requests,
secure headers, WAF and ingress limits.

`record` atomically binds provider/event identity to its recipient, reason and
observation time. Identical replay is idempotent; conflicting reuse is rejected
without changing state. Reason precedence only increases: manual suppression,
hard bounce, then spam complaint. Earlier events can strengthen a reason but
cannot undo a complaint. Recipient normalization preserves the local part and
lowercases the domain, matching the existing stores; it does not infer aliases.

The database stores HMAC-derived recipient/event identifiers and fingerprints,
the authoritative bounded provider/reason and first/last observation times.
It does not persist raw addresses, provider event IDs, message bodies or delivery
history. Callers still handle addresses in memory, and keyed identifiers remain
pseudonymous data, not anonymous data. Keep keys, backups and database access
protected. Debug/errors omit addresses, event identities and keys.

`lookup` returns the current record for the supplied normalized address.
`snapshot` exposes counts and quotas only. `prune_events_before` deletes old
replay evidence and retains every recipient suppression. Choose a positive
cutoff no later than now and older than all provider redelivery/reconciliation
windows: pruned identities may be accepted again. This API does not automatically
reenable delivery, remove complaints or implement a privacy-deletion workflow.
When capacity is exhausted, ingesting a new event fails explicitly. Investigate
and retain/reconcile verified feedback instead of acknowledging a failed write
as successfully persisted.

## Database and failure boundaries

The private pool has at most four connections, verified TLS for effective remote
hosts, five-second SQL/lock deadlines and ten-second operation deadlines. Local
loopback/socket fixtures can use local transport. Every operation checks that
the three fixed `public.rullst_mail_pg_suppression_*` tables are permanent, the
database is writable and `fsync`, `full_page_writes` and synchronous commits are
enabled. Runtime needs:

- USAGE on the trusted `public` schema and SELECT/UPDATE on the control table.
- SELECT/INSERT/UPDATE on recipient state.
- SELECT/INSERT/DELETE on replay events.

Keep CREATE/ALTER and recipient deletion with the deployment/operator role;
revoke schema CREATE from untrusted roles. Namespace row locks serialize lookups,
ingestion and retention. Quotas, clock observations and both event/recipient
writes commit together. Cancellation before commit rolls back; an uncertain
commit requires reconciliation. Storage/configuration failures make the guard
return `SuppressionUnavailable` without invoking its transport.

The host owns clock synchronization, database availability, encryption at rest,
replication fencing, a bounded number of namespaces and capacity planning. A
stale database restore can lose newer complaints or replay evidence; quiesce
delivery and reconcile authoritative suppression before resuming. Configuration
changes need a deliberate migration. The store does not implement backup
anti-rollback or automatic failover.

A suppression committed after the guard's database check cannot recall mail
already handed to a provider. Distributed provider delivery is not atomic with
this SQL transaction, and retries can still deliver duplicates. These controls
do not prove inbox acceptance, provider account readiness or legal compliance.

## Automated evidence

The candidate's owned PostgreSQL suite exercises independent initializers/pools,
replay and conflict races, reason precedence, recipient/event quotas, SQL failure
rollback, cancellation, lock deadlines, clock/configuration drift, restricted
runtime privileges, non-durable storage rejection and close/reopen. Fresh
processes and an actual database restart verify persistent suppression and replay.
The real Mail worker/tenant resolver/guard composition uses a bounded queue
fixture and isolated memory transports to prove feedback after enqueue blocks
the selected tenant without cross-delivery. No external inbox is contacted.

Run `python3 .github/check-mail-postgres.py` for the disposable loopback Docker
database. The package campaign repeats this contract through extracted
facade/Mail/Core archives; full workspace, coverage, security and release gates
remain separate.
