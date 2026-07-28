use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate(parsed: &ParsedModel, relationship_methods: &[TokenStream]) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let builder_name = quote::format_ident!("{}QueryBuilder", name);
    let observer_trait_name = quote::format_ident!("{}Observer", name);

    let enum_def = generate_column_enum(parsed);
    let json_methods = generate_json_methods(parsed);
    let search_method = generate_search_method(parsed, &builder_name);
    let save_method = generate_save_method(parsed);
    let delete_methods = generate_delete_methods(parsed);
    let query_methods = generate_query_methods(parsed, &builder_name);
    let (update_builder_struct, update_builder_method) = generate_update_builder(parsed);

    quote! {
        #enum_def
        #update_builder_struct

        #[rullst_orm::async_trait]
        impl rullst_orm::RullstModel for #name {
            fn table_name() -> &'static str {
                #table_name
            }
        }

        impl #name {
            #(#relationship_methods)*

            #json_methods

            pub fn observe(observer: std::sync::Arc<dyn #observer_trait_name + Send + Sync>) {
                let list = Self::observers();
                let mut writer = list.write().unwrap_or_else(|poisoned| poisoned.into_inner());
                writer.push(observer);
            }

            fn observers() -> &'static std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>> {
                static LIST: std::sync::OnceLock<std::sync::RwLock<Vec<std::sync::Arc<dyn #observer_trait_name + Send + Sync>>>> = std::sync::OnceLock::new();
                LIST.get_or_init(|| std::sync::RwLock::new(vec![]))
            }

            #search_method
            #query_methods

            #update_builder_method

            #save_method
            #delete_methods
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_column_enum(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let normal_fields = &parsed.normal_fields;
    let column_enum_name = quote::format_ident!("{}Column", name);

    let column_variants: Vec<_> = normal_fields
        .iter()
        .map(|ident| {
            let name_str = ident.to_string();
            let mut chars = name_str.chars();
            let mut camel = match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            };
            camel = camel
                .split('_')
                .map(|s| {
                    let mut c = s.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect();
            quote::format_ident!("{}", camel)
        })
        .collect();

    let column_to_string: Vec<_> = normal_fields
        .iter()
        .zip(column_variants.iter())
        .map(|(ident, variant)| {
            let field_name_str = ident.to_string();
            quote! { #column_enum_name::#variant => #field_name_str }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum #column_enum_name {
            #(#column_variants),*
        }
        impl #column_enum_name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#column_to_string),*
                }
            }
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_json_methods(parsed: &ParsedModel) -> TokenStream {
    let normal_fields = &parsed.normal_fields;
    let hidden_fields = &parsed.hidden_fields;
    let skipped_fields = &parsed.skipped_fields;
    let mut relation_field_idents = vec![];
    for rel in &parsed.relations {
        relation_field_idents.push(rel.field_name.clone());
    }

    let mut to_json_fields = vec![];
    for field_name in normal_fields {
        let field_name_str = field_name.to_string();
        if !hidden_fields.contains(field_name) {
            to_json_fields.push(quote! {
                map.insert(#field_name_str.to_string(), rullst_orm::_serde_json::json!(self.#field_name));
            });
        }
    }

    let skip_tail = if skipped_fields.is_empty() {
        // No `#[orm(skip)]` / `#[sqlx(skip)]` fields, so the
        // exhaustive struct literal is fine and we don't force the
        // user model to implement `Default`.
        quote! {}
    } else {
        // When a model has skipped fields the struct literal
        // intentionally omits them; trailing `..Default::default()`
        // fills them in. Users must therefore add
        // `#[derive(Default)]` (or implement `Default` manually) on
        // any model that opts into `#[orm(skip)]`.
        quote! { ..Default::default() }
    };

    quote! {
        pub fn from_json(json_str: &str) -> Result<Self, rullst_orm::_serde_json::Error> {
            let value: rullst_orm::_serde_json::Value = rullst_orm::_serde_json::from_str(json_str)?;
            Self::from_json_value(value)
        }

        pub fn from_json_value(value: rullst_orm::_serde_json::Value) -> Result<Self, rullst_orm::_serde_json::Error> {
            Ok(Self {
                #(
                    #normal_fields: rullst_orm::_serde_json::from_value(value[stringify!(#normal_fields)].clone())?,
                )*
                #(
                    #relation_field_idents: None,
                )*
                #skip_tail
            })
        }

        pub fn from_json_array(json_str: &str) -> Result<Vec<Self>, rullst_orm::_serde_json::Error> {
            let value: rullst_orm::_serde_json::Value = rullst_orm::_serde_json::from_str(json_str)?;
            if let rullst_orm::_serde_json::Value::Array(arr) = value {
                let mut results = Vec::with_capacity(arr.len());
                for item in arr {
                    results.push(Self::from_json_value(item)?);
                }
                Ok(results)
            } else {
                Ok(vec![])
            }
        }

        pub fn to_cache_json(&self) -> String {
            let mut map = rullst_orm::_serde_json::Map::new();
            #(
                map.insert(stringify!(#normal_fields).to_string(), rullst_orm::_serde_json::json!(self.#normal_fields));
            )*
            rullst_orm::_serde_json::Value::Object(map).to_string()
        }

        pub fn to_cache_json_array(models: &[Self]) -> String {
            let json_values: Vec<rullst_orm::_serde_json::Value> = models.iter().map(|m| {
                let mut map = rullst_orm::_serde_json::Map::new();
                #(
                    map.insert(stringify!(#normal_fields).to_string(), rullst_orm::_serde_json::json!(m.#normal_fields));
                )*
                rullst_orm::_serde_json::Value::Object(map)
            }).collect();
            rullst_orm::_serde_json::Value::Array(json_values).to_string()
        }

        pub fn from_cache_json(json_str: &str) -> Result<Self, rullst_orm::_serde_json::Error> {
            Self::from_json(json_str)
        }

        pub fn from_cache_json_array(json_str: &str) -> Result<Vec<Self>, rullst_orm::_serde_json::Error> {
            Self::from_json_array(json_str)
        }

        pub fn to_json(&self) -> String {
            let mut map = rullst_orm::_serde_json::Map::new();
            #(#to_json_fields)*
            rullst_orm::_serde_json::Value::Object(map).to_string()
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_search_method(parsed: &ParsedModel, builder_name: &syn::Ident) -> TokenStream {
    if !parsed.searchable {
        return quote! {};
    }
    let table_name = &parsed.table_name;
    let cols = parsed
        .normal_fields
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>();
    quote! {
        pub async fn search(query: &str) -> #builder_name {
            let mut base_builder = #builder_name::new();
            if let Some(engine) = rullst_orm::scout::get_search_engine() {
                let ids = engine.search(#table_name, query).await.unwrap_or_default();
                if ids.is_empty() {
                    base_builder = base_builder.where_eq("id", 0); // impossible match
                } else {
                    let sql_ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
                    base_builder = base_builder.where_raw(format!("id IN ({})", sql_ids).as_str(), vec![] as Vec<rullst_orm::RullstValue>);
                }
                return base_builder;
            }

            let driver = rullst_orm::Orm::driver();
            let cast_type = if driver == "mysql" { "CHAR" } else { "TEXT" };
            let like_query = format!("%{}%", query);
            let cols = vec![#(#cols),*];
            let mut raw_parts: Vec<String> = Vec::with_capacity(cols.len());
            for col in &cols {
                raw_parts.push(format!("CAST({} AS {}) LIKE ?", col, cast_type));
            }
            let raw_where = raw_parts.join(" OR ");
            let mut bindings = Vec::with_capacity(cols.len());
            for _ in &cols {
                bindings.push(rullst_orm::RullstValue::String(like_query.clone()));
            }
            base_builder.where_raw(raw_where.as_str(), bindings)
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_query_methods(parsed: &ParsedModel, builder_name: &syn::Ident) -> TokenStream {
    let global_scope_logic = if !parsed.global_scope.is_empty() {
        let name = &parsed.name;
        let method = syn::Ident::new(&parsed.global_scope, name.span());
        quote! { builder = builder.#method(); }
    } else {
        quote! {}
    };

    let tenant_scope_logic = if !parsed.tenant_column.is_empty() {
        let col = &parsed.tenant_column;
        quote! {
            if let Some(tenant) = rullst_orm::tenant::get_tenant_id() {
                builder = builder.where_eq(#col, tenant);
            }
        }
    } else {
        quote! {}
    };

    quote! {
        pub fn query() -> #builder_name {
            let mut builder = #builder_name::new();
            #global_scope_logic
            #tenant_scope_logic
            builder
        }

        pub async fn find(id: i32) -> Result<Option<Self>, rullst_orm::Error> {
            Self::query().where_eq("id", id).first().await
        }

        pub async fn find_with_tx(id: i32, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Option<Self>, rullst_orm::Error> {
            Self::query().where_eq("id", id).first_with_tx(tx).await
        }

        pub async fn all() -> Result<Vec<Self>, rullst_orm::Error> {
            Self::query().get().await
        }

        pub async fn all_with_tx(tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Vec<Self>, rullst_orm::Error> {
            Self::query().get_with_tx(tx).await
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_save_method(parsed: &ParsedModel) -> TokenStream {
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
fn generate_delete_methods(parsed: &ParsedModel) -> TokenStream {
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

    // Compose the soft delete UPDATE statement. The column name and
    // the `delval` expression both come from the user config so the
    // same generated SQL works against MySQL (`now()`,
    // `UNIX_TIMESTAMP()`), PostgreSQL (`now() :: timestamp`) and
    // SQLite (`CURRENT_TIMESTAMP`).
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
        // Build the SET fragment as a runtime string so the
        // user-supplied `delval` is interpolated verbatim rather than
        // being reparsed as Rust tokens (which would mistake
        // `now()` for an unresolved function call).
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

    // Build the restore SQL: flip the soft delete column back to its
    // "not deleted" sentinel. If the user configured the sentinel as
    // `null` we emit `<column> = NULL`; otherwise we use the literal
    // value they provided.
    let restore_logic = if has_soft_deletes {
        let cfg = parsed
            .soft_delete
            .as_ref()
            .expect("has_soft_deletes implies a soft_delete config");
        let set_clause = if cfg.value.trim().eq_ignore_ascii_case("null") {
            format!("{} = NULL", cfg.column)
        } else if cfg.value.is_empty() {
            // No explicit value -> fall back to the legacy NULL default.
            format!("{} = NULL", cfg.column)
        } else {
            format!("{} = {}", cfg.column, cfg.value)
        };
        let set_clause_lit = set_clause;
        quote! {
            let pool = rullst_orm::Orm::pool();
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
            if rel.cascade_soft_delete && (rel.rel_type == "has_many" || rel.rel_type == "has_one") {
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
            let pool = rullst_orm::Orm::pool();
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

#[cfg_attr(test, mutants::skip)]
fn generate_update_builder(parsed: &ParsedModel) -> (TokenStream, TokenStream) {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let update_builder_name = quote::format_ident!("{}UpdateBuilder", name);
    let normal_fields = &parsed.normal_fields;
    let normal_fields_types = &parsed.normal_fields_types;

    let mut builder_fields = vec![];
    let mut builder_methods = vec![];
    let mut set_clauses = vec![];
    let mut update_bindings = vec![];
    let mut apply_to_model = vec![];
    let mut builder_inits = vec![];

    for (field, ty) in normal_fields.iter().zip(normal_fields_types.iter()) {
        if field == "id" {
            continue;
        }

        builder_fields.push(quote! {
            #field: Option<#ty>
        });

        builder_inits.push(quote! {
            #field: None
        });

        builder_methods.push(quote! {
            pub fn #field(mut self, value: #ty) -> Self {
                self.#field = Some(value);
                self
            }
        });

        let field_str = field.to_string();
        set_clauses.push(quote! {
            if self.#field.is_some() {
                sets.push(format!("{} = ?", #field_str));
            }
        });

        update_bindings.push(quote! {
            if let Some(ref val) = self.#field {
                exec = exec.bind(val.clone());
            }
        });

        apply_to_model.push(quote! {
            if let Some(ref val) = self.#field {
                self.model.#field = val.clone();
            }
        });
    }

    let struct_def = quote! {
        pub struct #update_builder_name<'a> {
            model: &'a mut #name,
            #(#builder_fields),*
        }

        impl<'a> #update_builder_name<'a> {
            #(#builder_methods)*

            pub async fn save(mut self) -> Result<(), rullst_orm::Error> {
                let mut sets = vec![];
                #(#set_clauses)*

                if sets.is_empty() {
                    return Ok(()); // Nothing to update
                }

                #(#apply_to_model)*

                let driver = rullst_orm::Orm::driver();
                let mut sql = format!("UPDATE {} SET {} WHERE id = ?", #table_name, sets.join(", "));
                if driver == "postgres" {
                    sql = rullst_orm::replace_placeholders(&sql);
                }

                if rullst_orm::schema::is_query_log_enabled() {
                    println!("[SQL Debug Partial Update] {:?} | ID: {}", sql, self.model.id);
                }

                let pool = rullst_orm::Orm::pool();
                let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
                let mut exec = query;

                #(#update_bindings)*

                exec = exec.bind(self.model.id);
                rullst_orm::execute_query!(exec, execute, pool)?;

                Ok(())
            }
        }
    };

    let method_def = quote! {
        pub fn update_partial(&mut self) -> #update_builder_name<'_> {
            #update_builder_name {
                model: self,
                #(#builder_inits),*
            }
        }
    };

    (struct_def, method_def)
}
