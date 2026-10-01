use crate::models::mutation_parts::saved_redis_effects;
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
        quote! { rullst_orm::__transaction_access::run(self.#method()).await?; }
    } else {
        quote! {}
    };
    let hook_after_save = if !parsed.after_save.is_empty() {
        let method = syn::Ident::new(&parsed.after_save, name.span());
        quote! { rullst_orm::__transaction_access::run(self.#method()).await?; }
    } else {
        quote! {}
    };

    let scout_after_commit = if parsed.searchable {
        quote! {
            let event = rullst_orm::ModelCommittedEvent::new(
                #table_name,
                self.id,
                operation,
                self.__rullst_search_json(),
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
    // Only delete()/restore()/force_delete() change the soft-delete marker; a
    // stale handle must not undelete (or delete) its row through save().
    let soft_delete_column = parsed.soft_delete_column();

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

            if soft_delete_column != Some(field_name_str.as_str()) {
                update_sets.push(format!("{} = ?", field_name_str));
                bind_updates.push(binding);
            }
        }
    }
    if update_sets.is_empty() {
        update_sets.push("id = id".to_string());
    }

    // A model whose only persisted column is `id` inserts the defaults: no
    // dialect accepts an empty column list followed by `VALUES ()` except
    // MySQL/MariaDB, which in turn rejects `DEFAULT VALUES`.
    let (insert_returning_sql, insert_sql) = if insert_columns.is_empty() {
        (
            format!("INSERT INTO {table_name} DEFAULT VALUES RETURNING id"),
            format!("INSERT INTO {table_name} () VALUES ()"),
        )
    } else {
        let columns = insert_columns.join(", ");
        let placeholders = insert_placeholders.join(", ");
        (
            format!("INSERT INTO {table_name} ({columns}) VALUES ({placeholders}) RETURNING id"),
            format!("INSERT INTO {table_name} ({columns}) VALUES ({placeholders})"),
        )
    };
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
    // An UPDATE that matches no row (a row deleted since it was loaded) must
    // not report a successful save or run the update observers and effects.
    let update_tenant_result_check = if !parsed.tenant_column.is_empty() {
        quote! {
            if update_result.rows_affected() != 1 {
                return Err(rullst_orm::Error::Validation(
                    "record is outside the active tenant scope".to_string()
                ));
            }
        }
    } else {
        quote! {
            if update_result.rows_affected() == 0 {
                return Err(rullst_orm::Error::RecordNotFound);
            }
        }
    };

    let save_entrypoints = save_entrypoints::generate(parsed);
    let saved_redis_effects = saved_redis_effects(parsed);

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
                rullst_orm::__transaction_access::run(obs.saving(self)).await?;
            }
            if self.id == 0 {
                for obs in &observers {
                    rullst_orm::__transaction_access::run(obs.creating(self)).await?;
                }
                let driver = rullst_orm::Orm::driver()?;
                if driver == "postgres" || driver == "sqlite" {
                    use rullst_orm::_sqlx::Execute;
                    let final_sql = if driver == "postgres" {
                        rullst_orm::replace_placeholders(#insert_returning_sql)
                    } else {
                        #insert_returning_sql.to_string()
                    };
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
                    let final_sql = #insert_sql.to_string();
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
                rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            } else {
                for obs in &observers {
                    rullst_orm::__transaction_access::run(obs.updating(self)).await?;
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
                rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            }
            let futures = observers.iter().map(|obs| obs.saved(&*self));
            rullst_orm::__transaction_access::run(rullst_orm::_futures::future::try_join_all(futures)).await?;
            #hook_after_save
            let operation = if is_new {
                rullst_orm::ModelOperation::Created
            } else {
                rullst_orm::ModelOperation::Updated
            };
            #saved_redis_effects
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
