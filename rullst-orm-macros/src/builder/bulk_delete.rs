//! Generated `delete_all()`: one bulk `DELETE` (or soft-delete `UPDATE`).
//!
//! The statement renders only the WHERE and soft-delete predicates, so builder
//! clauses that would bound or reshape a SELECT (an explicit `limit()`,
//! `offset()`, `order_by()`, joins, grouping or CTEs) are rejected instead of
//! being silently dropped into an unbounded delete.

use crate::parser::{ParsedModel, SoftDeleteConfig};
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
            let delval = "CURRENT_TIMESTAMP";
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
pub fn generate_delete_all_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let delete_all_logic = generate_delete_all_logic(parsed);
    let has_policy = !parsed.policy.is_empty();
    quote! {
    #[rullst_orm::_tracing::instrument(
        name = "rullst.orm.query",
        target = "rullst_orm",
        skip(self),
        fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "delete_all")
    )]
    pub async fn delete_all(&self) -> Result<u64, rullst_orm::Error> {
        rullst_orm::dispatch_executor!(pool, |pool| self.delete_all_with_tx_internal(pool).await)
    }

    #[rullst_orm::_tracing::instrument(
        name = "rullst.orm.query",
        target = "rullst_orm",
        skip(self, tx),
        fields(orm.model = stringify!(#name), orm.table = #table_name, orm.operation = "delete_all_with_tx")
    )]
    pub async fn delete_all_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'_>) -> Result<u64, rullst_orm::Error> {
        self.delete_all_with_tx_internal(&mut **tx).await
    }

    async fn delete_all_with_tx_internal<'e, E>(&self, executor: E) -> Result<u64, rullst_orm::Error>
    where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
    {
        if !self.errors.is_empty() {
            return Err(self.errors[0].clone());
        }
        if let Some(clause) = self.__rullst_unsupported_delete_clause() {
            return Err(rullst_orm::Error::Validation(format!(
                "delete_all() does not support {}; it would delete every matching row. Select the IDs with get() or pluck_i32() and delete them with where_in(\"id\", ...)",
                clause
            )));
        }
        if #has_policy {
            return Err(rullst_orm::Error::Validation(
                "delete_all() cannot authorize a policy-protected model; load the records and call each model's delete() inside Orm::transaction(...)".to_string()
            ));
        }
        let query_str = self.__rullst_delete_all_sql(rullst_orm::Orm::driver().unwrap_or_default());
        let query_bindings = self.scope_bindings.iter().chain(self.bindings.iter());
        if rullst_orm::schema::is_query_log_enabled() {
            println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.scope_bindings.len() + self.bindings.len());
        }
        let result = {
            let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
            for binding in query_bindings {
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

    /// Renders `delete_all` (a soft-delete UPDATE on soft-delete models) for `driver`;
    /// only PostgreSQL numbers the portable `?` markers, once, in textual order.
    fn __rullst_delete_all_sql(&self, driver: &str) -> String {
        #delete_all_logic
        let first_where = self.push_wheres(&mut query_str);
        self.push_soft_deletes(&mut query_str, first_where);
        if driver == "postgres" {
            rullst_orm::replace_placeholders(&query_str)
        } else {
            query_str
        }
    }

        /// The first builder clause `delete_all()` cannot honour, if any. The
        /// global row cap is implicit; only an explicit `limit()` (or a
        /// directly assigned different limit) counts.
        fn __rullst_unsupported_delete_clause(&self) -> Option<&'static str> {
            let explicit_limit = self.limit.is_some()
                && (self.limit_explicit || self.limit != rullst_orm::schema::get_max_query_limit());
            if explicit_limit {
                Some("limit()")
            } else if self.offset.is_some() {
                Some("offset()")
            } else if self.order_by.is_some() {
                Some("order_by()")
            } else if !self.joins.is_empty() || !self.join_bindings.is_empty() {
                Some("joins")
            } else if self.group_by.is_some() || !self.havings.is_empty() {
                Some("group_by()/having")
            } else if !self.ctes.is_empty() {
                Some("CTEs")
            } else {
                None
            }
        }
    }
}
