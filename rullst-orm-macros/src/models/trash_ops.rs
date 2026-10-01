//! Generated `force_delete()` and `restore()`.
//!
//! Both check the tenant and policy first, then run in a savepoint of the
//! task-scoped `Orm::transaction` (or of their own transaction) with the same
//! hook, observer, audit and post-commit pipeline as `delete()`/`save()`.

use super::mutation_parts::{
    deleted_effects, execute_mutation, instance_hook, tenant_guard, tenant_predicate,
    tenant_scoped_invalidation,
};
use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate(parsed: &ParsedModel) -> TokenStream {
    let force_delete = generate_force_delete(parsed);
    let restore = generate_restore(parsed);
    quote! {
        #force_delete
        #restore
    }
}

fn policy_check(parsed: &ParsedModel, method: &str, message: &str) -> TokenStream {
    if parsed.policy.is_empty() {
        return quote! {};
    }
    let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
    let method = syn::Ident::new(method, parsed.name.span());
    quote! {
        if !<#policy_type as rullst_orm::Policy<Self>>::#method(self).await? {
            return Err(rullst_orm::Error::Validation(#message.to_string()));
        }
    }
}

/// Reuses the task-scoped transaction or owns one, like `delete()`.
fn transactional_entrypoint(with_tx: &syn::Ident, operation: &str) -> TokenStream {
    quote! {
        let scoped_transaction = rullst_orm::CURRENT_TX
            .try_with(|transaction| transaction.clone())
            .ok();
        if let Some(transaction) = scoped_transaction {
            let mut transaction = transaction.lock().await;
            if let Some(tx) = transaction.as_mut() {
                return self.#with_tx(tx).await;
            }
        }

        let mut transaction = rullst_orm::Orm::begin_transaction().await?;
        let post_commit = rullst_orm::post_commit::PostCommitScope::new();
        let result = post_commit.run(self.#with_tx(&mut transaction)).await;
        if let Err(error) = result {
            return match transaction.rollback().await {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                    "{} failed: {}; rollback also failed: {}",
                    #operation,
                    error,
                    rollback_error,
                ))),
            };
        }
        transaction.commit().await?;
        post_commit.commit().await
    }
}

/// Runs `body` in a savepoint of `tx`; its post-commit callbacks reach the
/// enclosing commit boundary only when the savepoint is released.
fn savepoint_body(body: TokenStream, operation: &str) -> TokenStream {
    quote! {
        use rullst_orm::_sqlx::Acquire;
        let mut savepoint = (&mut **tx).begin().await?;
        let operation_callbacks = rullst_orm::post_commit::PostCommitScope::new();
        let result = operation_callbacks
            .run(async {
                let tx = &mut savepoint;
                #body
                Ok::<(), rullst_orm::Error>(())
            })
            .await;
        if let Err(error) = result {
            return match savepoint.rollback().await {
                Ok(()) => Err(error),
                Err(rollback_error) => Err(rullst_orm::Error::DatabaseError(format!(
                    "{} failed: {}; savepoint rollback also failed: {}",
                    #operation,
                    error,
                    rollback_error,
                ))),
            };
        }
        savepoint.commit().await?;
        operation_callbacks.promote_to_parent().await?;
        Ok(())
    }
}

fn load_observers() -> TokenStream {
    quote! {
        let observers = {
            let list = Self::observers().read().unwrap_or_else(|poisoned| poisoned.into_inner());
            list.clone()
        };
    }
}

