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
    let encrypted_columns: Vec<String> = parsed
        .encrypted_fields
        .iter()
        .map(|field| field.name.to_string())
        .collect();
    let encrypted_columns_lit = encrypted_columns.clone();
    let subquery_methods = super::subqueries::generate_subquery_methods();

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
            scope_wheres: Vec<String>,
            scope_bindings: Vec<rullst_orm::RullstValue>,
            pub havings: Vec<(String, String)>,
            pub cte_bindings: Vec<rullst_orm::RullstValue>,
            pub join_bindings: Vec<rullst_orm::RullstValue>,
            pub bindings: Vec<rullst_orm::RullstValue>,
            pub order_bindings: Vec<rullst_orm::RullstValue>,
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
            fn ordered_bindings(&self) -> Vec<rullst_orm::RullstValue> {
                self.select_bindings()
            }
            fn validation_error(&self) -> Option<rullst_orm::Error> {
                self.errors.first().cloned()
            }
        }

        impl #builder_name {
            const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns_lit),*];
            const ENCRYPTED_COLUMNS: &'static [&'static str] = &[#(#encrypted_columns_lit),*];

            fn is_skipped_column(column: &str) -> bool {
                let column = column.rsplit('.').next().unwrap_or(column);
                Self::SKIPPED_COLUMNS.iter().any(|c| *c == column)
            }

            fn reject_skipped_column(&mut self, column: &str) -> bool {
                let column = column.rsplit('.').next().unwrap_or(column);
                if Self::is_skipped_column(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "column `{}` is declared with `#[orm(skip)]` / `#[sqlx(skip)]` and does not exist in the table; it must not be used in WHERE / ORDER BY / GROUP BY / SELECT",
                        column
                    )));
                    true
                } else if Self::ENCRYPTED_COLUMNS.iter().any(|candidate| *candidate == column) {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "column `{}` uses randomized `#[orm(encrypted)]` storage and cannot be used in WHERE / ORDER BY / GROUP BY / SELECT; query a separate blind-index column instead",
                        column
                    )));
                    true
                } else {
                    false
                }
            }

            fn select_bindings(&self) -> Vec<rullst_orm::RullstValue> {
                self.cte_bindings
                    .iter()
                    .chain(self.join_bindings.iter())
                    .chain(self.scope_bindings.iter())
                    .chain(self.bindings.iter())
                    .chain(self.order_bindings.iter())
                    .cloned()
                    .collect()
            }

            fn count_bindings(&self) -> Vec<rullst_orm::RullstValue> {
                self.cte_bindings
                    .iter()
                    .chain(self.join_bindings.iter())
                    .chain(self.scope_bindings.iter())
                    .chain(self.bindings.iter())
                    .cloned()
                    .collect()
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
                    scope_wheres: vec![],
                    scope_bindings: vec![],
                    havings: vec![],
                    cte_bindings: vec![],
                    join_bindings: vec![],
                    bindings: vec![],
                    order_bindings: vec![],
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
                if seconds == 0 {
                    self.errors.push(rullst_orm::Error::Validation(
                        "remember() requires a TTL greater than zero".to_string()
                    ));
                } else {
                    self.remember_ttl = Some(seconds);
                }
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

            #subquery_methods

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
                if let Err(error) = rullst_orm::schema::validate_table_name(table) {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "join_constrained() — invalid table identifier: {}",
                        error
                    )));
                }
                let mut clause = rullst_orm::JoinClause::new(table);
                modifier(&mut clause);
                self.errors.extend(clause.errors.iter().cloned());
                self.joins.push(format!("INNER JOIN {} ON {}", table, clause.to_sql()));
                for binding in clause.bindings {
                    self.join_bindings.push(binding);
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
                if !rullst_orm::schema::ALLOWED_OPERATORS.contains(&operator) {
                    self.errors.push(rullst_orm::Error::Validation(format!("join() — invalid operator `{}`", operator)));
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
                if !rullst_orm::schema::ALLOWED_OPERATORS.contains(&operator) {
                    self.errors.push(rullst_orm::Error::Validation(format!("left_join() — invalid operator `{}`", operator)));
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
                if !rullst_orm::schema::ALLOWED_OPERATORS.contains(&operator) {
                    self.errors.push(rullst_orm::Error::Validation(format!("right_join() — invalid operator `{}`", operator)));
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
                self.order_bindings.clear();
                self.order_by = Some(format!("{} ASC", column));
                self
            }

            pub fn order_by_desc(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_desc() — invalid column identifier: {}", e)));
                }
                self.order_bindings.clear();
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
                self.order_bindings.clear();
                if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
                    self.errors.push(rullst_orm::Error::Validation(
                        "order_by_l2_distance() requires a non-empty finite vector".to_string()
                    ));
                    return self;
                }
                let vec_str = match rullst_orm::_serde_json::to_string(&vector) {
                    Ok(value) => value,
                    Err(error) => {
                        self.errors.push(error.into());
                        return self;
                    }
                };
                self.order_by = Some(format!("{} <-> CAST(? AS vector)", column));
                self.order_bindings.push(vec_str.into());
                self
            }

            pub fn order_by_cosine_distance(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_cosine_distance() — invalid column identifier: {}", e)));
                }
                self.order_bindings.clear();
                if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
                    self.errors.push(rullst_orm::Error::Validation(
                        "order_by_cosine_distance() requires a non-empty finite vector".to_string()
                    ));
                    return self;
                }
                let vec_str = match rullst_orm::_serde_json::to_string(&vector) {
                    Ok(value) => value,
                    Err(error) => {
                        self.errors.push(error.into());
                        return self;
                    }
                };
                self.order_by = Some(format!("{} <=> CAST(? AS vector)", column));
                self.order_bindings.push(vec_str.into());
                self
            }

            pub fn order_by_inner_product(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_inner_product() — invalid column identifier: {}", e)));
                }
                self.order_bindings.clear();
                if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
                    self.errors.push(rullst_orm::Error::Validation(
                        "order_by_inner_product() requires a non-empty finite vector".to_string()
                    ));
                    return self;
                }
                let vec_str = match rullst_orm::_serde_json::to_string(&vector) {
                    Ok(value) => value,
                    Err(error) => {
                        self.errors.push(error.into());
                        return self;
                    }
                };
                self.order_by = Some(format!("{} <#> CAST(? AS vector)", column));
                self.order_bindings.push(vec_str.into());
                self
            }

            pub fn where_similar(mut self, column: &str, vector: Vec<f64>, distance: f64) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_similar() — invalid column identifier: {}", e)));
                }
                if vector.is_empty() || vector.iter().any(|value| !value.is_finite()) {
                    self.errors.push(rullst_orm::Error::Validation(
                        "where_similar() requires a non-empty finite vector".to_string()
                    ));
                }
                if !distance.is_finite() || distance < 0.0 {
                    self.errors.push(rullst_orm::Error::Validation(
                        "where_similar() requires a finite non-negative distance".to_string()
                    ));
                }
                if !self.errors.is_empty() {
                    return self;
                }
                let vec_str = match rullst_orm::_serde_json::to_string(&vector) {
                    Ok(value) => value,
                    Err(error) => {
                        self.errors.push(error.into());
                        return self;
                    }
                };
                self.wheres.push(("AND".to_string(), format!("{} <-> CAST(? AS vector) < ?", column)));
                self.bindings.push(vec_str.into());
                self.bindings.push(distance.into());
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
