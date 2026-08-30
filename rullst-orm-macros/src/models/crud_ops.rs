use crate::models::save_entrypoints;
use crate::parser::{EncryptedFieldKind, ParsedModel};
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_save_method(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let normal_fields = &parsed.normal_fields;

    let hook_before_save = if !parsed.before_save.is_empty() {
        let method = syn::Ident::new(&parsed.before_save, name.span());
        quote! { self.#method().await?; }
    } else {
        quote! {}
    };
    let hook_after_save = if !parsed.after_save.is_empty() {
        let method = syn::Ident::new(&parsed.after_save, name.span());
        quote! { self.#method().await?; }
    } else {
        quote! {}
    };

    let scout_after_commit = if parsed.searchable {
        quote! {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                operation,
                self.to_json(),
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

    let mut insert_columns = vec![];
    let mut insert_placeholders = vec![];
    let mut bind_inserts = vec![];

    let mut update_sets = vec![];
    let mut bind_updates = vec![];

    for field_name in normal_fields {
        let field_name_str = field_name.to_string();
        if field_name_str != "id" {
            insert_columns.push(field_name_str.clone());
            insert_placeholders.push("?");
            let encrypted_kind = parsed
                .encrypted_fields
                .iter()
                .find(|field| field.name == *field_name)
                .map(|field| field.kind);
            let binding = match encrypted_kind {
                Some(EncryptedFieldKind::String) => quote! {
                    .bind(rullst_orm::privacy::encrypt_model_field(
                        &self.#field_name,
                        #table_name,
                        #field_name_str,
                    )?)
                },
                Some(EncryptedFieldKind::OptionalString) => quote! {
                    .bind(match self.#field_name.as_deref() {
                        Some(value) => Some(rullst_orm::privacy::encrypt_model_field(
                            value,
                            #table_name,
                            #field_name_str,
                        )?),
                        None => None,
                    })
                },
                None => quote! { .bind(self.#field_name.clone()) },
            };
            bind_inserts.push(binding.clone());

            update_sets.push(format!("{} = ?", field_name_str));
            bind_updates.push(binding);
        }
    }

    let insert_columns_str = insert_columns.join(", ");
    let insert_placeholders_str = insert_placeholders.join(", ");
    let update_sets_str = update_sets.join(", ");

    let update_tenant_clause = if !parsed.tenant_column.is_empty() {
        format!(" AND {} = ?", parsed.tenant_column)
    } else {
        String::new()
    };
    let update_tenant_binding = if !parsed.tenant_column.is_empty() {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        quote! { .bind(self.#col_ident.clone()) }
    } else {
        quote! {}
    };
    let update_tenant_result_check = if !parsed.tenant_column.is_empty() {
        quote! {
            if update_result.rows_affected() != 1 {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        }
    } else {
        quote! {}
    };

    let save_entrypoints = save_entrypoints::generate(parsed);

    quote! {
        #save_entrypoints

        async fn save_with_tx_internal<'e, E>(&mut self, executor: E) -> Result<(), rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            let is_new = self.id == 0;
            #hook_before_save
            let observers = {
                let list = Self::observers().read().unwrap_or_else(|poisoned| poisoned.into_inner());
                list.clone()
            };
            for obs in &observers {
                obs.saving(self).await?;
            }
            if self.id == 0 {
                for obs in &observers {
                    obs.creating(self).await?;
                }
                let driver = rullst_orm::Orm::driver()?;
                if driver == "postgres" || driver == "sqlite" {
                    use rullst_orm::_sqlx::Execute;
                    let mut final_sql = format!("INSERT INTO {} ({}) VALUES ({}) RETURNING id", #table_name, #insert_columns_str, #insert_placeholders_str);
                    if driver == "postgres" {
                        final_sql = rullst_orm::replace_placeholders(&final_sql);
                    }
                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?}", final_sql);
                    }
                    let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(final_sql.as_str()));
                    let row = {
                        let exec = query #(#bind_inserts)*;
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, exec.fetch_one(executor))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            exec.fetch_one(executor).await?
                        }
                    };
                    self.id = rullst_orm::_sqlx::Row::try_get(&row, "id")?;
                } else {
                    use rullst_orm::_sqlx::Execute;
                    let mut final_sql = format!("INSERT INTO {} ({}) VALUES ({})", #table_name, #insert_columns_str, #insert_placeholders_str);
                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?}", final_sql);
                    }
                    let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(final_sql.as_str()));
                    let result = {
                        let exec = query #(#bind_inserts)*;
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, exec.execute(executor))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            exec.execute(executor).await?
                        }
                    };
                    self.id = {
                        use rullst_orm::database::QueryResultExt;
                        result.get_last_insert_id() as i32
                    }
                }
                let futures = observers.iter().map(|obs| obs.created(&*self));
                rullst_orm::_futures::future::try_join_all(futures).await?;
            } else {
                for obs in &observers {
                    obs.updating(self).await?;
                }
                use rullst_orm::_sqlx::Execute;
                let mut final_sql = format!(
                    "UPDATE {} SET {} WHERE id = ?{}",
                    #table_name,
                    #update_sets_str,
                    #update_tenant_clause
                );
                if rullst_orm::Orm::driver()? == "postgres" {
                    final_sql = rullst_orm::replace_placeholders(&final_sql);
                }
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | ID: {}", final_sql, self.id);
                }
                let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(final_sql.as_str()));
                let exec = query #(#bind_updates)*.bind(self.id) #update_tenant_binding;
                let timeout = rullst_orm::schema::get_query_timeout();
                let update_result = if let Some(t) = timeout {
                    tokio::time::timeout(t, exec.execute(executor))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    exec.execute(executor).await?
                };
                #update_tenant_result_check
                let futures = observers.iter().map(|obs| obs.updated(&*self));
                rullst_orm::_futures::future::try_join_all(futures).await?;
            }
            let futures = observers.iter().map(|obs| obs.saved(&*self));
            rullst_orm::_futures::future::try_join_all(futures).await?;
            #hook_after_save
            let operation = if is_new {
                rullst_orm::ModelOperation::Created
            } else {
                rullst_orm::ModelOperation::Updated
            };
            #[cfg(feature = "redis")]
            {
                let event = rullst_orm::ModelCommittedEvent::new(
                    #table_name,
                    self.id,
                    operation,
                    self.to_json(),
                );
                rullst_orm::after_commit(move || async move {
                    use rullst_orm::_redis::AsyncCommands;
                    rullst_orm::query_cache::invalidate_table(event.table).await?;
                    if let Ok(mut connection) = rullst_orm::Orm::redis_manager() {
                        let topic = format!(
                            "orm:events:{}:{}",
                            event.table,
                            event.operation.as_str(),
                        );
                        let _: usize = connection.publish(&topic, &event.payload).await?;
                        let topic = format!("orm:events:{}:saved", event.table);
                        let _: usize = connection.publish(&topic, &event.payload).await?;
                    }
                    Ok(())
                }).await?;
            }
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                operation,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                let futures = observers.iter().map(|observer| observer.committed(&event));
                rullst_orm::_futures::future::try_join_all(futures).await?;
                Ok(())
            }).await?;
            #scout_after_commit
            Ok(())
        }
    }
}

