//! Transaction-coupled audit records for explicitly auditable ORM models.

mod context;
mod diff;
mod revision;
mod schema;
mod storage;

pub use context::{AuditActorKind, AuditContext, current_audit_context, with_audit_context};
pub use diff::compute_diff;
#[doc(hidden)]
pub use revision::{RestorableRevision, apply_reverse_patch, load_restorable_revision_with_tx};
pub use schema::create_audit_table;
pub use storage::{AuditLog, log_audit, log_audit_diff, log_audit_diff_with_tx, log_audit_with_tx};

#[cfg(test)]
mod tests;
