# Auditable Revisions

Rullst SQLx models can bind each instance mutation to a validated principal and
restore eligible update revisions as a new compensating mutation. The audit
row and model write share a transaction; an audit failure rejects the write.

This contract is opt-in and does not infer identity from request data. Install
the principal and tenant contexts only after your authentication and
authorization boundary has validated them.

## 1. Mark the model and install the table

```rust,no_run
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "projects", auditable, tenant_column = "account_id")]
struct Project {
    id: i32,
    account_id: String,
    name: String,
    #[orm(masked)]
    api_token: String,
}

# async fn install() -> Result<(), rullst_orm::Error> {
rullst_orm::audit::create_audit_table().await?;
# Ok(())
# }
```

`create_audit_table` also adds the v2 columns to the legacy audit table. Legacy
records retain version 1 and cannot be restored.

Fields whose names contain password, token, secret, API key, credential, cookie
or similar markers must use `#[orm(masked)]` on auditable models. Their values
are never retained in audit payloads or reverse patches.

## 2. Bind a mutation to trusted context

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
# #[orm(table = "projects", auditable, tenant_column = "account_id")]
# struct Project { id: i32, account_id: String, name: String, #[orm(masked)] api_token: String }
use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::with_tenant;

# async fn update(mut project: Project) -> Result<(), rullst_orm::Error> {
let audit = AuditContext::user("user-42")?
    .with_correlation_id("request-01J...")?;

with_tenant("account-a", with_audit_context(audit, async move {
    project.name = "New name".to_string();
    project.save().await
}))
.await?;
# Ok(())
# }
```

Background work can use `AuditContext::service("billing-worker")`; reviewed
maintenance can use `AuditContext::system("migration-2026-08")`. Empty,
padded, control-character, and oversized identifiers fail validation. An
auditable mutation without an active context also fails closed.

## 3. Restore an eligible update

```rust,no_run
# use rullst_orm::{FromRow, Orm};
# #[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
# #[orm(table = "projects", auditable, tenant_column = "account_id")]
# struct Project { id: i32, account_id: String, name: String, #[orm(masked)] api_token: String }
# use rullst_orm::audit::{AuditContext, with_audit_context};
# use rullst_orm::with_tenant;
# async fn restore(project: Project, audit_id: i32) -> Result<Project, rullst_orm::Error> {
let actor = AuditContext::user("admin-7")?
    .with_correlation_id("support-case-1042")?;

let restored = with_tenant("account-a", with_audit_context(actor, async move {
    project
        .restore_revision(audit_id, "approved support rollback")
        .await
}))
.await?;
# Ok(restored)
# }
```

The returned model contains the restored state. Rullst verifies the exact
model, ID, active tenant, patch version and current post-state before writing.
PostgreSQL and MySQL/MariaDB lock the row during this check. Success creates a
new `updated` audit entry with `reverted_audit_id` and the bounded reason; audit
history is not rewritten or deleted.

Use `restore_revision_with_tx` when a caller-owned SQLx transaction must include
other relational writes. Prefer `Orm::transaction` when generated post-commit
observers must wait for the final commit decision.

## Deliberate refusal cases

Restoration fails for:

- legacy v1, create, or delete entries;
- a revision from another model, record, or tenant;
- a row changed again after the selected revision;
- a patch containing a redacted/sensitive change;
- malformed, empty, too deep, too large, or excessively wide patches.

Those refusals prevent audit history from becoming an unsafe generic backup
mechanism. Use reviewed database backups and restore drills for disaster
recovery. Bulk update/delete builders do not invent per-row audit history, and
cross-process export or notification should use the transactional outbox.
