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
    let engine_result = if scoped {
        quote! {
            if ids.len() < rullst_orm::scout::MAX_SEARCH_HITS {
                return base_builder.where_in("id", ids);
            }
        }
    } else {
        quote! { return base_builder.where_in("id", ids); }
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

    let tenant_scope_logic = if !parsed.tenant_column.is_empty() {
        let col = &parsed.tenant_column;
        quote! {
            if let Some(tenant) = rullst_orm::tenant::get_tenant_id() {
                builder = builder.where_eq(#col, tenant);
            } else {
                builder.errors.push(rullst_orm::Error::Validation(format!(
                    "tenant context is required to query `{}`; use with_tenant(...) or the explicit unscoped() escape hatch",
                    #table_name
                )));
            }
        }
    } else {
        quote! {}
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
        assert!(generated.contains("return base_builder . where_in (\"id\" , ids)"));
    }
}
