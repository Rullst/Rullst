use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_search_method(parsed: &ParsedModel, builder_name: &syn::Ident) -> TokenStream {
    if !parsed.searchable {
        return quote! {};
    }
    let table_name = &parsed.table_name;
    let encrypted_fields = parsed
        .encrypted_fields
        .iter()
        .map(|field| &field.name)
        .collect::<Vec<_>>();
    let cols = parsed
        .normal_fields
        .iter()
        .filter(|field| !encrypted_fields.contains(field))
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

            let driver = match rullst_orm::Orm::driver() {
                Ok(driver) => driver,
                Err(error) => {
                    base_builder.errors.push(error);
                    return base_builder;
                }
            };
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
            #tenant_scope_logic
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
