use crate::parser::ParsedModel;
use crate::query_span::instrument_query;
use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn generate(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let tenant_prepare = tenant_prepare(parsed);
    let (policy_create, policy_update) = policy_checks(parsed);
    let save_doc = if parsed.snapshot_field.is_some() {
        super::save_docs::TRACKED_SAVE_DOC
    } else {
        super::save_docs::SAVE_DOC
    };
    let clean_return = super::dirty_tracking::clean_save_return(parsed);
    let (keep_snapshot, restore_snapshot) = super::dirty_tracking::snapshot_guard(parsed);

    if !parsed.auditable {
        let save_body = instrument_query(
            name,
            table_name,
            "save",
            &[],
            quote! {
                rullst_orm::__transaction_access::ensure_allowed()?;
                #clean_return
                let scoped_transaction = rullst_orm::CURRENT_TX
                    .try_with(|transaction| transaction.clone())
                    .ok();
                if let Some(transaction) = scoped_transaction {
                    let mut transaction = transaction.lock().await;
                    if let Some(tx) = transaction.as_mut() {
                        return self.save_with_tx(tx).await;
                    }
                }

                let original_id = self.id;
                #keep_snapshot
                let mut transaction = rullst_orm::Orm::begin_transaction().await?;
                let post_commit = rullst_orm::post_commit::PostCommitScope::new();
                // The whole transaction rolls back on error, so no savepoint.
                let save_result = post_commit
                    .run(self.__rullst_save_in_tx(&mut transaction))
                    .await;
                if let Err(save_error) = save_result {
                    self.id = original_id;
                    #restore_snapshot
                    return match transaction.rollback().await {
                        Ok(()) => Err(save_error),
                        Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                            "save failed: {}; rollback also failed: {}",
                            save_error,
                            rollback_error,
                        ))),
                    };
                }
                // A failed COMMIT persisted nothing: keep the handle new so a
                // retry inserts instead of updating a row that never existed.
                if let Err(commit_error) = transaction.commit().await {
                    self.id = original_id;
                    #restore_snapshot
                    return Err(rullst_orm::Error::from(commit_error));
                }
                post_commit.commit().await
            },
        );
        let save_with_tx_body = instrument_query(
            name,
            table_name,
            "save_with_tx",
            &[],
            quote! {
                use rullst_orm::_sqlx::Acquire;
                #clean_return
                let original_id = self.id;
                #keep_snapshot
                let mut savepoint = (&mut **tx).begin().await?;
                let operation_callbacks = rullst_orm::post_commit::PostCommitScope::new();
                let save_result = operation_callbacks
                    .run(self.__rullst_save_in_tx(&mut savepoint))
                    .await;
                if let Err(save_error) = save_result {
                    self.id = original_id;
                    #restore_snapshot
                    return match savepoint.rollback().await {
                        Ok(()) => Err(save_error),
                        Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                            "save failed: {}; savepoint rollback also failed: {}",
                            save_error,
                            rollback_error,
                        ))),
                    };
                }
                if let Err(release_error) = savepoint.commit().await {
                    self.id = original_id;
                    #restore_snapshot
                    return Err(rullst_orm::Error::from(release_error));
                }
                operation_callbacks.promote_to_parent().await?;
                Ok(())
            },
        );
        return quote! {
            #[doc = #save_doc]
            pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
                #save_body
            }

            /// Saves through a caller-owned transaction.
            ///
            /// The write runs in a savepoint: if a hook, observer or policy fails,
            /// the savepoint is rolled back, `id` is restored and the error is
            /// returned, so the caller's transaction can continue without it.
            /// Strict post-commit effects require this transaction to be managed by
            /// `Orm::transaction`; a raw SQLx transaction cannot expose its later
            /// commit decision to the ORM.
            pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
                #save_with_tx_body
            }

            async fn __rullst_save_in_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
                let is_new = self.id == 0;
                #tenant_prepare
                if is_new {
                    #policy_create
                } else {
                    #policy_update
                }
                self.save_with_tx_internal(&mut **tx).await.map(|_| ())
            }
        };
    }

    let pre_image = super::row_lookup::locked_row(parsed, &quote::format_ident!("old_row"));
    let revision_lookup =
        super::row_lookup::locked_row(parsed, &quote::format_ident!("current_row"));
    // save() never writes the soft-delete marker, so the audited post-state
    // (and the handle) take the stored marker instead of a stale one.
    let soft_delete_sync = parsed.soft_delete_column().map(|column| {
        let column = syn::Ident::new(column, parsed.name.span());
        quote! {
            if let Some(old_model) = old_model_for_audit.as_ref() {
                self.#column = old_model.#column.clone();
            }
        }
    });
    let before_tx = audit_before_tx(&pre_image, soft_delete_sync);
    let after_tx = audit_after_tx(table_name);
    let restored_after_fetch =
        super::update_builder::after_fetch_hook(parsed, &quote::format_ident!("restored"));
    let revision_restore = revision_restore(table_name, &revision_lookup, &restored_after_fetch);

    let save_body = instrument_query(
        name,
        table_name,
        "save",
        &[],
        quote! {
            rullst_orm::__transaction_access::ensure_allowed()?;
            #clean_return
            let scoped_transaction = rullst_orm::CURRENT_TX
                .try_with(|transaction| transaction.clone())
                .ok();
            if let Some(transaction) = scoped_transaction {
                let mut transaction = transaction.lock().await;
                if let Some(tx) = transaction.as_mut() {
                    return self.save_with_tx(tx).await;
                }
            }

            let original_id = self.id;
            #keep_snapshot
            let mut transaction = rullst_orm::Orm::begin_transaction().await?;
            let post_commit = rullst_orm::post_commit::PostCommitScope::new();
            let save_result = post_commit
                .run(self.save_with_tx(&mut transaction))
                .await;
            if let Err(save_error) = save_result {
                self.id = original_id;
                #restore_snapshot
                return match transaction.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            // A failed COMMIT persisted nothing: keep the handle new so a
            // retry inserts instead of updating a row that never existed.
            if let Err(commit_error) = transaction.commit().await {
                self.id = original_id;
                #restore_snapshot
                return Err(rullst_orm::Error::from(commit_error));
            }
            post_commit.commit().await
        },
    );
    let save_with_tx_body = instrument_query(
        name,
        table_name,
        "save_with_tx",
        &[],
        quote! {
            use rullst_orm::_sqlx::Acquire;
            #clean_return
            let original_id = self.id;
            #keep_snapshot
            let mut savepoint = (&mut **tx).begin().await?;
            let operation_callbacks = rullst_orm::post_commit::PostCommitScope::new();
            let save_result = operation_callbacks
                .run(async {
                    let tx = &mut savepoint;
                    let is_new = self.id == 0;
                    #tenant_prepare
                    if is_new {
                        #policy_create
                    } else {
                        #policy_update
                    }
                    #before_tx
                    // An unchanged tracked record wrote nothing to audit.
                    if self.save_with_tx_internal(&mut **tx).await? {
                        #after_tx
                    }
                    Ok::<(), rullst_orm::Error>(())
                })
                .await;
            if let Err(save_error) = save_result {
                self.id = original_id;
                #restore_snapshot
                return match savepoint.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; savepoint rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            if let Err(release_error) = savepoint.commit().await {
                self.id = original_id;
                #restore_snapshot
                return Err(rullst_orm::Error::from(release_error));
            }
            operation_callbacks.promote_to_parent().await?;
            Ok(())
        },
    );

    quote! {
        #[doc = #save_doc]
        pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
            #save_body
        }

        /// Saves through a caller-owned transaction.
        ///
        /// Strict post-commit effects require this transaction to be managed by
        /// `Orm::transaction`; a raw SQLx transaction cannot expose its later
        /// commit decision to the ORM.
        pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
            #save_with_tx_body
        }

        #revision_restore
    }
}

