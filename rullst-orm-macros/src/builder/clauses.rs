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
    let column_guards = super::column_guards::generate_column_guards(parsed);
    let subquery_methods = super::subqueries::generate_subquery_methods();
    let redis_cfg = crate::feature_gates::redis();

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
            select_raw_bound: Option<(String, Vec<rullst_orm::RullstValue>)>,
            limit_explicit: bool,
            #redis_cfg
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
            #column_guards

            /// Bindings of a `select_raw_bindings` fragment while it is still the
            /// rendered select list; replacing the select list drops them.
            fn __rullst_select_raw_bindings(&self) -> &[rullst_orm::RullstValue] {
                match &self.select_raw_bound {
                    Some((sql, bindings)) if self.selects.as_deref() == Some(sql.as_str()) => bindings,
                    _ => &[],
                }
            }

            fn select_bindings(&self) -> Vec<rullst_orm::RullstValue> {
                self.cte_bindings
                    .iter()
                    .chain(self.__rullst_select_raw_bindings().iter())
                    .chain(self.join_bindings.iter())
                    .chain(self.scope_bindings.iter())
                    .chain(self.bindings.iter())
                    .chain(self.order_bindings.iter())
                    .cloned()
                    .collect()
            }

            /// `to_pluck_sql` replaces the select list, so its fragment's bindings go too.
            fn __rullst_pluck_bindings(&self) -> Vec<rullst_orm::RullstValue> {
                self.count_bindings()
                    .into_iter()
                    .chain(self.order_bindings.iter().cloned())
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
                    select_raw_bound: None,
                    limit_explicit: false,
                    #redis_cfg
                    remember_ttl: None,
                    #(#relation_inits)*
                }
            }

            #(#relation_methods)*

            #redis_cfg
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

            /// Appends a value for the next unbound `?` of a WHERE fragment
            /// such as `where_raw`. Raw CTE/select fragments take their values
            /// through their `_bindings` variants instead.
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

            /// Sets a caller-owned raw select list without bind markers; use
            /// [`Self::select_raw_bindings`] for a parameterized one.
            pub fn select_raw(mut self, query: &str) -> Self {
                self.__rullst_reject_raw_markers("select_raw", query);
                self.selects = Some(query.to_string());
                self
            }

            /// Sets a caller-owned raw select list whose `?` markers take
            /// `bindings` in order; they are bound after the CTEs and before
            /// JOIN, scope and WHERE values. A marker/binding mismatch fails closed.
            pub fn select_raw_bindings<V: Into<rullst_orm::RullstValue>>(mut self, query: &str, bindings: Vec<V>) -> Self {
                match rullst_orm::raw_fragment(query, bindings.into_iter().map(Into::into).collect()) {
                    Ok((sql, ordered)) => {
                        self.selects = Some(sql.clone());
                        self.select_raw_bound = Some((sql, ordered));
                    }
                    Err(error) => self.errors.push(error),
                }
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
                self.limit_explicit = true;
                if let Some(max_limit) = rullst_orm::schema::get_max_query_limit() {
                    self.limit = Some(value.min(max_limit));
                } else {
                    self.limit = Some(value);
                }
                self
            }

            pub fn unsafe_unlimited(mut self) -> Self {
                self.limit = None;
                self.limit_explicit = false;
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
