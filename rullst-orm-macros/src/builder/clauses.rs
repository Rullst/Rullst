// src/builder/clauses.rs — Query builder struct definition, chained JOIN, ORDER BY, and CTE clauses.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[allow(clippy::too_many_arguments)]
pub fn generate_builder_struct(
    parsed: &ParsedModel,
    builder_name: &syn::Ident,
    relation_flags: &[TokenStream],
    relation_inits: &[TokenStream],
    relation_methods: &[TokenStream],
    where_clause_methods: &TokenStream,
    sql_assembly_methods: &TokenStream,
    execution_methods: &[TokenStream],
    magic_methods: &[TokenStream],
) -> TokenStream {
    let skipped_columns: Vec<String> = parsed
        .skipped_fields
        .iter()
        .map(|ident| ident.to_string())
        .collect();
    let skipped_columns_lit = skipped_columns.clone();

    quote! {
        #[derive(Clone)]
        pub struct #builder_name {
            pub selects: Option<String>,
            pub is_distinct: bool,
            pub limit: Option<usize>,
            pub offset: Option<usize>,
            pub order_by: Option<String>,
            pub group_by: Option<String>,
            pub joins: Vec<String>,
            pub wheres: Vec<(String, String)>,
            pub havings: Vec<(String, String)>,
            pub bindings: Vec<rullst_orm::RullstValue>,
            pub errors: Vec<rullst_orm::Error>,
            pub ctes: Vec<String>,
            pub has_recursive_cte: bool,
            pub with_trashed: bool,
            pub only_trashed: bool,
            #[cfg(feature = "redis")]
            pub remember_ttl: Option<usize>,
            #(#relation_flags)*
        }

        impl rullst_orm::schema::SubqueryBuilder for #builder_name {
            fn to_sql(&self) -> String {
                self.to_sql()
            }
            fn bindings(&self) -> &Vec<rullst_orm::RullstValue> {
                &self.bindings
            }
        }

        impl #builder_name {
            const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns_lit),*];

            fn is_skipped_column(column: &str) -> bool {
                Self::SKIPPED_COLUMNS.iter().any(|c| *c == column)
            }

            fn reject_skipped_column(&mut self, column: &str) -> bool {
                if Self::is_skipped_column(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "column `{}` is declared with `#[orm(skip)]` / `#[sqlx(skip)]` and does not exist in the table; it must not be used in WHERE / ORDER BY / GROUP BY / SELECT",
                        column
                    )));
                    true
                } else {
                    false
                }
            }

            pub fn new() -> Self {
                Self {
                    selects: None,
                    is_distinct: false,
                    limit: rullst_orm::schema::get_max_query_limit(),
                    offset: None,
                    order_by: None,
                    group_by: None,
                    joins: vec![],
                    wheres: vec![],
                    havings: vec![],
                    bindings: vec![],
                    errors: vec![],
                    ctes: vec![],
                    has_recursive_cte: false,
                    with_trashed: false,
                    only_trashed: false,
                    #[cfg(feature = "redis")]
                    remember_ttl: None,
                    #(#relation_inits)*
                }
            }

            #(#relation_methods)*

            #[cfg(feature = "redis")]
            pub fn remember(mut self, seconds: usize) -> Self {
                self.remember_ttl = Some(seconds);
                self
            }

            /// Executes a raw WHERE clause with parameterized bindings.
            pub fn where_raw<V: Into<rullst_orm::RullstValue>>(mut self, query: &str, bindings: Vec<V>) -> Self {
                self.wheres.push(("AND".to_string(), query.to_string()));
                for b in bindings {
                    self.bindings.push(b.into());
                }
                self
            }

            pub fn bind<T: Into<rullst_orm::RullstValue>>(mut self, value: T) -> Self {
                self.bindings.push(value.into());
                self
            }

            /// Executes a raw OR WHERE clause with parameterized bindings.
            pub fn or_where_raw<V: Into<rullst_orm::RullstValue>>(mut self, query: &str, bindings: Vec<V>) -> Self {
                self.wheres.push(("OR".to_string(), query.to_string()));
                for b in bindings {
                    self.bindings.push(b.into());
                }
                self
            }

            pub fn where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let sql = subquery.to_sql();
                self.wheres.push(("AND".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            pub fn or_where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let sql = subquery.to_sql();
                self.wheres.push(("OR".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            pub fn with_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_raw() — invalid CTE identifier: {}", e)));
                }
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self
            }

            pub fn with_recursive_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive_raw() — invalid CTE identifier: {}", e)));
                }
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self.has_recursive_cte = true;
                self
            }

            pub fn with_cte<B: rullst_orm::schema::SubqueryBuilder>(mut self, cte_name: &str, subquery: B) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_cte() — invalid CTE identifier: {}", e)));
                }
                let sql = subquery.to_sql();
                self.ctes.push(format!("{} AS ({})", cte_name, sql));
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            pub fn with_recursive<B: rullst_orm::schema::SubqueryBuilder>(mut self, cte_name: &str, subquery: B) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive() — invalid CTE identifier: {}", e)));
                }
                let sql = subquery.to_sql();
                self.ctes.push(format!("{} AS ({})", cte_name, sql));
                self.has_recursive_cte = true;
                for binding in subquery.bindings() {
                    self.bindings.push(binding.clone());
                }
                self
            }

            pub fn select_raw(mut self, query: &str) -> Self {
                self.selects = Some(query.to_string());
                self
            }

            pub fn distinct(mut self) -> Self {
                self.is_distinct = true;
                self
            }

            pub fn with_trashed(mut self) -> Self {
                self.with_trashed = true;
                self
            }

            pub fn only_trashed(mut self) -> Self {
                self.only_trashed = true;
                self
            }

            pub fn join_constrained<F>(mut self, table: &str, modifier: F) -> Self
            where F: FnOnce(&mut rullst_orm::JoinClause) -> &mut rullst_orm::JoinClause
            {
                let mut clause = rullst_orm::JoinClause::new("INNER");
                modifier(&mut clause);
                self.joins.push(format!("INNER JOIN {} ON {}", table, clause.to_sql()));
                for binding in clause.bindings {
                    self.bindings.push(binding);
                }
                self
            }

            pub fn join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(table) {
                    self.errors.push(rullst_orm::Error::Validation(format!("join() — invalid table identifier: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(first) {
                    self.errors.push(rullst_orm::Error::Validation(format!("join() — invalid column identifier for `first`: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(second) {
                    self.errors.push(rullst_orm::Error::Validation(format!("join() — invalid column identifier for `second`: {}", e)));
                }
                self.joins.push(format!("INNER JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn left_join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(table) {
                    self.errors.push(rullst_orm::Error::Validation(format!("left_join() — invalid table identifier: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(first) {
                    self.errors.push(rullst_orm::Error::Validation(format!("left_join() — invalid column identifier for `first`: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(second) {
                    self.errors.push(rullst_orm::Error::Validation(format!("left_join() — invalid column identifier for `second`: {}", e)));
                }
                self.joins.push(format!("LEFT JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn right_join(mut self, table: &str, first: &str, operator: &str, second: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(table) {
                    self.errors.push(rullst_orm::Error::Validation(format!("right_join() — invalid table identifier: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(first) {
                    self.errors.push(rullst_orm::Error::Validation(format!("right_join() — invalid column identifier for `first`: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(second) {
                    self.errors.push(rullst_orm::Error::Validation(format!("right_join() — invalid column identifier for `second`: {}", e)));
                }
                self.joins.push(format!("RIGHT JOIN {} ON {} {} {}", table, first, operator, second));
                self
            }

            pub fn group_by(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("group_by() — invalid column identifier: {}", e)));
                }
                self.group_by = Some(column.to_string());
                self
            }

            pub fn order_by(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by() — invalid column identifier: {}", e)));
                }
                self.order_by = Some(format!("{} ASC", column));
                self
            }

            pub fn order_by_desc(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_desc() — invalid column identifier: {}", e)));
                }
                self.order_by = Some(format!("{} DESC", column));
                self
            }

            pub fn order_by_similarity(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.order_by_l2_distance(column, vector)
            }

            pub fn order_by_l2_distance(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_l2_distance() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <-> '{}'", column, vec_str));
                self
            }

            pub fn order_by_cosine_distance(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_cosine_distance() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <=> '{}'", column, vec_str));
                self
            }

            pub fn order_by_inner_product(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_inner_product() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <#> '{}'", column, vec_str));
                self
            }

            pub fn where_similar(mut self, column: &str, vector: Vec<f64>, distance: f64) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_similar() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.wheres.push(("AND".to_string(), format!("{} <-> '{}' < {}", column, vec_str, distance)));
                self
            }

            pub fn limit(mut self, value: usize) -> Self {
                if let Some(max_limit) = rullst_orm::schema::get_max_query_limit() {
                    self.limit = Some(value.min(max_limit));
                } else {
                    self.limit = Some(value);
                }
                self
            }

            pub fn unsafe_unlimited(mut self) -> Self {
                self.limit = None;
                self
            }

            pub fn offset(mut self, value: usize) -> Self {
                self.offset = Some(value);
                self
            }

            #where_clause_methods
            #sql_assembly_methods
            #(#execution_methods)*
            #(#magic_methods)*
        }
    }
}
