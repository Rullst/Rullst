//! Safe scalar projections share the model query's transaction and validation.

use crate::parser::{EncryptedFieldKind, ParsedModel};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_pluck_methods(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let nullable_encrypted = parsed
        .encrypted_fields
        .iter()
        .filter(|field| field.kind == EncryptedFieldKind::OptionalString)
        .map(|field| field.name.to_string());
    let fetch_strings = generate_fetch("pluck_strings_with_executor", quote!(String));
    let fetch_integers = generate_fetch("pluck_integers_with_executor", quote!(i32));

    quote! {
        fn validate_pluck_column<'a>(&self, column: &'a str) -> Result<&'a str, rullst_orm::Error> {
            if let Some(error) = self.errors.first() {
                return Err(error.clone());
            }
            rullst_orm::schema::validate_identifier(column).map_err(|error| {
                rullst_orm::Error::Validation(format!("pluck() — invalid column identifier: {}", error))
            })?;
            if Self::is_skipped_column(column) {
                return Err(rullst_orm::Error::Validation(format!(
                    "pluck() cannot read skipped column `{}`", column
                )));
            }
            Ok(column.rsplit('.').next().unwrap_or(column))
        }

        pub async fn pluck_string(&self, column: &str) -> Result<Vec<String>, rullst_orm::Error> {
            let field = self.validate_pluck_column(column)?;
            const ENCRYPTED_OPTIONAL_COLUMNS: &[&str] = &[#(#nullable_encrypted),*];
            if ENCRYPTED_OPTIONAL_COLUMNS.contains(&field) {
                return Err(rullst_orm::Error::Validation(format!(
                    "pluck_string() cannot decode nullable encrypted column `{}`; load the model instead", column
                )));
            }
            let rows = rullst_orm::dispatch_executor!(read_pool, |executor| {
                self.pluck_strings_with_executor(column, executor).await
            })?;
            if Self::ENCRYPTED_COLUMNS.contains(&field) {
                rows.into_iter().map(|value| {
                    rullst_orm::privacy::decrypt_model_field(&value, #table_name, field).map_err(Into::into)
                }).collect()
            } else {
                Ok(rows)
            }
        }

        pub async fn pluck_i32(&self, column: &str) -> Result<Vec<i32>, rullst_orm::Error> {
            let field = self.validate_pluck_column(column)?;
            if Self::ENCRYPTED_COLUMNS.contains(&field) {
                return Err(rullst_orm::Error::Validation(format!(
                    "pluck_i32() cannot decode encrypted column `{}`; load the model instead", column
                )));
            }
            rullst_orm::dispatch_executor!(read_pool, |executor| {
                self.pluck_integers_with_executor(column, executor).await
            })
        }

        #fetch_strings
        #fetch_integers
    }
}

fn generate_fetch(method: &str, value_type: TokenStream) -> TokenStream {
    let method = format_ident!("{method}");
    quote! {
        async fn #method<'e, E>(&self, column: &str, executor: E) -> Result<Vec<#value_type>, rullst_orm::Error>
        where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
        {
            let query_str = self.to_pluck_sql(column);
            let query_bindings = self.select_bindings();
            let mut query = rullst_orm::_sqlx::query_as::<_, (#value_type,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
            for binding in &query_bindings {
                match binding {
                    rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                    rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                    rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                    rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                }
            }
            let rows = if let Some(timeout) = rullst_orm::schema::get_query_timeout() {
                tokio::time::timeout(timeout, query.fetch_all(executor))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                query.fetch_all(executor).await?
            };
            Ok(rows.into_iter().map(|(value,)| value).collect())
        }
    }
}