fn generate_force_delete(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let tenant_guard = tenant_guard(parsed);
    let tenant = tenant_predicate(parsed);
    let tenant_binding = &tenant.binding;
    let tenant_rows_check = &tenant.rows_check;
    let policy = policy_check(
        parsed,
        "can_force_delete",
        "Policy prevents force deleting this record",
    );
    let hook_before_delete = instance_hook(parsed, &parsed.before_delete);
    let hook_after_delete = instance_hook(parsed, &parsed.after_delete);
    let load_observers = load_observers();
    let execute = execute_mutation();
    let effects = deleted_effects(parsed);
    let audit = if parsed.auditable {
        quote! {
            rullst_orm::audit::log_audit_with_tx(
                tx,
                #table_name,
                self.id,
                "force_deleted",
                Some(self.to_json()),
                None
            ).await?;
        }
    } else {
        quote! {}
    };
    let force_delete_sql = format!("DELETE FROM {} WHERE id = ?{}", table_name, tenant.clause);
    let with_tx = quote::format_ident!("__rullst_force_delete_with_tx");
    let entrypoint = transactional_entrypoint(&with_tx, "force delete");
    let body = savepoint_body(
        quote! {
            #hook_before_delete
            #load_observers
            let futures = observers.iter().map(|obs| obs.deleting(&*self));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            let query = Self::__rullst_force_delete_sql(rullst_orm::Orm::driver()?);
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | ID: {}", query, self.id);
            }
            let exec = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id) #tenant_binding;
            #execute
            #tenant_rows_check
            let futures = observers.iter().map(|obs| obs.deleted(&*self));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            #hook_after_delete
            #audit
            #effects
        },
        "force delete",
    );

    quote! {
        /// Permanently deletes this row, also for soft-delete models.
        ///
        /// Runs the tenant guard and `Policy::can_force_delete`, then the
        /// `before_delete`/`after_delete` hooks, `deleting`/`deleted`
        /// observers, an auditable `force_deleted` entry and the post-commit
        /// cache, Redis event, `committed` and Scout removal effects inside a
        /// savepoint. `cascade_soft_delete` relations are not touched.
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "force_delete")
        )]
        pub async fn force_delete(&self) -> Result<(), rullst_orm::Error> {
            rullst_orm::__transaction_access::ensure_allowed()?;
            #tenant_guard
            #policy
            #entrypoint
        }

        async fn __rullst_force_delete_with_tx(
            &self,
            tx: &mut rullst_orm::db::Transaction<'_>,
        ) -> Result<(), rullst_orm::Error> {
            #body
        }

        /// Renders the permanent-delete statement for `driver`.
        fn __rullst_force_delete_sql(driver: &str) -> String {
            if driver == "postgres" {
                rullst_orm::replace_placeholders(#force_delete_sql)
            } else {
                #force_delete_sql.to_string()
            }
        }
    }
}

fn generate_restore(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let tenant_guard = tenant_guard(parsed);
    let policy = policy_check(
        parsed,
        "can_restore",
        "Policy prevents restoring this record",
    );
    let instrument = quote! {
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "restore")
        )]
    };
    let Some(cfg) = parsed
        .soft_delete
        .as_ref()
        .filter(|_| parsed.has_soft_deletes)
    else {
        // Without soft deletes there is no trashed state to restore.
        return quote! {
            #instrument
            pub async fn restore(&self) -> Result<(), rullst_orm::Error> {
                rullst_orm::__transaction_access::ensure_allowed()?;
                #tenant_guard
                #policy
                Ok(())
            }
        };
    };

    let tenant = tenant_predicate(parsed);
    let tenant_binding = &tenant.binding;
    // A missing row is a documented no-op, checked before this tenant check.
    let tenant_rows_check = if parsed.tenant_column.is_empty() {
        quote! {}
    } else {
        tenant.rows_check.clone()
    };
    let set_clause = if cfg.value.trim().eq_ignore_ascii_case("null") || cfg.value.is_empty() {
        format!("{} = NULL", cfg.column)
    } else {
        format!("{} = {}", cfg.column, cfg.value)
    };
    let restore_sql = format!(
        "UPDATE {} SET {} WHERE id = ?{}",
        table_name, set_clause, tenant.clause
    );
    let restored_row_sql = format!("SELECT * FROM {} WHERE id = ?{}", table_name, tenant.clause);
    let load_observers = load_observers();
    let execute = execute_mutation();
    let audit = if parsed.auditable {
        quote! {
            rullst_orm::audit::log_audit_with_tx(
                tx,
                #table_name,
                self.id,
                "restored",
                Some(self.to_json()),
                Some(restored.to_json())
            ).await?;
        }
    } else {
        quote! {}
    };
    let effects = restored_effects(parsed);
    let with_tx = quote::format_ident!("__rullst_restore_with_tx");
    let entrypoint = transactional_entrypoint(&with_tx, "restore");
    let body = savepoint_body(
        quote! {
            let driver = rullst_orm::Orm::driver()?;
            let query = Self::__rullst_restore_sql(driver);
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | ID: {}", query, self.id);
            }
            let exec = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                .bind(self.id) #tenant_binding;
            #execute
            if mutation_result.rows_affected() == 0 {
                // No such row: nothing was restored, so no effect is emitted.
                // A tenant model reaches this only after its tenant guard.
                return Ok(());
            }
            #tenant_rows_check
            // Observers, audit, events and Scout receive the persisted row.
            let lookup = if driver == "postgres" {
                rullst_orm::replace_placeholders(#restored_row_sql)
            } else {
                #restored_row_sql.to_string()
            };
            let lookup_query = rullst_orm::_sqlx::query_as::<_, Self>(
                rullst_orm::_sqlx::AssertSqlSafe(lookup.as_str())
            ).bind(self.id) #tenant_binding;
            let mut restored = lookup_query
                .fetch_optional(&mut **tx)
                .await?
                .ok_or(rullst_orm::Error::RecordNotFound)?;
            restored.__rullst_decrypt_encrypted_fields()?;
            #load_observers
            let futures = observers.iter().map(|obs| obs.updated(&restored));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            let futures = observers.iter().map(|obs| obs.saved(&restored));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            #audit
            #effects
        },
        "restore",
    );

    quote! {
        /// Clears the soft-delete marker of this row.
        ///
        /// Runs the tenant guard and `Policy::can_restore`, then, inside a
        /// savepoint, the UPDATE, the `updated`/`saved` observers with the
        /// re-read row, an auditable `restored` entry and the post-commit
        /// cache, Redis `updated`/`saved` events, `committed(Updated)` and
        /// Scout re-index effects. Save hooks and the `saving`/`updating`
        /// observers are not run: they receive a mutable model whose changes
        /// restore() would not persist. A missing row is a no-op.
        #instrument
        pub async fn restore(&self) -> Result<(), rullst_orm::Error> {
            rullst_orm::__transaction_access::ensure_allowed()?;
            #tenant_guard
            #policy
            #entrypoint
        }

        async fn __rullst_restore_with_tx(
            &self,
            tx: &mut rullst_orm::db::Transaction<'_>,
        ) -> Result<(), rullst_orm::Error> {
            #body
        }

        /// Renders the soft-delete restore statement for `driver`.
        fn __rullst_restore_sql(driver: &str) -> String {
            if driver == "postgres" {
                rullst_orm::replace_placeholders(#restore_sql)
            } else {
                #restore_sql.to_string()
            }
        }
    }
}

