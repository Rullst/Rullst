//! The read of the persisted row behind a model handle inside a mutation's
//! savepoint: the pre-image of an auditable `save()`, the row a revision
//! restore patches and the row `restore()` re-reads.

use super::mutation_parts::tenant_predicate;
use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Binds `row: Option<Self>` to the decrypted row with `self.id` (and the
/// handle's tenant), read through the savepoint `tx` within the configured
/// query timeout. PostgreSQL and MySQL/MariaDB lock it `FOR UPDATE`, so the
/// write that follows replaces exactly the row that was read.
pub(super) fn locked_row(parsed: &ParsedModel, row: &syn::Ident) -> TokenStream {
    let tenant = tenant_predicate(parsed);
    let tenant_binding = &tenant.binding;
    let sql = format!(
        "SELECT * FROM {} WHERE id = ?{}",
        parsed.table_name, tenant.clause
    );
    let locked_sql = format!("{sql} FOR UPDATE");
    quote! {
        let #row: Option<Self> = {
            let lookup = match rullst_orm::Orm::driver()? {
                "postgres" => rullst_orm::replace_placeholders(#locked_sql),
                "mysql" => #locked_sql.to_string(),
                _ => #sql.to_string(),
            };
            let query = rullst_orm::_sqlx::query_as::<_, Self>(
                rullst_orm::_sqlx::AssertSqlSafe(lookup.as_str())
            ).bind(self.id) #tenant_binding;
            let fetched = if let Some(timeout) = rullst_orm::schema::get_query_timeout() {
                tokio::time::timeout(timeout, query.fetch_optional(&mut **tx))
                    .await
                    .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
            } else {
                query.fetch_optional(&mut **tx).await?
            };
            match fetched {
                Some(mut fetched) => {
                    fetched.__rullst_decrypt_encrypted_fields()?;
                    Some(fetched)
                }
                None => None,
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn locks_the_row_with_its_tenant_under_the_query_timeout() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "lessons", tenant_column = "org", auditable)]
            struct Lesson { id: i32, org: String, title: String }
        };
        let parsed = crate::parser::parse(&input).expect("parse model");
        let row = quote::format_ident!("current");
        let lookup = super::locked_row(&parsed, &row).to_string();
        assert!(lookup.contains("\"SELECT * FROM lessons WHERE id = ? AND org = ? FOR UPDATE\""));
        assert!(lookup.contains("\"mysql\" =>"));
        assert!(lookup.contains("_ => \"SELECT * FROM lessons WHERE id = ? AND org = ?\""));
        assert!(lookup.contains(". bind (self . id) . bind (self . org . clone ())"));
        assert!(lookup.contains("get_query_timeout"));
        assert!(lookup.contains("__rullst_decrypt_encrypted_fields"));
    }
}