fn revision_restore(
    table_name: &str,
    lookup: &TokenStream,
    restored_after_fetch: &TokenStream,
) -> TokenStream {
    quote! {
        /// Restores one bounded update revision and records the compensating
        /// mutation under the active audit principal.
        pub async fn restore_revision(
            &self,
            audit_id: i32,
            reason: impl Into<String>,
        ) -> Result<Self, rullst_orm::Error> {
            rullst_orm::__transaction_access::ensure_allowed()?;
            let reason = reason.into();
            let scoped_transaction = rullst_orm::CURRENT_TX
                .try_with(|transaction| transaction.clone())
                .ok();
            if let Some(transaction) = scoped_transaction {
                let mut transaction = transaction.lock().await;
                if let Some(tx) = transaction.as_mut() {
                    return self
                        .restore_revision_with_tx(tx, audit_id, reason)
                        .await;
                }
            }

            let mut transaction = rullst_orm::Orm::begin_transaction().await?;
            let post_commit = rullst_orm::post_commit::PostCommitScope::new();
            let restore_result = post_commit
                .run(self.restore_revision_with_tx(&mut transaction, audit_id, reason))
                .await;
            let restored = match restore_result {
                Ok(restored) => restored,
                Err(restore_error) => {
                    return match transaction.rollback().await {
                        Ok(()) => Err(restore_error),
                        Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                            "revision restore failed: {}; rollback also failed: {}",
                            restore_error,
                            rollback_error,
                        ))),
                    };
                }
            };
            transaction.commit().await?;
            post_commit.commit().await?;
            Ok(restored)
        }

        /// Restores one revision through a caller-owned transaction.
        pub async fn restore_revision_with_tx(
            &self,
            tx: &mut rullst_orm::db::Transaction<'_>,
            audit_id: i32,
            reason: impl Into<String>,
        ) -> Result<Self, rullst_orm::Error> {
            use rullst_orm::_sqlx::Acquire;
            let reason = reason.into();
            let mut savepoint = (&mut **tx).begin().await?;
            let restore_callbacks = rullst_orm::post_commit::PostCommitScope::new();
            let restore_result = restore_callbacks
                .run(async {
                    let tx = &mut savepoint;
                    let revision = rullst_orm::audit::load_restorable_revision_with_tx(
                        tx,
                        audit_id,
                        #table_name,
                        self.id,
                    )
                    .await?;

                    #lookup
                    let current_model = current_row.ok_or(rullst_orm::Error::RecordNotFound)?;
                    let current_json = current_model.__rullst_cache_json_value()?;
                    let restored_json = rullst_orm::audit::apply_reverse_patch(
                        current_json,
                        revision.restore_patch(),
                    )?;
                    let mut restored = Self::from_json_value(restored_json)?;
                    if restored.id != self.id {
                        return Err(rullst_orm::Error::Validation(
                            "audit revision attempted to change the model identity".to_string(),
                        ));
                    }
                    // The patch matches stored values; the save lifecycle then
                    // expects the representation every read returns.
                    #restored_after_fetch
                    let restore_context = rullst_orm::audit::current_audit_context()
                        .ok_or_else(|| rullst_orm::Error::Validation(
                            "revision restore requires an active audit context".to_string(),
                        ))?
                        .for_revision_restore(audit_id, reason)?;
                    rullst_orm::audit::with_audit_context(
                        restore_context,
                        restored.save_with_tx(tx),
                    )
                    .await?;
                    Ok::<Self, rullst_orm::Error>(restored)
                })
                .await;
            let restored = match restore_result {
                Ok(restored) => restored,
                Err(restore_error) => {
                    return match savepoint.rollback().await {
                        Ok(()) => Err(restore_error),
                        Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                            "revision restore failed: {}; savepoint rollback also failed: {}",
                            restore_error,
                            rollback_error,
                        ))),
                    };
                }
            };
            savepoint.commit().await?;
            restore_callbacks.promote_to_parent().await?;
            Ok(restored)
        }
    }
}