/// Post-commit effects of a restored row, mirroring an update by `save()`.
/// Expects `restored` and `observers` bindings in scope.
fn restored_effects(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let redis_cfg = crate::feature_gates::redis();
    let invalidate = tenant_scoped_invalidation();
    let scout_update = if parsed.searchable {
        quote! {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                restored.id,
                rullst_orm::ModelOperation::Updated,
                restored.__rullst_search_json(),
            );
            rullst_orm::after_commit(move || async move {
                if let Some(engine) = rullst_orm::scout::get_search_engine() {
                    let payload: rullst_orm::_serde_json::Value =
                        rullst_orm::_serde_json::from_str(&event.payload)?;
                    engine.update(event.table, event.id, payload).await?;
                }
                Ok(())
            }).await?;
        }
    } else {
        quote! {}
    };
    quote! {
        #redis_cfg
        {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                restored.id,
                rullst_orm::ModelOperation::Updated,
                restored.to_json(),
            );
            let cache_tenant = rullst_orm::get_tenant_id();
            rullst_orm::after_commit(move || async move {
                use rullst_orm::_redis::AsyncCommands;
                // A failed invalidation must not suppress the events.
                #invalidate
                if let Ok(mut connection) = rullst_orm::Orm::redis_manager() {
                    let topic = format!("orm:events:{}:updated", event.table);
                    let _: usize = connection.publish(&topic, &event.payload).await?;
                    let topic = format!("orm:events:{}:saved", event.table);
                    let _: usize = connection.publish(&topic, &event.payload).await?;
                }
                invalidated?;
                Ok(())
            }).await?;
        }
        let event = rullst_orm::ModelCommittedEvent::new(
            #table_name,
            restored.id,
            rullst_orm::ModelOperation::Updated,
            restored.to_json(),
        );
        rullst_orm::after_commit(move || async move {
            let futures = observers.iter().map(|observer| observer.committed(&event));
            rullst_orm::_futures::future::try_join_all(futures).await?;
            Ok(())
        }).await?;
        #scout_update
    }
}
