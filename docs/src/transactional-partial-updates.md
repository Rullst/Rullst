# Transactional partial updates

The unpublished v13 candidate makes a SQLx model’s typed partial change participate in the
model's normal save lifecycle. A lesson editor can submit a new title while
preserving a note that another operation has already changed, applying its
policy, recording the audit and refreshing projections after commit.

Local SQLite, PostgreSQL and MySQL journeys, cancellation, Redis cache, Scout,
MSRV 1.96, strict linting and an extracted-facade consumer passed. Full hosted
workspace, platform, archive and release admission remain pending.

## Application use

The consumer needs its selected `rullst` ORM/database features and direct
`sqlx`, `tokio` and `tracing` dependencies, which the generated macros reference.
The [extracted-package consumer](../../.github/test-packaged-distribution.sh)
records a compiled dependency profile; this candidate is not yet a published
crates.io installation.

```rust,no_run
# #[cfg(feature = "orm")]
# mod example {
use rullst::orm as rullst_orm;
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "lessons", tenant_column = "tenant_id", auditable)]
pub struct Lesson {
    pub id: i32,
    pub tenant_id: String,
    pub title: String,
    pub note: Option<String>,
}

// The host first authenticates the editor and authorizes this lesson.
// Supply tenant/actor from trusted identity, not request-body assertions.
pub async fn rename(
    lesson: &mut Lesson,
    trusted_tenant: String,
    trusted_editor: String,
    title: String,
) -> Result<(), rullst_orm::Error> {
    let actor = rullst_orm::audit::AuditContext::user(trusted_editor)?;
    rullst_orm::tenant::with_tenant(trusted_tenant,
        rullst_orm::audit::with_audit_context(actor,
            lesson.update_partial().title(title).save(),
        ),
    ).await
}
pub async fn rename_in_transaction(
    lesson: &mut Lesson,
    tx: &mut rullst_orm::db::Transaction<'_>,
    title: String,
) -> Result<(), rullst_orm::Error> {
    lesson.update_partial().title(title).save_with_tx(tx).await
}
# }
```

Create/migrate the application and audit tables before serving requests. An
update on an auditable model now requires the same `AuditContext` as a full save.
Tenant scoping does not establish ownership or grant an editor permission.
Existing application routes keep their authentication, CSRF, security headers
and ownership checks.

`update_partial()` excludes the primary key and declared tenant column from its
setters. `note(None)` explicitly clears a nullable field; omitting the setter
preserves that field from the persisted row. An empty builder remains a no-op,
without reading the database, running hooks or creating an audit entry.

## Database and lifecycle behavior

A nonempty patch opens a savepoint, validates the model handle's active tenant,
and reads the current row by bound primary key and tenant. Missing rows fail
with `RecordNotFound`. PostgreSQL and MySQL use `SELECT ... FOR UPDATE` so a
competing update cannot change that row between the read and write. SQLite
retains its transaction semantics; a contended read-to-write upgrade can fail
with a database error. There is no automatic retry.

The row must decode through the selected SQLx profile before the patch can
write. This differs from a bare selected-column update: incompatible database
types now fail before mutation. For example, SQLx Any 0.9 rejects SQLite
`BOOLEAN` column metadata; select and validate a typed backend profile for
backend-specific types instead of assuming every Any schema can be read.

Selected values are merged into the fresh model. The normal generated save then
runs its policy, before/after hooks, observers, encryption, audit and post-commit
registrations. A policy, hook, SQL or audit failure rolls the operation's
savepoint back and discards its queued effects. Reentrant implicit ORM reads
inside borrowed mutation callbacks fail validation; load required policy data
explicitly before the mutation, as for full saves.

The successful direct path commits its own transaction, replaces the caller's
object with the fresh merged model, and runs committed observers, cache
invalidation and Scout projections. A `PostCommit` error means that SQL has
already committed; it does not authorize replaying the domain mutation. These
callbacks are process-local. Use an explicit transactional outbox for effects
that need durable recovery.

This implementation performs a full-row SQL save after merging the logical
patch. It is not the v12 selected-column SQL optimization. The extra read and
savepoint have a cost; row-read privileges, full-row triggers/update privileges and encrypted-field
re-encryption follow full-save behavior. Model hooks may transform the candidate
under their usual contract. Review hooks that assumed partial updates skipped
them, and database triggers that react to the SQL column list.

## Caller-owned transactions

The new `save_with_tx(&mut transaction)` terminal method supports an explicit
borrowed transaction. Ordinary `.save()` also reuses the active
`Orm::transaction` task scope. Neither commits the enclosing transaction.

The `rename_in_transaction` example above uses that explicit terminal method.


A successful scoped update replaces the object with the transaction's tentative
row. If a later operation rolls that transaction back, discard or reload the
object; the ORM cannot retroactively mutate an application reference after its
borrow has ended. Catching a failed patch and continuing the managed transaction
does not promote that failed savepoint's audit or effects.

Strict post-commit timing requires the direct owned path or `Orm::transaction`.
A raw SQLx transaction cannot report its eventual commit decision to these
callbacks, so it retains the existing raw-transaction limitation. Do not treat
savepoint release as proof that a caller-owned transaction is durable.

## Migration and evidence

For existing `update_partial()` call sites, review:

- Required audit identity, tenant context and the model's newly applied hooks.
- Database full-row update triggers/privileges and the extra read/savepoint.
- Unsaved fields and loaded relations in the old object: successful updates
  refresh them from the database representation, rather than retaining stale
  local values. Only explicit setters submit field values.
- Post-commit errors, enclosing rollback and ambiguous commit responses. A
  database commit error can require reconciliation; do not assume every error
  proves that no write occurred.

The row lock merges current values; it does not reject a stale editor's expected
revision. Conflicting edits to the same field still need an application revision
or compare-and-set rule. This is also not a bulk update, multi-shard transaction,
background repair service or automatic schema migration.

The [shared database journey](../../rullst-orm/tests/partial_update_contract/mod.rs)
exercises rejected policy/hooks/observers, constraints, missing audit context,
tenant isolation, fresh-row merging, nullable fields, concurrent patches,
explicit/task-scoped rollback, cancellation after SQL, audit deltas and post-commit failures. The same
journey is part of the existing SQLite, PostgreSQL and MySQL matrices.

The separate Redis cache and Scout tests verify that committed patches refresh
projections while rolled-back patches leave them intact. SQLite contention tests
verify known rollback state before an explicit fresh retry. Hosted admission is
still separate from these local checks.
