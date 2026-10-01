use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_search_method(parsed: &ParsedModel, builder_name: &syn::Ident) -> TokenStream {
    if !parsed.searchable {
        return quote! {};
    }
    let table_name = &parsed.table_name;
    // Hidden, encrypted, masked and `SecretString` columns never take part in
    // the SQL fallback, so it cannot act as a substring oracle for them.
    let cols = parsed
        .normal_fields
        .iter()
        .filter(|field| !parsed.hidden_fields.contains(field) && !parsed.is_redacted(field))
        .map(|f| f.to_string())
        .collect::<Vec<_>>();
    // Providers truncate hits over the whole shared index before this model's
    // tenant, global and soft-delete scopes apply, so a capped answer may hold
    // none of the scoped matches. Such a model then answers from SQL.
    let scoped = !parsed.tenant_column.is_empty()
        || !parsed.global_scope.is_empty()
        || parsed.has_soft_deletes;
    // The engine's ranking is kept as the default order of the matches.
    let engine_result = if scoped {
        quote! {
            if ids.len() < rullst_orm::scout::MAX_SEARCH_HITS {
                return base_builder.__rullst_order_by_relevance(&ids).where_in("id", ids);
            }
        }
    } else {
        quote! { return base_builder.__rullst_order_by_relevance(&ids).where_in("id", ids); }
    };
    quote! {
        pub async fn search(query: &str) -> #builder_name {
            let mut base_builder = Self::query();
            if !base_builder.errors.is_empty() {
                return base_builder;
            }
            if let Some(engine) = rullst_orm::scout::get_search_engine() {
                let ids = match engine.search(#table_name, query).await {
                    Ok(ids) => ids,
                    Err(error) => {
                        base_builder.errors.push(error);
                        return base_builder;
                    }
                };
                #engine_result
            }

            let driver = match rullst_orm::Orm::driver() {
                Ok(driver) => driver,
                Err(error) => {
                    base_builder.errors.push(error);
                    return base_builder;
                }
            };
            let cast_type = if driver == "mysql" { "CHAR" } else { "TEXT" };
            let pattern = match rullst_orm::scout::sql_contains_pattern(query) {
                Ok(pattern) => pattern,
                Err(error) => {
                    base_builder.errors.push(error);
                    return base_builder;
                }
            };
            let cols: &[&str] = &[#(#cols),*];
            if cols.is_empty() {
                return base_builder.where_raw::<rullst_orm::RullstValue>("1 = 0", Vec::new());
            }
            let mut raw_parts: Vec<String> = Vec::with_capacity(cols.len());
            for col in cols {
                raw_parts.push(format!("CAST({} AS {}) LIKE ? ESCAPE '!'", col, cast_type));
            }
            let raw_where = raw_parts.join(" OR ");
            let bindings = cols
                .iter()
                .map(|_| rullst_orm::RullstValue::String(pattern.clone()))
                .collect::<Vec<_>>();
            base_builder.where_raw(raw_where.as_str(), bindings)
        }
    }
}

