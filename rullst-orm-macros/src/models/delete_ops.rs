//! Generated `delete()`, `delete_with_tx()` and the soft-delete cascade.

use crate::models::mutation_parts::{
    TenantPredicate, deleted_effects, instance_hook, tenant_guard, tenant_predicate,
};
use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_delete_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let has_soft_deletes = parsed.has_soft_deletes;
    let soft_delete_config = if has_soft_deletes {
        let Some(config) = parsed.soft_delete.as_ref() else {
            return syn::Error::new(
                name.span(),
                "internal ORM macro error: soft-delete configuration is missing",
            )
            .to_compile_error();
        };
        Some(config)
    } else {
        None
    };

    let hook_before_delete = instance_hook(parsed, &parsed.before_delete);
    let hook_after_delete = instance_hook(parsed, &parsed.after_delete);
    let tenant_guard = tenant_guard(parsed);
    let TenantPredicate {
        clause: tenant_where_clause,
        binding: tenant_binding,
        rows_check: tenant_rows_check,
    } = tenant_predicate(parsed);
    let deleted_effects = deleted_effects(parsed);

    let audit_after_delete_with_tx = if parsed.auditable {
        quote! {
            rullst_orm::audit::log_audit_with_tx(
                tx,
                #table_name,
                self.id,
                "deleted",
                Some(self.to_json()),
                None
            ).await?;
        }
    } else {
        quote! {}
    };

    let delete_logic = if let Some(cfg) = soft_delete_config {
        let delval_expr = if cfg.delval.trim().is_empty() {
            "CURRENT_TIMESTAMP".to_string()
        } else {
            cfg.delval.clone()
        };
        let set_clause = format!("{} = {}", cfg.column, delval_expr);
        // Only a live row is soft-deleted: deleting a trashed row again
        // must not re-stamp its deletion time or repeat its effects.
        let live = crate::builder::soft_delete_where_clause(cfg, false);
        let sql = format!(
            "UPDATE {} SET {} WHERE id = ? AND {}{}",
            table_name, set_clause, live, tenant_where_clause
        );
        quote! {
            let driver = rullst_orm::Orm::driver()?;
            let query = if driver == "postgres" {
                rullst_orm::replace_placeholders(#sql)
            } else {
                #sql.to_string()
            };
        }
    } else {
        quote! {
            let driver = rullst_orm::Orm::driver()?;
            let query = if driver == "postgres" {
                let base = format!("DELETE FROM {} WHERE id = ?{}", #table_name, #tenant_where_clause);
                rullst_orm::replace_placeholders(&base)
            } else {
                format!("DELETE FROM {} WHERE id = ?{}", #table_name, #tenant_where_clause)
            };
        }
    };

    let policy_check_delete = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !rullst_orm::__transaction_access::run(<#policy_type as rullst_orm::Policy<Self>>::can_delete(self)).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents deleting this record".to_string()));
            }
        }
    } else {
        quote! {}
    };

    let mut cascade_deletes_with_tx = quote! {};
    if has_soft_deletes {
        for rel in &parsed.relations {
            if rel.cascade_soft_delete && (rel.rel_type == "has_many" || rel.rel_type == "has_one")
            {
                let rel_model = syn::Ident::new(&rel.rel_model, name.span());
                let default_fk = format!("{}_id", name.to_string().to_lowercase());
                let fk = if rel.foreign_key.is_empty() {
                    default_fk
                } else {
                    rel.foreign_key.clone()
                };
                let lk = syn::Ident::new(
                    if rel.local_key.is_empty() {
                        "id"
                    } else {
                        &rel.local_key
                    },
                    name.span(),
                );

                // Only soft-delete models define this method; a cascade into a
                // hard-delete child is reported at the relation field instead of
                // permanently deleting the children of a restorable parent.
                let cascade = syn::Ident::new(
                    "__rullst_cascade_soft_delete_with_tx",
                    rel.field_name.span(),
                );
                // The child's global_scope must not leave children outside
                // it live under a trashed parent; its tenant scope applies.
                cascade_deletes_with_tx.extend(quote! {
                    #rel_model::__rullst_cascade_query()
                        .where_eq(#fk, self.#lk.clone())
                        .#cascade(tx)
                        .await?;
                });
            }
        }
    }

    quote! {
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "delete")
        )]
        pub async fn delete(&self) -> Result<(), rullst_orm::Error> {
            rullst_orm::__transaction_access::ensure_allowed()?;
            let scoped_transaction = rullst_orm::CURRENT_TX
                .try_with(|transaction| transaction.clone())
                .ok();
            if let Some(transaction) = scoped_transaction {
                let mut transaction = transaction.lock().await;
                if let Some(tx) = transaction.as_mut() {
                    return self.delete_with_tx(tx).await;
                }
            }

            let mut transaction = rullst_orm::Orm::begin_transaction().await?;
            let post_commit = rullst_orm::post_commit::PostCommitScope::new();
            let delete_result = post_commit
                .run(self.delete_with_tx(&mut transaction))
                .await;
            if let Err(delete_error) = delete_result {
                return match transaction.rollback().await {
                    Ok(()) => Err(delete_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "cascade delete failed: {}; rollback also failed: {}",
                        delete_error,
                        rollback_error,
                    ))),
                };
            }
            transaction.commit().await?;
            post_commit.commit().await
        }

        /// Deletes through a caller-owned transaction.
        ///
        /// Strict post-commit effects require this transaction to be managed by
        /// `Orm::transaction`; a raw SQLx transaction cannot expose its later
        /// commit decision to the ORM.
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, tx),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "delete_with_tx")
        )]
        pub async fn delete_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<(), rullst_orm::Error> {
            use rullst_orm::_sqlx::Acquire;
            let mut savepoint = (&mut **tx).begin().await?;
            let operation_callbacks = rullst_orm::post_commit::PostCommitScope::new();
            let delete_result = operation_callbacks
                .run(async {
                    let tx = &mut savepoint;
                    self.delete_with_tx_internal(&mut **tx).await?;
                    #cascade_deletes_with_tx
                    #audit_after_delete_with_tx
                    Ok::<(), rullst_orm::Error>(())
                })
                .await;
            if let Err(delete_error) = delete_result {
                return match savepoint.rollback().await {
                    Ok(()) => Err(delete_error),
                    Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                        "delete failed: {}; savepoint rollback also failed: {}",
                        delete_error,
                        rollback_error,
                    ))),
                };
            }
            savepoint.commit().await?;
            operation_callbacks.promote_to_parent().await?;
            Ok(())
        }

        async fn delete_with_tx_internal<'e, E>(&self, executor: E) -> Result<(), rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            #tenant_guard
            #policy_check_delete
            #hook_before_delete
            let observers = {
                let list = Self::observers().read().unwrap_or_else(|poisoned| poisoned.into_inner());
                list.clone()
            };
            let futures = observers.iter().map(|obs| obs.deleting(&*self));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            #delete_logic
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | ID: {}", query, self.id);
            }
            let exec = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id) #tenant_binding;
            let timeout = rullst_orm::schema::get_query_timeout();
            let mutation_result = if let Some(t) = timeout {
                tokio::time::timeout(t, exec.execute(executor))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                exec.execute(executor).await?
            };
            #tenant_rows_check
            let futures = observers.iter().map(|obs| obs.deleted(&*self));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            #hook_after_delete
            #deleted_effects
            Ok(())
        }
    }
}
