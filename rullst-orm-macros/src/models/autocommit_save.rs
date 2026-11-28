//! A direct `save()` that sends one statement runs it without `BEGIN` and
//! `COMMIT` (since 13.1): the statement commits on its own, and the save runs
//! nothing after it that could fail and would need a rollback.
//!
//! A model qualifies at compile time when it is not auditable and declares no
//! `before_save` or `after_save` hook and no policy. A save qualifies at run
//! time when no observer is registered, the query timeout is off
//! (`Orm::set_query_timeout(0)`; it is 30 seconds by default, and a statement
//! that times out keeps running on the server, where only a transaction
//! undoes it) and it sends a single statement that reads nothing back except
//! an inserted `id`: an insert, a full-row update, or a tracked update that
//! writes every tracked column of a model without `#[orm(db_default)]` or
//! soft-delete columns. Every other direct save keeps its transaction.

use crate::models::dirty_tracking;
use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// The statement `save()` runs before it opens a transaction, and the
/// methods behind it; both empty for a model that never qualifies.
pub(super) struct AutocommitSave {
    pub(super) attempt: TokenStream,
    pub(super) methods: TokenStream,
}

/// Whether nothing the model declares runs around the statement of its save:
/// no audit entry, no hook and no policy.
pub(super) fn model_qualifies(parsed: &ParsedModel) -> bool {
    !parsed.auditable
        && parsed.before_save.is_empty()
        && parsed.after_save.is_empty()
        && parsed.policy.is_empty()
}

/// The condition under which the save of `self`, its tenant column already
/// set, sends one statement and reads nothing back but an inserted `id`.
fn single_statement(parsed: &ParsedModel) -> TokenStream {
    let Some(snapshot) = parsed.snapshot_field.as_ref() else {
        // An insert, or the full-row update.
        return quote! { true };
    };
    if dirty_tracking::has_database_managed_columns(parsed)
        || dirty_tracking::update_columns(parsed).is_empty()
    {
        // Every tracked update reads the row back, or writes nothing.
        return quote! { self.id == 0 };
    }
    quote! {
        self.id == 0
            || self
                .__rullst_fingerprints()
                .iter()
                .enumerate()
                .all(|(index, fingerprint)| self.#snapshot.__is_changed(index, *fingerprint))
    }
}

/// `tenant_prepare` applies the tenant context as the save would;
/// `keep_snapshot` and `restore_snapshot` restore the handle after a failed
/// statement.
pub(super) fn generate(
    parsed: &ParsedModel,
    tenant_prepare: &TokenStream,
    keep_snapshot: &TokenStream,
    restore_snapshot: &TokenStream,
) -> AutocommitSave {
    if !model_qualifies(parsed) {
        return AutocommitSave {
            attempt: TokenStream::new(),
            methods: TokenStream::new(),
        };
    }
    let single_statement = single_statement(parsed);
    let attempt = quote! {
        let observed = !Self::observers()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty();
        if self.__rullst_saves_in_one_statement(observed)? {
            return self.__rullst_save_autocommit().await;
        }
    };
    let methods = quote! {
        /// Whether this `save()` can run without a transaction: no observer
        /// and no query timeout, and one statement that reads nothing back
        /// but an inserted `id`. Applies the tenant context first, as the
        /// save does.
        fn __rullst_saves_in_one_statement(&mut self, observed: bool) -> Result<bool, rullst_orm::Error> {
            if observed || rullst_orm::schema::get_query_timeout().is_some() {
                return Ok(false);
            }
            #tenant_prepare
            Ok(#single_statement)
        }

        /// Runs a single-statement save on a pool connection without a
        /// transaction. A failed statement persisted nothing, so the handle
        /// is restored as after a rollback. Post-commit callbacks run once
        /// the statement has committed.
        async fn __rullst_save_autocommit(&mut self) -> Result<(), rullst_orm::Error> {
            let original_id = self.id;
            #keep_snapshot
            let mut connection = rullst_orm::Orm::pool()?.acquire().await?;
            let post_commit = rullst_orm::post_commit::PostCommitScope::new();
            let is_new = self.id == 0;
            let save_result = post_commit
                .run(self.__rullst_save_observed(&mut *connection, is_new, Vec::new(), true))
                .await;
            drop(connection);
            if let Err(save_error) = save_result {
                self.id = original_id;
                #restore_snapshot
                return Err(save_error);
            }
            post_commit.commit().await
        }
    };
    AutocommitSave { attempt, methods }
}

#[cfg(test)]
#[path = "autocommit_save_tests.rs"]
mod tests;
