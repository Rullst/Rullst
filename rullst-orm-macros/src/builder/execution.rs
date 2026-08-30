// src/builder/execution.rs — Terminal database execution, pagination, streaming, and mutation methods.

use crate::parser::{EncryptedFieldKind, ParsedModel, SoftDeleteConfig};
use proc_macro2::TokenStream;
use quote::quote;

/// Build the SET clause template for soft delete updates. The string
/// `{VALUE}` is replaced at codegen with the actual `delval` SQL
/// fragment.
#[cfg_attr(test, mutants::skip)]
pub fn build_soft_delete_set_clause(cfg: &SoftDeleteConfig) -> String {
    format!("{} = {{VALUE}}", cfg.column)
}

#[cfg_attr(test, mutants::skip)]
pub fn generate_delete_all_logic(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    if !parsed.has_soft_deletes {
        return quote! {
            let mut estimated_capacity = 20 + #table_name.len() + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>();
            let mut query_str = String::with_capacity(estimated_capacity);
            query_str.push_str("DELETE FROM ");
            query_str.push_str(#table_name);
        };
    }
    let Some(cfg) = parsed.soft_delete.as_ref() else {
        return syn::Error::new(
            parsed.name.span(),
            "internal ORM macro error: soft-delete configuration is missing",
        )
        .to_compile_error();
    };
    let set_fragment = build_soft_delete_set_clause(cfg);
    let delval_token: TokenStream = if cfg.delval.trim().is_empty() {
        quote! {
            let delval = if rullst_orm::Orm::driver()? == "postgres" {
                "CURRENT_TIMESTAMP"
            } else {
                "CURRENT_TIMESTAMP"
            };
        }
    } else {
        let delval_lit = cfg.delval.clone();
        quote! {
            let delval = #delval_lit;
        }
    };
    let set_template = set_fragment;
    let table_lit = table_name.clone();
    quote! {
        #delval_token
        let mut estimated_capacity = 50 + #table_lit.len() + delval.len() + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>();
        let mut query_str = String::with_capacity(estimated_capacity);
        query_str.push_str("UPDATE ");
        query_str.push_str(#table_lit);
        query_str.push_str(" SET ");
        query_str.push_str(#set_template.replace("{VALUE}", delval).as_str());
    }
}

#[cfg_attr(test, mutants::skip)]
pub fn generate_execution_methods(
    parsed: &ParsedModel,
    _builder_name: &syn::Ident,
    eager_loads: &TokenStream,
) -> Vec<TokenStream> {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let hook_after_fetch = if !parsed.after_fetch.is_empty() {
        let method = syn::Ident::new(&parsed.after_fetch, name.span());
        quote! {
            let futures = results.iter_mut().map(|model| model.#method());
            rullst_orm::_futures::future::try_join_all(futures).await?;
        }
    } else {
        quote! {}
    };
    let hook_after_fetch_single = if !parsed.after_fetch.is_empty() {
        let method = syn::Ident::new(&parsed.after_fetch, name.span());
        quote! { row.#method().await?; }
    } else {
        quote! {}
    };
    let decrypt_results = if parsed.encrypted_fields.is_empty() {
        quote! {}
    } else {
        quote! {
            for model in &mut results {
                model.__rullst_decrypt_encrypted_fields()?;
            }
        }
    };
    let decrypt_row = if parsed.encrypted_fields.is_empty() {
        quote! {}
    } else {
        quote! { row.__rullst_decrypt_encrypted_fields()?; }
    };
    let encrypted_string_columns = parsed
        .encrypted_fields
        .iter()
        .filter(|field| field.kind == EncryptedFieldKind::String)
        .map(|field| field.name.to_string())
        .collect::<Vec<_>>();
    let encrypted_optional_columns = parsed
        .encrypted_fields
        .iter()
        .filter(|field| field.kind == EncryptedFieldKind::OptionalString)
        .map(|field| field.name.to_string())
        .collect::<Vec<_>>();
    let delete_all_logic = generate_delete_all_logic(parsed);

    vec![quote! {
        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn get(&self) -> Result<Vec<#name>, rullst_orm::Error> {
            rullst_orm::dispatch_executor!(read_pool, |pool| self.get_with_tx_internal(pool).await)
        }

        pub async fn get_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Vec<#name>, rullst_orm::Error> {
            self.get_with_tx_internal(&mut **tx).await
        }

        async fn get_with_tx_internal<'e, E>(&self, executor: E) -> Result<Vec<#name>, rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let query_str = self.to_sql();

            #[cfg(feature = "redis")]
            {
                if let Some(ttl) = self.remember_ttl {
                    use rullst_orm::_redis::AsyncCommands;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    query_str.hash(&mut hasher);
                    for binding in &self.bindings {
                        match binding {
                            rullst_orm::RullstValue::String(s) => s.hash(&mut hasher),
                            rullst_orm::RullstValue::Int(i) => i.hash(&mut hasher),
                            rullst_orm::RullstValue::Float(f) => f.to_bits().hash(&mut hasher),
                            rullst_orm::RullstValue::Bool(b) => b.hash(&mut hasher),
                        }
                    }
                    let cache_key = format!("orm:cache:{}:{:x}", #table_name, hasher.finish());
                    let mut conn = rullst_orm::Orm::redis_manager()?;
                    if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                        if !cached_data.is_empty() {
                            if let Ok(mut results) = #name::from_cache_json_array(&cached_data) {
                                #decrypt_results
                                #hook_after_fetch
                                #eager_loads
                                return Ok(results);
                            }
                        }
                    }
                }
            }

            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
            }
            let mut results: Vec<#name> = {
                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.fetch_all(executor))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.fetch_all(executor).await?
                }
            };

            #[cfg(feature = "redis")]
            {
                if let Some(ttl) = self.remember_ttl {
                    use rullst_orm::_redis::AsyncCommands;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    query_str.hash(&mut hasher);
                    for binding in &self.bindings {
                        match binding {
                            rullst_orm::RullstValue::String(s) => s.hash(&mut hasher),
                            rullst_orm::RullstValue::Int(i) => i.hash(&mut hasher),
                            rullst_orm::RullstValue::Float(f) => f.to_bits().hash(&mut hasher),
                            rullst_orm::RullstValue::Bool(b) => b.hash(&mut hasher),
                        }
                    }
                    let cache_key = format!("orm:cache:{}:{:x}", #table_name, hasher.finish());
                    let serialized = #name::to_cache_json_array(&results);
                    let mut conn = rullst_orm::Orm::redis_manager()?;
                    let _: Result<(), rullst_orm::_redis::RedisError> = conn.set_ex(&cache_key, serialized, ttl as u64).await;
                }
            }

            #decrypt_results
            #hook_after_fetch
            #eager_loads
            Ok(results)
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn first(&self) -> Result<Option<#name>, rullst_orm::Error> {
            let mut builder = self.clone();
            builder.limit = Some(1);
            let results = builder.get().await?;
            Ok(results.into_iter().next())
        }

        pub async fn first_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Option<#name>, rullst_orm::Error> {
            let mut builder = self.clone();
            builder.limit = Some(1);
            let results = builder.get_with_tx(tx).await?;
            Ok(results.into_iter().next())
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn paginate(&self, page: usize, per_page: usize) -> Result<rullst_orm::PaginationResult<#name>, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            if per_page == 0 {
                return Err(rullst_orm::Error::Validation(
                    "paginate() requires per_page greater than zero".to_string()
                ));
            }
            let current_page = page.max(1);
            let mut total_builder = self.clone();
            total_builder.selects = Some("COUNT(*)".to_string());
            total_builder.limit = None;
            total_builder.offset = None;
            total_builder.order_by = None;

            let query_str = total_builder.to_sql();
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, total_builder.bindings.len());
            }
            let total_row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &total_builder.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.fetch_one(pool))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.fetch_one(pool).await?
                }
            });
            let total = total_row.0;
            let total_for_pages = usize::try_from(total).map_err(|_| {
                rullst_orm::Error::DatabaseError(
                    "paginate() received a negative or unsupported row count".to_string()
                )
            })?;
            let last_page = total_for_pages.div_ceil(per_page);

            let mut data_builder = self.clone();
            data_builder.limit = Some(per_page);
            if current_page > 1 {
                data_builder.offset = Some(
                    (current_page - 1).checked_mul(per_page).ok_or_else(|| {
                        rullst_orm::Error::Validation(
                            "paginate() page offset exceeds the supported range".to_string()
                        )
                    })?
                );
            }
            let data = data_builder.get().await?;

            Ok(rullst_orm::PaginationResult {
                data,
                total,
                per_page,
                current_page,
                last_page,
            })
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn count(&self) -> Result<i64, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let query_str = self.to_count_sql();
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
            }
            let row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.fetch_one(pool))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.fetch_one(pool).await?
                }
            });
            Ok(row.0)
        }

        pub fn stream<'a>(&'a self) -> impl rullst_orm::_futures::Stream<Item = Result<#name, rullst_orm::Error>> + 'a {
            rullst_orm::_async_stream::try_stream! {
                if !self.errors.is_empty() {
                    Err(self.errors[0].clone())?;
                }
                let query_str = self.to_sql();
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                }

                let pool = rullst_orm::Orm::try_read_pool()?;
                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }

                let mut db_stream = query.fetch(pool);
                while let Some(row_res) = rullst_orm::_futures::StreamExt::next(&mut db_stream).await {
                    let mut row = row_res.map_err(|e| rullst_orm::Error::DatabaseError(e.to_string()))?;
                    #decrypt_row
                    #hook_after_fetch_single
                    yield row;
                }
            }
        }

        pub fn stream_with_tx<'a>(&'a self, tx: &'a mut rullst_orm::db::Transaction<'static>) -> impl rullst_orm::_futures::Stream<Item = Result<#name, rullst_orm::Error>> + 'a {
            rullst_orm::_async_stream::try_stream! {
                if !self.errors.is_empty() {
                    Err(self.errors[0].clone())?;
                }
                let query_str = self.to_sql();
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                }

                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }

                let mut db_stream = query.fetch(&mut **tx);
                while let Some(row_res) = rullst_orm::_futures::StreamExt::next(&mut db_stream).await {
                    let mut row = row_res.map_err(|e| rullst_orm::Error::DatabaseError(e.to_string()))?;
                    #decrypt_row
                    #hook_after_fetch_single
                    yield row;
                }
            }
        }

        #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
        pub async fn delete_all(&self) -> Result<u64, rullst_orm::Error> {
            rullst_orm::dispatch_executor!(pool, |pool| self.delete_all_with_tx_internal(pool).await)
        }

        pub async fn delete_all_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<u64, rullst_orm::Error> {
            self.delete_all_with_tx_internal(&mut **tx).await
        }

        async fn delete_all_with_tx_internal<'e, E>(&self, executor: E) -> Result<u64, rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            #delete_all_logic

            if !self.wheres.is_empty() {
                query_str.push_str(" WHERE ");
                let mut first = true;
                for (operator, condition) in &self.wheres {
                    if first {
                        query_str.push('(');
                        query_str.push_str(condition);
                        query_str.push(')');
                        first = false;
                    } else {
                        query_str.push(' ');
                        query_str.push_str(operator);
                        query_str.push_str(" (");
                        query_str.push_str(condition);
                        query_str.push(')');
                    }
                }
            }
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
            }
            let query_str = rullst_orm::replace_placeholders(&query_str);
            let result = {
                let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.execute(executor))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.execute(executor).await?
                }
            };
            Ok(result.rows_affected())
        }

        pub async fn pluck_string(&self, column: &str) -> Result<Vec<String>, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let pool = rullst_orm::Orm::try_read_pool()?;
            const ENCRYPTED_STRING_COLUMNS: &[&str] = &[#(#encrypted_string_columns),*];
            const ENCRYPTED_OPTIONAL_COLUMNS: &[&str] = &[#(#encrypted_optional_columns),*];
            if ENCRYPTED_OPTIONAL_COLUMNS.contains(&column) {
                return Err(rullst_orm::Error::Validation(format!(
                    "pluck_string() cannot decode nullable encrypted column `{}`; load the model instead",
                    column
                )));
            }
            let is_encrypted = ENCRYPTED_STRING_COLUMNS.contains(&column);
            let query_str = self.to_pluck_sql(column);
            let rows: Vec<(String,)> = {
                let mut query = rullst_orm::_sqlx::query_as::<_, (String,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.fetch_all(pool))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.fetch_all(pool).await?
                }
            };
            if is_encrypted {
                rows.into_iter()
                    .map(|(value,)| rullst_orm::privacy::decrypt_model_field(&value, #table_name, column).map_err(Into::into))
                    .collect()
            } else {
                Ok(rows.into_iter().map(|(value,)| value).collect())
            }
        }

        pub async fn pluck_i32(&self, column: &str) -> Result<Vec<i32>, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let pool = rullst_orm::Orm::try_read_pool()?;
            let query_str = self.to_pluck_sql(column);
            let rows: Vec<(i32,)> = {
                let mut query = rullst_orm::_sqlx::query_as::<_, (i32,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &self.bindings {
                    match binding {
                        rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                        rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                        rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                        rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                    }
                }
                let timeout = rullst_orm::schema::get_query_timeout();
                if let Some(t) = timeout {
                    tokio::time::timeout(t, query.fetch_all(pool))
                        .await
                        .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                } else {
                    query.fetch_all(pool).await?
                }
            };
            Ok(rows.into_iter().map(|(s,)| s).collect())
        }
    }]
}