#[cfg_attr(test, mutants::skip)]
pub fn generate_query_methods(parsed: &ParsedModel, builder_name: &syn::Ident) -> TokenStream {
    let table_name = &parsed.table_name;
    let global_scope_logic = if !parsed.global_scope.is_empty() {
        let name = &parsed.name;
        let method = syn::Ident::new(&parsed.global_scope, name.span());
        quote! { builder = builder.#method(); }
    } else {
        quote! {}
    };

    let tenant_field_type = parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
        .find(|(field, _)| *field == parsed.tenant_column.as_str())
        .map(|(_, ty)| ty);
    let tenant_scope_logic = if let Some(tenant_type) = tenant_field_type {
        let col = &parsed.tenant_column;
        // Bind the context as the tenant field's type, like the mutation
        // paths: a mistyped context must not reach the mandatory predicate,
        // where MySQL would compare it numerically across tenants.
        quote! {
            if let Some(tenant) = rullst_orm::tenant::get_tenant_id() {
                let typed: Result<#tenant_type, _> = tenant.try_into();
                match typed {
                    Ok(tenant) => builder = builder.where_eq(#col, tenant),
                    Err(_) => builder.errors.push(rullst_orm::Error::Validation(format!(
                        "tenant context type does not match `{}.{}`",
                        #table_name,
                        #col
                    ))),
                }
            } else {
                builder.errors.push(rullst_orm::Error::Validation(format!(
                    "tenant context is required to query `{}`; use with_tenant(...) or the explicit unscoped() escape hatch",
                    #table_name
                )));
            }
        }
    } else if parsed.tenant_column.is_empty() {
        quote! {}
    } else {
        // The structured model parser rejects this before code generation.
        quote! { compile_error!("tenant column is missing from the model"); }
    };

    quote! {
        pub fn query() -> #builder_name {
            let mut builder = #builder_name::new();
            #global_scope_logic
            builder.freeze_scope();
            #tenant_scope_logic
            builder.freeze_scope();
            builder
        }

        /// Builds the child query of a parent's `cascade_soft_delete`: the
        /// mandatory tenant scope applies, the model-wide `global_scope`
        /// does not, so every direct child is trashed with its parent.
        #[doc(hidden)]
        pub fn __rullst_cascade_query() -> #builder_name {
            let mut builder = #builder_name::new();
            #tenant_scope_logic
            builder.freeze_scope();
            builder
        }

        /// Builds a query without model-wide or tenant scopes.
        ///
        /// This deliberately noisy escape hatch is intended for reviewed
        /// administrative and maintenance paths. Application request handlers
        /// should normally use [`rullst_orm::with_tenant`] and [`Self::query`].
        pub fn unscoped() -> #builder_name {
            #builder_name::new()
        }

        pub async fn find(id: i32) -> Result<Option<Self>, rullst_orm::Error> {
            Self::query().where_eq("id", id).first().await
        }

        pub async fn find_with_tx(id: i32, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<Option<Self>, rullst_orm::Error> {
            Self::query().where_eq("id", id).first_with_tx(tx).await
        }

        pub async fn all() -> Result<Vec<Self>, rullst_orm::Error> {
            Self::query().get().await
        }

        pub async fn all_with_tx(tx: &mut rullst_orm::db::Transaction<'_>) -> Result<Vec<Self>, rullst_orm::Error> {
            Self::query().get_with_tx(tx).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn cascade_query_keeps_the_tenant_scope_but_not_the_global_scope() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "notes", tenant_column = "org", global_scope = "approved")]
            struct Note { id: i32, org: String, deleted_at: Option<String> }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let builder = quote::format_ident!("NoteQueryBuilder");
        let generated = generate_query_methods(&parsed, &builder).to_string();
        let (query, cascade) = generated
            .split_once("fn __rullst_cascade_query")
            .expect("cascade query constructor");
        assert!(query.contains("builder = builder . approved ()"));
        let cascade = cascade
            .split_once("fn unscoped")
            .expect("cascade constructor body")
            .0;
        assert!(!cascade.contains("approved"));
        assert!(cascade.contains("where_eq (\"org\" , tenant)"));
        assert!(cascade.contains("tenant context is required"));
    }

    #[test]
    fn sql_search_fallback_skips_hidden_and_protected_columns() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "accounts", searchable)]
            struct Account {
                id: i32,
                name: String,
                #[orm(hidden)]
                reset_token: String,
                #[orm(masked)]
                recovery_hint: String,
                #[orm(encrypted)]
                note: String,
                cpf: SecretString,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let builder = quote::format_ident!("AccountQueryBuilder");
        let generated = generate_search_method(&parsed, &builder).to_string();

        assert!(generated.contains("& [\"id\" , \"name\"]"));
        for column in ["reset_token", "recovery_hint", "note", "cpf"] {
            assert!(
                !generated.contains(&format!("\"{column}\"")),
                "{column} searched"
            );
        }
        assert!(generated.contains("LIKE ? ESCAPE '!'"));
    }

    #[test]
    fn query_binds_the_tenant_context_as_the_tenant_field_type() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "orders", tenant_column = "org")]
            struct Order { id: i32, org: String }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let builder = quote::format_ident!("OrderQueryBuilder");
        let generated = generate_query_methods(&parsed, &builder).to_string();
        assert!(generated.contains("let typed : Result < String , _ > = tenant . try_into ()"));
        assert!(generated.contains("tenant context type does not match"));
        assert!(!generated.contains("where_eq (\"org\" , tenant) ;"));
    }

    #[test]
    fn scoped_models_answer_a_capped_engine_result_from_sql() {
        for input in [
            parse_quote! {
                #[orm(table = "invoices", searchable, tenant_column = "org")]
                struct Invoice { id: i32, org: String, title: String }
            },
            parse_quote! {
                #[orm(table = "invoices", searchable)]
                struct Invoice { id: i32, title: String, deleted_at: Option<String> }
            },
        ] {
            let input: DeriveInput = input;
            let parsed = crate::parser::parse(&input).expect("test model should parse");
            let builder = quote::format_ident!("InvoiceQueryBuilder");
            let generated = generate_search_method(&parsed, &builder).to_string();
            assert!(generated.contains("ids . len () < rullst_orm :: scout :: MAX_SEARCH_HITS"));
        }
    }

    #[test]
    fn unscoped_models_keep_the_capped_engine_answer() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "articles", searchable)]
            struct Article { id: i32, title: String }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let builder = quote::format_ident!("ArticleQueryBuilder");
        let generated = generate_search_method(&parsed, &builder).to_string();
        assert!(!generated.contains("MAX_SEARCH_HITS"));
        assert!(generated.contains(
            "return base_builder . __rullst_order_by_relevance (& ids) . where_in (\"id\" , ids)"
        ));
    }
}
