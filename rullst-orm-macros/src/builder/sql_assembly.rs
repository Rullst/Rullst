// src/builder/sql_assembly.rs — SQL string building, formatting, and dialect serialization.

use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_sql_assembly_methods(
    table_name: &str,
    has_soft_deletes: bool,
    soft_delete_filter_unset: &str,
    soft_delete_filter_set: &str,
) -> TokenStream {
    let table_lit = table_name;
    let unset_lit = soft_delete_filter_unset;
    let set_lit = soft_delete_filter_set;

    quote! {
        fn push_ctes(&self, sql: &mut String) {
            if !self.ctes.is_empty() {
                if self.has_recursive_cte {
                    sql.push_str("WITH RECURSIVE ");
                } else {
                    sql.push_str("WITH ");
                }
                sql.push_str(&self.ctes.join(", "));
                sql.push(' ');
            }
        }

        fn push_select(&self, sql: &mut String) {
            let select_clause = match &self.selects {
                Some(s) => s.as_str(),
                None => "*",
            };
            sql.push_str("SELECT ");
            if self.is_distinct {
                sql.push_str("DISTINCT ");
            }
            sql.push_str(select_clause);
        }

        fn push_from(&self, sql: &mut String) {
            sql.push_str(" FROM ");
            sql.push_str(#table_lit);
        }

        fn push_joins(&self, sql: &mut String) {
            for join in &self.joins {
                sql.push(' ');
                sql.push_str(join);
            }
        }

        fn push_condition_group(sql: &mut String, conditions: &[(String, String)]) {
            if conditions.len() > 1 { sql.push('('); }
            for (index, (operator, condition)) in conditions.iter().enumerate() {
                if index > 0 {
                    sql.push(' ');
                    sql.push_str(operator);
                    sql.push(' ');
                }
                sql.push('(');
                sql.push_str(condition);
                sql.push(')');
            }
            if conditions.len() > 1 { sql.push(')'); }
        }

        // Model-wide and tenant filters must remain independent AND groups;
        // a later user OR filter must never broaden either mandatory scope.
        fn freeze_scope(&mut self) {
            if !self.wheres.is_empty() {
                let mut scope = String::new();
                Self::push_condition_group(&mut scope, &self.wheres);
                self.scope_wheres.push(scope);
                self.wheres.clear();
                self.scope_bindings.append(&mut self.bindings);
            }
        }

        fn push_wheres(&self, sql: &mut String) -> bool {
            if self.scope_wheres.is_empty() && self.wheres.is_empty() {
                return true;
            }
            sql.push_str(" WHERE ");
            sql.push_str(&self.scope_wheres.join(" AND "));
            if !self.wheres.is_empty() {
                if !self.scope_wheres.is_empty() { sql.push_str(" AND "); }
                Self::push_condition_group(sql, &self.wheres);
            }
            false
        }

        fn push_soft_deletes(&self, sql: &mut String, first_where: bool) {
            if #has_soft_deletes && !self.with_trashed {
                if first_where {
                    sql.push_str(" WHERE ");
                } else {
                    sql.push_str(" AND ");
                }
                if self.only_trashed {
                    sql.push_str(#set_lit);
                } else {
                    sql.push_str(#unset_lit);
                }
            }
        }

        fn push_group_by(&self, sql: &mut String) {
            if let Some(group) = &self.group_by {
                sql.push_str(" GROUP BY ");
                sql.push_str(group);
            }
        }

        fn push_havings(&self, sql: &mut String) {
            let mut first_having = true;
            if !self.havings.is_empty() {
                sql.push_str(" HAVING ");
                for (op, cond) in &self.havings {
                    if first_having {
                        sql.push('(');
                        sql.push_str(cond);
                        sql.push(')');
                        first_having = false;
                    } else {
                        sql.push(' ');
                        sql.push_str(op);
                        sql.push_str(" (");
                        sql.push_str(cond);
                        sql.push(')');
                    }
                }
            }
        }

        fn push_order_by(&self, sql: &mut String) {
            if let Some(order) = &self.order_by {
                sql.push_str(" ORDER BY ");
                sql.push_str(order);
            }
        }

        fn push_limit_offset(&self, sql: &mut String) {
            if let Some(limit) = self.limit {
                sql.push_str(" LIMIT ");
                sql.push_str(&limit.to_string());
            }
            if let Some(offset) = self.offset {
                sql.push_str(" OFFSET ");
                sql.push_str(&offset.to_string());
            }
        }

        fn format_postgres(&self, sql: &str) -> String {
            match rullst_orm::Orm::driver() {
                Ok("postgres") => rullst_orm::replace_placeholders(sql),
                // SQL assembly is non-executing and historically infallible.
                // Execution still obtains a fallible pool before sending SQL.
                Ok(_) | Err(_) => sql.to_string(),
            }
        }

        pub fn to_sql(&self) -> String {
            let estimated_capacity = 50 + #table_lit.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
                + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>()
                + self.ctes.iter().map(|c| c.len() + 2).sum::<usize>();
            let mut sql = String::with_capacity(estimated_capacity);

            self.push_ctes(&mut sql);
            self.push_select(&mut sql);
            self.push_from(&mut sql);
            self.push_joins(&mut sql);
            let first_where = self.push_wheres(&mut sql);
            self.push_soft_deletes(&mut sql, first_where);
            self.push_group_by(&mut sql);
            self.push_havings(&mut sql);
            self.push_order_by(&mut sql);
            self.push_limit_offset(&mut sql);

            self.format_postgres(&sql)
        }

        pub fn to_count_sql(&self) -> String {
            let estimated_capacity = 50 + #table_lit.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
                + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>()
                + self.ctes.iter().map(|c| c.len() + 2).sum::<usize>();
            let mut sql = String::with_capacity(estimated_capacity);

            self.push_ctes(&mut sql);
            sql.push_str("SELECT COUNT(*)");
            self.push_from(&mut sql);
            self.push_joins(&mut sql);
            let first_where = self.push_wheres(&mut sql);
            self.push_soft_deletes(&mut sql, first_where);
            self.push_group_by(&mut sql);
            self.push_havings(&mut sql);

            self.format_postgres(&sql)
        }

        pub fn to_pluck_sql(&self, column: &str) -> String {
            let estimated_capacity = 50 + #table_lit.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
                + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>()
                + self.ctes.iter().map(|c| c.len() + 2).sum::<usize>();
            let mut sql = String::with_capacity(estimated_capacity);

            self.push_ctes(&mut sql);
            sql.push_str("SELECT ");
            if self.is_distinct { sql.push_str("DISTINCT "); }
            sql.push_str(column);

            self.push_from(&mut sql);
            self.push_joins(&mut sql);
            let first_where = self.push_wheres(&mut sql);
            self.push_soft_deletes(&mut sql, first_where);
            self.push_group_by(&mut sql);
            self.push_havings(&mut sql);
            self.push_order_by(&mut sql);
            self.push_limit_offset(&mut sql);

            self.format_postgres(&sql)
        }
    }
}