pub(super) fn tenant_prepare(parsed: &ParsedModel) -> TokenStream {
    if parsed.tenant_column.is_empty() {
        return quote! {};
    }

    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let column = syn::Ident::new(&parsed.tenant_column, name.span());
    quote! {
        let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
            rullst_orm::Error::Validation(format!(
                "tenant context is required to save `{}`",
                #table_name
            ))
        })?;
        self.#column = tenant.try_into().map_err(|_| {
            rullst_orm::Error::Validation(format!(
                "tenant context type does not match `{}.{}`",
                #table_name,
                stringify!(#column)
            ))
        })?;
    }
}

fn policy_checks(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    if parsed.policy.is_empty() {
        return (quote! {}, quote! {});
    }

    let policy = syn::Ident::new(&parsed.policy, parsed.name.span());
    (
        quote! {
            if !rullst_orm::__transaction_access::run(<#policy as rullst_orm::Policy<Self>>::can_create(self)).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents creation of this record".to_string()));
            }
        },
        quote! {
            if !rullst_orm::__transaction_access::run(<#policy as rullst_orm::Policy<Self>>::can_update(self)).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents updating this record".to_string()));
            }
        },
    )
}

/// Reads the audit pre-image locked, so a concurrent writer cannot change the
/// row between this read and the full-row UPDATE that the diff describes.
fn audit_before_tx(pre_image: &TokenStream, soft_delete_sync: Option<TokenStream>) -> TokenStream {
    quote! {
        let old_model_for_audit = if is_new {
            None
        } else {
            #pre_image
            old_row
        };
        #soft_delete_sync
    }
}

fn audit_after_tx(table_name: &str) -> TokenStream {
    quote! {
        if is_new {
            rullst_orm::audit::log_audit_with_tx(
                tx,
                #table_name,
                self.id,
                "created",
                None,
                Some(self.to_json())
            ).await?;
        } else if let Some(old_model) = old_model_for_audit {
            let redacted_changes = self.__rullst_redacted_changes(&old_model);
            rullst_orm::audit::log_audit_diff_redacted_with_tx(
                tx,
                #table_name,
                self.id,
                "updated",
                &old_model.to_json(),
                &self.to_json(),
                &redacted_changes,
            ).await?;
        }
    }
}

#[cfg(test)]
#[path = "save_entrypoints_tests.rs"]
mod tests;
