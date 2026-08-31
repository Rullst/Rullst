use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn generate(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let tenant_prepare = tenant_prepare(parsed);
    let (policy_create, policy_update) = policy_checks(parsed);

    if !parsed.auditable {
        return quote! {
            #[rullst_orm::_tracing::instrument(
                name = "rullst.orm.query",
                target = "rullst_orm",
                skip(self),
                fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "save")
            )]
            pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
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
                let mut transaction = rullst_orm::Orm::begin_transaction().await?;
                let post_commit = rullst_orm::post_commit::PostCommitScope::new();
                let save_result = post_commit
                    .run(self.save_with_tx(&mut transaction))
                    .await;
                if let Err(save_error) = save_result {
                    self.id = original_id;
                    return match transaction.rollback().await {
                        Ok(()) => Err(save_error),
                        Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                            "save failed: {}; rollback also failed: {}",
                            save_error,
                            rollback_error,
                        ))),
                    };
                }
                transaction.commit().await?;
                post_commit.commit().await
            }

            /// Saves through a caller-owned transaction.
            ///
            /// Strict post-commit effects require this transaction to be managed by
            /// `Orm::transaction`; a raw SQLx transaction cannot expose its later
            /// commit decision to the ORM.
            #[rullst_orm::_tracing::instrument(
                name = "rullst.orm.query",
                target = "rullst_orm",
                skip(self, tx),
                fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "save_with_tx")
            )]
            pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
                let is_new = self.id == 0;
                #tenant_prepare
                if is_new {
                    #policy_create
                } else {
                    #policy_update
                }
                self.save_with_tx_internal(&mut **tx).await
            }
        };
    }

    let lookup = audit_lookup(parsed);
    let revision_lookup = revision_lookup(parsed);
    let before_tx = audit_before_tx(&lookup);
    let after_tx = audit_after_tx(table_name);
    let revision_restore = revision_restore(table_name, &revision_lookup);

    quote! {
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "save")
        )]
        pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
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
            let mut transaction = rullst_orm::Orm::begin_transaction().await?;
            let post_commit = rullst_orm::post_commit::PostCommitScope::new();
            let save_result = post_commit
                .run(self.save_with_tx(&mut transaction))
                .await;
            if let Err(save_error) = save_result {
                self.id = original_id;
                return match transaction.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            transaction.commit().await?;
            post_commit.commit().await
        }

        /// Saves through a caller-owned transaction.
        ///
        /// Strict post-commit effects require this transaction to be managed by
        /// `Orm::transaction`; a raw SQLx transaction cannot expose its later
        /// commit decision to the ORM.
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, tx),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "save_with_tx")
        )]
        pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
            use rullst_orm::_sqlx::Acquire;
            let original_id = self.id;
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
                    self.save_with_tx_internal(&mut **tx).await?;
                    #after_tx
                    Ok::<(), rullst_orm::Error>(())
                })
                .await;
            if let Err(save_error) = save_result {
                self.id = original_id;
                return match savepoint.rollback().await {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "auditable save failed: {}; savepoint rollback also failed: {}",
                        save_error,
                        rollback_error,
                    ))),
                };
            }
            savepoint.commit().await?;
            operation_callbacks.promote_to_parent().await?;
            Ok(())
        }

        #revision_restore
    }
}

fn revision_restore(table_name: &str, lookup: &TokenStream) -> TokenStream {
    quote! {
        /// Restores one bounded update revision and records the compensating
        /// mutation under the active audit principal.
        pub async fn restore_revision(
            &self,
            audit_id: i32,
            reason: impl Into<String>,
        ) -> Result<Self, rullst_orm::Error> {
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

                    let driver = rullst_orm::Orm::driver()?;
                    #lookup
                    let mut current_model = q
                        .fetch_optional(&mut **tx)
                        .await?
                        .ok_or(rullst_orm::Error::RecordNotFound)?;
                    current_model.__rullst_decrypt_encrypted_fields()?;
                    let current_json = rullst_orm::_serde_json::from_str(
                        &current_model.to_cache_json(),
                    )?;
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

fn tenant_prepare(parsed: &ParsedModel) -> TokenStream {
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
            if !<#policy as rullst_orm::Policy<Self>>::can_create(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents creation of this record".to_string()));
            }
        },
        quote! {
            if !<#policy as rullst_orm::Policy<Self>>::can_update(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents updating this record".to_string()));
            }
        },
    )
}

fn audit_lookup(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    if parsed.tenant_column.is_empty() {
        return quote! {
            let query = if driver == "postgres" {
                format!("SELECT * FROM {} WHERE id = $1", #table_name)
            } else {
                format!("SELECT * FROM {} WHERE id = ?", #table_name)
            };
            let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id);
        };
    }

    let column = syn::Ident::new(&parsed.tenant_column, name.span());
    let column_name = &parsed.tenant_column;
    quote! {
        let query = if driver == "postgres" {
            format!("SELECT * FROM {} WHERE id = $1 AND {} = $2", #table_name, #column_name)
        } else {
            format!("SELECT * FROM {} WHERE id = ? AND {} = ?", #table_name, #column_name)
        };
        let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
            .bind(self.id)
            .bind(self.#column.clone());
    }
}

fn revision_lookup(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    if parsed.tenant_column.is_empty() {
        return quote! {
            let query = if driver == "postgres" {
                format!("SELECT * FROM {} WHERE id = $1 FOR UPDATE", #table_name)
            } else if driver == "mysql" {
                format!("SELECT * FROM {} WHERE id = ? FOR UPDATE", #table_name)
            } else {
                format!("SELECT * FROM {} WHERE id = ?", #table_name)
            };
            let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id);
        };
    }

    let column = syn::Ident::new(&parsed.tenant_column, name.span());
    let column_name = &parsed.tenant_column;
    quote! {
        let query = if driver == "postgres" {
            format!("SELECT * FROM {} WHERE id = $1 AND {} = $2 FOR UPDATE", #table_name, #column_name)
        } else if driver == "mysql" {
            format!("SELECT * FROM {} WHERE id = ? AND {} = ? FOR UPDATE", #table_name, #column_name)
        } else {
            format!("SELECT * FROM {} WHERE id = ? AND {} = ?", #table_name, #column_name)
        };
        let q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
            .bind(self.id)
            .bind(self.#column.clone());
    }
}

fn audit_before_tx(lookup: &TokenStream) -> TokenStream {
    quote! {
        let mut old_model_for_audit = if !is_new {
            let driver = rullst_orm::Orm::driver()?;
            #lookup
            q.fetch_optional(&mut **tx).await?
        } else {
            None
        };
        if let Some(old_model) = old_model_for_audit.as_mut() {
            old_model.__rullst_decrypt_encrypted_fields()?;
        }
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
            rullst_orm::audit::log_audit_diff_with_tx(
                tx,
                #table_name,
                self.id,
                "updated",
                &old_model.to_json(),
                &self.to_json()
            ).await?;
        }
    }
}