#[cfg_attr(test, mutants::skip)]
pub fn generate_delete_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let tenant_field_type = parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
        .find(|(field, _)| *field == parsed.tenant_column.as_str())
        .map(|(_, ty)| ty);
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

    let hook_before_delete = if !parsed.before_delete.is_empty() {
        let method = syn::Ident::new(&parsed.before_delete, name.span());
        quote! { self.#method().await?; }
    } else {
        quote! {}
    };
    let hook_after_delete = if !parsed.after_delete.is_empty() {
        let method = syn::Ident::new(&parsed.after_delete, name.span());
        quote! { self.#method().await?; }
    } else {
        quote! {}
    };

    let tenant_guard = if let Some(tenant_field_type) = tenant_field_type {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        quote! {
            let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
                rullst_orm::Error::Validation(format!(
                    "tenant context is required to mutate `{}`",
                    #table_name
                ))
            })?;
            let expected_tenant: #tenant_field_type = tenant.try_into().map_err(|_| {
                rullst_orm::Error::Validation(format!(
                    "tenant context type does not match `{}.{}`",
                    #table_name,
                    stringify!(#col_ident)
                ))
            })?;
            if self.#col_ident != expected_tenant {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        }
    } else {
        quote! {}
    };

    let tenant_where_clause = if !parsed.tenant_column.is_empty() {
        format!(" AND {} = ?", parsed.tenant_column)
    } else {
        String::new()
    };
    let tenant_binding = if !parsed.tenant_column.is_empty() {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        quote! { .bind(self.#col_ident.clone()) }
    } else {
        quote! {}
    };
    let tenant_rows_check = if !parsed.tenant_column.is_empty() {
        quote! {
            if mutation_result.rows_affected() != 1 {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        }
    } else {
        quote! {}
    };

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

    let scout_delete_after_commit = if parsed.searchable {
        quote! {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                rullst_orm::ModelOperation::Deleted,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                if let Some(engine) = rullst_orm::scout::get_search_engine() {
                    engine.delete(event.table, event.id).await?;
                }
                Ok(())
            }).await?;
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
        let set_clause_lit = set_clause;
        quote! {
            let driver = rullst_orm::Orm::driver()?;
            let query = if driver == "postgres" {
                let base = format!("UPDATE {} SET {} WHERE id = ?{}", #table_name, #set_clause_lit, #tenant_where_clause);
                rullst_orm::replace_placeholders(&base)
            } else {
                format!("UPDATE {} SET {} WHERE id = ?{}", #table_name, #set_clause_lit, #tenant_where_clause)
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

    let restore_logic = if let Some(cfg) = soft_delete_config {
        let set_clause = if cfg.value.trim().eq_ignore_ascii_case("null") || cfg.value.is_empty() {
            format!("{} = NULL", cfg.column)
        } else {
            format!("{} = {}", cfg.column, cfg.value)
        };
        let set_clause_lit = set_clause;
        quote! {
            let pool = rullst_orm::Orm::try_pool()?;
            use rullst_orm::_sqlx::query_builder::QueryBuilder;
            let mut query_builder = QueryBuilder::new("UPDATE ");
            query_builder.push(#table_name);
            query_builder.push(format!(" SET {} WHERE id = ?{}", #set_clause_lit, #tenant_where_clause));
            let query = query_builder.build();
            let exec = query.bind(self.id) #tenant_binding;
            let mutation_result = rullst_orm::execute_query!(exec, execute, pool)?;
            #tenant_rows_check
        }
    } else {
        quote! {}
    };

    let policy_check_delete = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<Self>>::can_delete(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents deleting this record".to_string()));
            }
        }
    } else {
        quote! {}
    };

    let policy_check_restore = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<Self>>::can_restore(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents restoring this record".to_string()));
            }
        }
    } else {
        quote! {}
    };

    let policy_check_force_delete = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<Self>>::can_force_delete(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents force deleting this record".to_string()));
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

                cascade_deletes_with_tx.extend(quote! {
                    #rel_model::query().where_eq(#fk, self.#lk.clone()).delete_all_with_tx(tx).await?;
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
            let delete_result = async {
                let tx = &mut savepoint;
                self.delete_with_tx_internal(&mut **tx).await?;
                #cascade_deletes_with_tx
                #audit_after_delete_with_tx
                Ok::<(), rullst_orm::Error>(())
            }.await;
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
            rullst_orm::_futures::future::try_join_all(futures).await?;
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
            rullst_orm::_futures::future::try_join_all(futures).await?;
            #hook_after_delete
            #[cfg(feature = "redis")]
            {
                let event = rullst_orm::ModelCommittedEvent::new(
                    #table_name,
                    self.id,
                    rullst_orm::ModelOperation::Deleted,
                    self.to_json(),
                );
                rullst_orm::after_commit(move || async move {
                    use rullst_orm::_redis::AsyncCommands;
                    rullst_orm::query_cache::invalidate_table(event.table).await?;
                    if let Ok(mut connection) = rullst_orm::Orm::redis_manager() {
                        let topic = format!("orm:events:{}:deleted", event.table);
                        let _: usize = connection.publish(&topic, &event.payload).await?;
                    }
                    Ok(())
                }).await?;
            }
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                rullst_orm::ModelOperation::Deleted,
                self.to_json(),
            );
            rullst_orm::after_commit(move || async move {
                let futures = observers.iter().map(|observer| observer.committed(&event));
                rullst_orm::_futures::future::try_join_all(futures).await?;
                Ok(())
            }).await?;
            #scout_delete_after_commit
            Ok(())
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "restore")
        )]
        pub async fn restore(&self) -> Result<(), rullst_orm::Error> {
            #tenant_guard
            #policy_check_restore
            #restore_logic
            Ok(())
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "force_delete")
        )]
        pub async fn force_delete(&self) -> Result<(), rullst_orm::Error> {
            #tenant_guard
            #policy_check_force_delete
            let pool = rullst_orm::Orm::try_pool()?;
            use rullst_orm::_sqlx::query_builder::QueryBuilder;
            let mut query_builder = QueryBuilder::new("DELETE FROM ");
            query_builder.push(#table_name);
            query_builder.push(format!(" WHERE id = ?{}", #tenant_where_clause));
            let query = query_builder.build();
            let exec = query.bind(self.id) #tenant_binding;
            let mutation_result = rullst_orm::execute_query!(exec, execute, pool)?;
            #tenant_rows_check
            Ok(())
        }
    }
}
