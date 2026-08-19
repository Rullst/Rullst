use crate::parser::ParsedModel;
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

    let tenant_set_logic = if !parsed.tenant_column.is_empty() {
        let col_ident = syn::Ident::new(&parsed.tenant_column, name.span());
        quote! {
            if let Some(tenant) = rullst_orm::tenant::get_tenant_id() {
                if let Ok(val) = tenant.try_into() {
                    self.#col_ident = val;
                }
            }
        }
    } else {
        quote! {}
    };

    let audit_before_update = if parsed.auditable {
        quote! {
            let old_model_for_audit = if !is_new {
                let driver = rullst_orm::Orm::driver();
                let query = if driver == "postgres" {
                    format!("SELECT * FROM {} WHERE id = $1", #table_name)
                } else {
                    format!("SELECT * FROM {} WHERE id = ?", #table_name)
                };
                let mut q = rullst_orm::_sqlx::query_as::<_, Self>(rullst_orm::_sqlx::AssertSqlSafe(query.as_str()))
                    .bind(self.id);
                rullst_orm::execute_query!(q, fetch_optional, read_pool)?
            } else {
                None
            };
        }
    } else {
        quote! {}
    };

    let audit_after_save = if parsed.auditable {
        quote! {
            if is_new {
                let _ = rullst_orm::audit::log_audit(
                    #table_name,
                    self.id,
                    "created",
                    None,
                    Some(self.to_json())
                ).await;
            } else if let Some(old_model) = old_model_for_audit {
                let _ = rullst_orm::audit::log_audit_diff(
                    #table_name,
                    self.id,
                    "updated",
                    &old_model.to_json(),
                    &self.to_json()
                ).await;
            }
        }
    } else {
        quote! {}
    };

    let scout_update = if parsed.searchable {
        quote! {
            if let Some(engine) = rullst_orm::scout::get_search_engine() {
                let payload: rullst_orm::_serde_json::Value = match rullst_orm::_serde_json::from_str(&self.to_json()) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[rullst-orm] Scout: failed to serialize model {} (id={}) to JSON: {e}", #table_name, self.id);
                        rullst_orm::_serde_json::Value::Null
                    }
                };
                let _ = engine.update(#table_name, self.id, payload).await;
            }
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
            bind_inserts.push(quote! { .bind(self.#field_name.clone()) });

            update_sets.push(format!("{} = ?", field_name_str));
            bind_updates.push(quote! { .bind(self.#field_name.clone()) });
        }
    }

    let insert_columns_str = insert_columns.join(", ");
    let insert_placeholders_str = insert_placeholders.join(", ");
    let update_sets_str = update_sets.join(", ");

    let policy_check_create = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<Self>>::can_create(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents creation of this record".to_string()));
            }
        }
    } else {
        quote! {}
    };

    let policy_check_update = if !parsed.policy.is_empty() {
        let policy_type = syn::Ident::new(&parsed.policy, parsed.name.span());
        quote! {
            if !<#policy_type as rullst_orm::Policy<Self>>::can_update(self).await? {
                return Err(rullst_orm::Error::Validation("Policy prevents updating this record".to_string()));
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn save(&mut self) -> Result<(), rullst_orm::Error> {
            rullst_orm::dispatch_executor!(pool, |pool| self.save_with_tx_internal(pool).await)
        }

        pub async fn save_with_tx(&mut self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<(), rullst_orm::Error> {
            self.save_with_tx_internal(&mut **tx).await
        }

        async fn save_with_tx_internal<'e, E>(&mut self, executor: E) -> Result<(), rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            let is_new = self.id == 0;
            if is_new {
                #policy_check_create
                #tenant_set_logic
            } else {
                #policy_check_update
            }
            #audit_before_update
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
                let driver = rullst_orm::Orm::driver();
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
                let mut final_sql = format!("UPDATE {} SET {} WHERE id = ?", #table_name, #update_sets_str);
                if rullst_orm::Orm::driver() == "postgres" {
                    final_sql = rullst_orm::replace_placeholders(&final_sql);
                }
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | ID: {}", final_sql, self.id);
                }
                let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(final_sql.as_str()));
                let exec = query #(#bind_updates)*.bind(self.id);
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, exec.execute(executor))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??;
                } else {
                    exec.execute(executor).await?;
                }
                let futures = observers.iter().map(|obs| obs.updated(&*self));
                rullst_orm::_futures::future::try_join_all(futures).await?;
            }
            let futures = observers.iter().map(|obs| obs.saved(&*self));
            rullst_orm::_futures::future::try_join_all(futures).await?;
            #[cfg(feature = "redis")]
            {
                use rullst_orm::_redis::AsyncCommands;
                if let Ok(mut conn) = rullst_orm::Orm::redis_manager() {
                    let payload = self.to_json();
                    if is_new {
                        let topic = format!("orm:events:{}:created", #table_name);
                        let _: Result<usize, _> = conn.publish(&topic, &payload).await;
                    } else {
                        let topic = format!("orm:events:{}:updated", #table_name);
                        let _: Result<usize, _> = conn.publish(&topic, &payload).await;
                    }
                    let topic = format!("orm:events:{}:saved", #table_name);
                    let _: Result<usize, _> = conn.publish(&topic, &payload).await;
                }
            }
            #audit_after_save
            #scout_update
            #hook_after_save
            Ok(())
        }
    }
}

#[cfg_attr(test, mutants::skip)]
pub fn generate_delete_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let has_soft_deletes = parsed.has_soft_deletes;

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

    let audit_after_delete = if parsed.auditable {
        quote! {
            let _ = rullst_orm::audit::log_audit(
                #table_name,
                self.id,
                "deleted",
                Some(self.to_json()),
                None
            ).await;
        }
    } else {
        quote! {}
    };

    let scout_delete = if parsed.searchable {
        quote! {
            if let Some(engine) = rullst_orm::scout::get_search_engine() {
                let _ = engine.delete(#table_name, self.id).await;
            }
        }
    } else {
        quote! {}
    };

    let delete_logic = if has_soft_deletes {
        let cfg = parsed
            .soft_delete
            .as_ref()
            .expect("has_soft_deletes implies a soft_delete config");
        let delval_expr = if cfg.delval.trim().is_empty() {
            "CURRENT_TIMESTAMP".to_string()
        } else {
            cfg.delval.clone()
        };
        let set_clause = format!("{} = {}", cfg.column, delval_expr);
        let set_clause_lit = set_clause;
        quote! {
            let driver = rullst_orm::Orm::driver();
            let query = if driver == "postgres" {
                format!("UPDATE {} SET {} WHERE id = $1", #table_name, #set_clause_lit)
            } else {
                format!("UPDATE {} SET {} WHERE id = ?", #table_name, #set_clause_lit)
            };
        }
    } else {
        quote! {
            let driver = rullst_orm::Orm::driver();
            let query = if driver == "postgres" {
                format!("DELETE FROM {} WHERE id = $1", #table_name)
            } else {
                format!("DELETE FROM {} WHERE id = ?", #table_name)
            };
        }
    };

    let restore_logic = if has_soft_deletes {
        let cfg = parsed
            .soft_delete
            .as_ref()
            .expect("has_soft_deletes implies a soft_delete config");
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
            if rullst_orm::Orm::driver() == "postgres" {
                query_builder.push(format!(" SET {} WHERE id = $1", #set_clause_lit));
            } else {
                query_builder.push(format!(" SET {} WHERE id = ?", #set_clause_lit));
            }
            let query = query_builder.build();
            let mut exec = query.bind(self.id);
            rullst_orm::execute_query!(exec, execute, pool)?;
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

    let mut cascade_deletes = quote! {};
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

                cascade_deletes.extend(quote! {
                    #rel_model::query().where_eq(#fk, self.#lk.clone()).delete_all().await?;
                });
                cascade_deletes_with_tx.extend(quote! {
                    #rel_model::query().where_eq(#fk, self.#lk.clone()).delete_all_with_tx(tx).await?;
                });
            }
        }
    }

    quote! {
        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn delete(&self) -> Result<(), rullst_orm::Error> {
            rullst_orm::dispatch_executor!(pool, |pool| self.delete_with_tx_internal(pool).await)?;
            #cascade_deletes
            Ok(())
        }

        pub async fn delete_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<(), rullst_orm::Error> {
            self.delete_with_tx_internal(&mut **tx).await?;
            #cascade_deletes_with_tx
            Ok(())
        }

        async fn delete_with_tx_internal<'e, E>(&self, executor: E) -> Result<(), rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
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
            let exec = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query.as_str())).bind(self.id);
            let timeout = rullst_orm::schema::get_query_timeout();
            if let Some(t) = timeout {
                tokio::time::timeout(t, exec.execute(executor))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??;
            } else {
                exec.execute(executor).await?;
            }
            let futures = observers.iter().map(|obs| obs.deleted(&*self));
            rullst_orm::_futures::future::try_join_all(futures).await?;
            #[cfg(feature = "redis")]
            {
                use rullst_orm::_redis::AsyncCommands;
                if let Ok(mut conn) = rullst_orm::Orm::redis_manager() {
                    let payload = self.to_json();
                    let topic = format!("orm:events:{}:deleted", #table_name);
                    let _: Result<usize, _> = conn.publish(&topic, &payload).await;
                }
            }
            #audit_after_delete
            #scout_delete
            #hook_after_delete
            Ok(())
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn restore(&self) -> Result<(), rullst_orm::Error> {
            #policy_check_restore
            #restore_logic
            Ok(())
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn force_delete(&self) -> Result<(), rullst_orm::Error> {
            #policy_check_force_delete
            let pool = rullst_orm::Orm::try_pool()?;
            use rullst_orm::_sqlx::query_builder::QueryBuilder;
            let mut query_builder = QueryBuilder::new("DELETE FROM ");
            query_builder.push(#table_name);
            if rullst_orm::Orm::driver() == "postgres" {
                query_builder.push(" WHERE id = $1");
            } else {
                query_builder.push(" WHERE id = ?");
            }
            let query = query_builder.build();
            let mut exec = query.bind(self.id);
            rullst_orm::execute_query!(exec, execute, pool)?;
            Ok(())
        }
    }
}
