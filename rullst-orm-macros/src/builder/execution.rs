// src/builder/execution.rs — Terminal database execution, pagination, streaming, and mutation methods.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

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
    let pluck_methods = super::pluck::generate_pluck_methods(parsed);
    let bulk_delete = super::bulk_delete::generate_delete_all_methods(parsed);
    let has_after_fetch = !parsed.after_fetch.is_empty();
    let eager_flags = parsed
        .relations
        .iter()
        .map(|relation| quote::format_ident!("load_{}", relation.field_name));
    let explicit_read_validation = quote! {
        if #has_after_fetch #(|| self.#eager_flags)* {
            return Err(rullst_orm::Error::Validation(
                "eager loading and after_fetch hooks require Orm::transaction(...) with get(); a caller-owned raw transaction cannot provide their task-scoped executor".to_string()
            ));
        }
    };
    let cache_read = super::query_cache::generate_cache_read(name, table_name, &decrypt_results);
    let cache_write = super::query_cache::generate_cache_write(name);

    vec![quote! {
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "select_many")
        )]
        pub async fn get(&self) -> Result<Vec<#name>, rullst_orm::Error> {
            rullst_orm::__transaction_access::ensure_allowed()?;
            let mut results = if let Ok(tx_arc) = rullst_orm::CURRENT_TX.try_with(|tx| tx.clone()) {
                let mut tx_guard = tx_arc.lock().await;
                if let Some(tx) = tx_guard.as_mut() {
                    self.get_with_tx_internal(&mut **tx, false).await?
                } else {
                    let pool = rullst_orm::Orm::read_pool()?;
                    self.get_with_tx_internal(pool, true).await?
                }
            } else {
                let pool = rullst_orm::Orm::read_pool()?;
                self.get_with_tx_internal(pool, true).await?
            };
            // Nested queries must acquire the transaction independently, after
            // the fetch releases its mutex guard.
            #hook_after_fetch
            #eager_loads
            Ok(results)
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, tx),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "select_many_with_tx")
        )]
        pub async fn get_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<Vec<#name>, rullst_orm::Error> {
            #explicit_read_validation
            self.get_with_tx_internal(&mut **tx, false).await
        }

        async fn get_with_tx_internal<'e, E>(&self, executor: E, _allow_cache: bool) -> Result<Vec<#name>, rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let query_str = self.to_sql();
            let query_bindings = self.select_bindings();

            #cache_read

            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, query_bindings.len());
            }
            let mut results: Vec<#name> = {
                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &query_bindings {
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

            #cache_write

            #decrypt_results
            Ok(results)
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "select_first")
        )]
        pub async fn first(&self) -> Result<Option<#name>, rullst_orm::Error> {
            let mut builder = self.clone();
            builder.limit = Some(1);
            let results = builder.get().await?;
            Ok(results.into_iter().next())
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, tx),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "select_first_with_tx")
        )]
        pub async fn first_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<Option<#name>, rullst_orm::Error> {
            let mut builder = self.clone();
            builder.limit = Some(1);
            let results = builder.get_with_tx(tx).await?;
            Ok(results.into_iter().next())
        }

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "paginate")
        )]
        pub async fn paginate(&self, page: usize, per_page: usize) -> Result<rullst_orm::PaginationResult<#name>, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            if per_page == 0 {
                return Err(rullst_orm::Error::Validation(
                    "paginate() requires per_page greater than zero".to_string()
                ));
            }
            // Request-supplied `per_page` obeys the same global row cap as `limit()`.
            let per_page = match rullst_orm::schema::get_max_query_limit() {
                Some(max_limit) => per_page.min(max_limit),
                None => per_page,
            };
            let current_page = page.max(1);
            let mut total_builder = self.clone();
            total_builder.selects = Some("COUNT(*)".to_string());
            total_builder.limit = None;
            total_builder.offset = None;
            total_builder.order_by = None;

            let query_str = total_builder.to_sql();
            let count_bindings = total_builder.count_bindings();
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, count_bindings.len());
            }
            let total_row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &count_bindings {
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

        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self),
            fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "count")
        )]
        pub async fn count(&self) -> Result<i64, rullst_orm::Error> {
            if !self.errors.is_empty() {
                return Err(self.errors[0].clone());
            }
            let query_str = self.to_count_sql();
            let count_bindings = self.count_bindings();
            if rullst_orm::schema::is_query_log_enabled() {
                println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, count_bindings.len());
            }
            let row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &count_bindings {
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
            rullst_orm::telemetry::instrument_query_stream(
                rullst_orm::_async_stream::try_stream! {
                rullst_orm::__transaction_access::ensure_allowed()?;
                if !self.errors.is_empty() {
                    Err(self.errors[0].clone())?;
                }
                if let Ok(tx_arc) = rullst_orm::CURRENT_TX.try_with(|tx| tx.clone()) {
                    let mut tx_guard = tx_arc.lock().await;
                    if let Some(tx) = tx_guard.as_mut() {
                        // The guard stays locked between rows: other ORM calls
                        // on this transaction fail fast instead of waiting.
                        let _stream_hold = rullst_orm::__transaction_access::hold_for_stream(&tx_arc);
                        let stream = self.stream_with_tx(tx);
                        rullst_orm::_futures::pin_mut!(stream);
                        while let Some(row) = rullst_orm::_futures::StreamExt::next(&mut stream).await {
                            yield row?;
                        }
                        return;
                    }
                }
                let query_str = self.to_sql();
                let query_bindings = self.select_bindings();
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, query_bindings.len());
                }

                let pool = rullst_orm::Orm::try_read_pool()?;
                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &query_bindings {
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
                },
                stringify!(#name),
                #table_name,
                "stream",
            )
        }

        pub fn stream_with_tx<'a>(&'a self, tx: &'a mut rullst_orm::db::Transaction<'static>) -> impl rullst_orm::_futures::Stream<Item = Result<#name, rullst_orm::Error>> + 'a {
            rullst_orm::telemetry::instrument_query_stream(
                rullst_orm::_async_stream::try_stream! {
                if !self.errors.is_empty() {
                    Err(self.errors[0].clone())?;
                }
                if #has_after_fetch {
                    Err(rullst_orm::Error::Validation(
                        "transactional streams cannot run after_fetch hooks while retaining the transaction; use get() inside Orm::transaction(...)".to_string()
                    ))?;
                }
                let query_str = self.to_sql();
                let query_bindings = self.select_bindings();
                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, query_bindings.len());
                }

                let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                for binding in &query_bindings {
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
                },
                stringify!(#name),
                #table_name,
                "stream_with_tx",
            )
        }

        #bulk_delete

        #pluck_methods
    }]
}
