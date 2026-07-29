use crate::parser::{ParsedModel, SoftDeleteConfig};
use proc_macro2::TokenStream;
use quote::quote;

/// Identifies how a soft-delete value should be compared inside
/// `SELECT` / `restore` queries. The "literal" mode matches against
/// `<column> = <value>` while the "null" mode matches against
/// `<column> IS NULL` / `IS NOT NULL`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SoftDeleteCmp {
    NullSentinel,
    LiteralSentinel,
}

impl SoftDeleteCmp {
    fn for_value(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("null") || value.trim().is_empty() {
            SoftDeleteCmp::NullSentinel
        } else {
            SoftDeleteCmp::LiteralSentinel
        }
    }
}

/// Renders the `<column> = <value>` fragment used in `SELECT` queries
/// to filter non-deleted rows.
#[cfg_attr(test, mutants::skip)]
fn soft_delete_where_clause(cfg: &SoftDeleteConfig, is_trashed: bool) -> String {
    let cmp = SoftDeleteCmp::for_value(&cfg.value);
    match (cmp, is_trashed) {
        (SoftDeleteCmp::NullSentinel, false) => format!("{} IS NULL", cfg.column),
        (SoftDeleteCmp::NullSentinel, true) => format!("{} IS NOT NULL", cfg.column),
        (SoftDeleteCmp::LiteralSentinel, false) => format!("{} = {}", cfg.column, cfg.value),
        (SoftDeleteCmp::LiteralSentinel, true) => format!("{} != {}", cfg.column, cfg.value),
    }
}

/// Generates the magic methods for each field (where_field, order_by_field, etc)
#[cfg_attr(test, mutants::skip)]
fn generate_magic_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let mut magic_methods = vec![];
    for field_name in &parsed.normal_fields {
        let field_name_str = field_name.to_string();

        let where_method = quote::format_ident!("where_{}", field_name);
        let or_where_method = quote::format_ident!("or_where_{}", field_name);
        let where_not_method = quote::format_ident!("where_not_{}", field_name);

        magic_methods.push(quote! {
            pub fn #where_method<T: Into<rullst_orm::RullstValue>>(self, value: T) -> Self {
                self.where_eq(#field_name_str, value)
            }
            pub fn #or_where_method<T: Into<rullst_orm::RullstValue>>(self, value: T) -> Self {
                self.or_where(#field_name_str, value)
            }
            pub fn #where_not_method<T: Into<rullst_orm::RullstValue>>(self, value: T) -> Self {
                self.where_not_eq(#field_name_str, value)
            }
        });

        let order_by_method = quote::format_ident!("order_by_{}", field_name);
        let order_by_desc_method = quote::format_ident!("order_by_{}_desc", field_name);
        magic_methods.push(quote! {
            pub fn #order_by_method(self) -> Self {
                self.order_by(#field_name_str)
            }
            pub fn #order_by_desc_method(self) -> Self {
                self.order_by_desc(#field_name_str)
            }
        });
    }
    magic_methods
}

#[cfg_attr(test, mutants::skip)]
fn generate_delete_all_logic(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    if !parsed.has_soft_deletes {
        return quote! {
            let mut estimated_capacity = 20 + #table_name.len() + self.wheres.iter().map(|(o, c)| o.len() + c.len() + 4).sum::<usize>();
            let mut query_str = String::with_capacity(estimated_capacity);
            query_str.push_str("DELETE FROM ");
            query_str.push_str(#table_name);
        };
    }
    // Build a portable UPDATE that flips the soft delete column to
    // its "deleted" value. We prefer the user-supplied `delval`
    // expression (e.g. `now()`, `UNIX_TIMESTAMP()`, `1`) so the same
    // generated SQL works on MySQL, PostgreSQL and SQLite.
    let cfg = parsed
        .soft_delete
        .as_ref()
        .expect("has_soft_deletes implies soft_delete config");
    let set_fragment = build_soft_delete_set_clause(cfg);
    let delval_token: TokenStream = if cfg.delval.trim().is_empty() {
        // No explicit delval: fall back to driver-specific defaults that
        // mimic the historical hard-coded behaviour.
        quote! {
            let delval = if rullst_orm::Orm::driver() == "postgres" {
                "CURRENT_TIMESTAMP"
            } else {
                "CURRENT_TIMESTAMP"
            };
        }
    } else {
        // The user supplied an expression like `now()` or
        // `UNIX_TIMESTAMP()`. It is interpolated as raw SQL (it must
        // not contain user input) so it travels through the `?`/SQL
        // pipeline untouched.
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

/// Build the SET clause template for soft delete updates. The string
/// `{VALUE}` is replaced at codegen with the actual `delval` SQL
/// fragment. This indirection keeps the column name configurable
/// while still letting us splice in a function call as a literal SQL
/// string.
#[cfg_attr(test, mutants::skip)]
fn build_soft_delete_set_clause(cfg: &SoftDeleteConfig) -> String {
    format!("{} = {{VALUE}}", cfg.column)
}

pub fn generate(
    parsed: &ParsedModel,
    relation_flags: &[TokenStream],
    relation_inits: &[TokenStream],
    relation_methods: &[TokenStream],
    eager_loads: &TokenStream,
) -> TokenStream {
    let name = &parsed.name;
    let column_enum_name = quote::format_ident!("{}Column", name);
    let builder_name = quote::format_ident!("{}QueryBuilder", name);
    let table_name = &parsed.table_name;
    let has_soft_deletes = parsed.has_soft_deletes;
    // Pre-render the SQL fragments that depend on the user's soft
    // delete config. We pre-compute them here (instead of inlining
    // strings) so the generated `push_soft_deletes` body stays clean
    // and so any user-provided `delval` / `value` survives verbatim
    // across every supported driver.
    let soft_delete_filter_unset = parsed
        .soft_delete
        .as_ref()
        .map(|cfg| soft_delete_where_clause(cfg, false))
        .unwrap_or_else(|| "deleted_at IS NULL".to_string());
    let soft_delete_filter_set = parsed
        .soft_delete
        .as_ref()
        .map(|cfg| soft_delete_where_clause(cfg, true))
        .unwrap_or_else(|| "deleted_at IS NOT NULL".to_string());
    let execution_methods = generate_execution_methods(parsed, &builder_name, eager_loads);
    let magic_methods = generate_magic_methods(parsed);
    // Names of fields tagged with `#[orm(skip)]` / `#[sqlx(skip)]`.
    // They are not real columns in the underlying table, so any
    // raw string-based `where_*` / `order_by` / `group_by` / `select`
    // call that references one of these names must surface a
    // `Validation` error instead of silently generating
    // `WHERE skipped = ?`. The typed `*Column` enum and the
    // `where_<field>` magic methods already exclude them at compile
    // time; this constant closes the remaining escape hatch.
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
            /// Names of struct fields that the user marked with
            /// `#[orm(skip)]` or `#[sqlx(skip)]`. Those fields are
            /// intentionally not part of the SQL column list, so any
            /// builder method that accepts a raw column name must
            /// reject references to them with a `Validation` error
            /// rather than letting them reach the database.
            const SKIPPED_COLUMNS: &'static [&'static str] = &[#(#skipped_columns_lit),*];

            /// Returns `true` when `column` is a struct field that
            /// the user opted out of generated SQL via
            /// `#[orm(skip)]` / `#[sqlx(skip)]`.
            fn is_skipped_column(column: &str) -> bool {
                Self::SKIPPED_COLUMNS.iter().any(|c| *c == column)
            }

            /// Records a `Validation` error when the caller referenced
            /// a `#[orm(skip)]` / `#[sqlx(skip)]` field. Returns `true`
            /// if the column is invalid, so the caller can `return
            /// self` early and skip pushing the SQL fragment.
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

            /// Executes a raw SELECT clause.
            /// WARNING: Make sure to avoid user input concatenation in the select string.
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

            pub fn where_eq<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_eq() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} = ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_not_eq<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_not_eq() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} != ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_gt<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_gt() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} > ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_lt<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_lt() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} < ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_like<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_like() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_not_like<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_not_like() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} NOT LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn where_null(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_null() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} IS NULL", column)));
                self
            }

            pub fn select(mut self, columns: &[&str]) -> Self {
                for col in columns {
                    self.reject_skipped_column(col);
                }
                self.selects = Some(columns.join(", "));
                self
            }

            pub fn select_cols(mut self, cols: &[#column_enum_name]) -> Self {
                let s = cols.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", ");
                self.selects = Some(s);
                self
            }

            pub fn where_col<T: Into<rullst_orm::RullstValue>>(mut self, col: #column_enum_name, value: T) -> Self {
                self.wheres.push(("AND".to_string(), format!("{} = ?", col.as_str())));
                self.bindings.push(value.into());
                self
            }

            pub fn order_by_col(mut self, col: #column_enum_name) -> Self {
                self.order_by = Some(col.as_str().to_string());
                self
            }

            pub fn order_by_desc_col(mut self, col: #column_enum_name) -> Self {
                self.order_by = Some(format!("{} DESC", col.as_str()));
                self
            }

            pub fn where_not_null(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_not_null() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} IS NOT NULL", column)));
                self
            }

            /// WARNING: Ensure `column` does not contain user input to prevent SQL Injection.
            pub fn where_in<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_in() — invalid column identifier: {}", e)));
                }
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("AND".to_string(), format!("{} IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn where_not_in<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_not_in() — invalid column identifier: {}", e)));
                }
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("AND".to_string(), format!("{} NOT IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn where_between<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_between() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
                self
            }

            pub fn where_not_between<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_not_between() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} NOT BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
                self
            }

            /// Adds a WHERE condition comparing two columns.
            ///
            /// # Panics
            /// Panics if `first` or `second` are not valid SQL identifiers.
            /// Column names must always be hardcoded — never pass user input here.
            pub fn where_column(mut self, first: &str, second: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(first) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_column() — invalid identifier for `first`: {}", e)));
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(second) {
                    self.errors.push(rullst_orm::Error::Validation(format!("where_column() — invalid identifier for `second`: {}", e)));
                }
                self.wheres.push(("AND".to_string(), format!("{} = {}", first, second)));
                self
            }

            pub fn or_where<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} = ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_not_eq<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_not_eq() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} != ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_gt<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_gt() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} > ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_lt<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_lt() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} < ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_like<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, value: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_like() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} LIKE ?", column)));
                self.bindings.push(value.into());
                self
            }

            pub fn or_where_null(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_null() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} IS NULL", column)));
                self
            }

            pub fn or_where_not_null(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_not_null() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} IS NOT NULL", column)));
                self
            }

            /// WARNING: Ensure `column` does not contain user input to prevent SQL Injection.
            pub fn or_where_in<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, values: Vec<T>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_in() — invalid column identifier: {}", e)));
                }
                if values.is_empty() { return self; }
                let placeholders = vec!["?"; values.len()].join(", ");
                self.wheres.push(("OR".to_string(), format!("{} IN ({})", column, placeholders)));
                for v in values { self.bindings.push(v.into()); }
                self
            }

            pub fn or_where_between<T: Into<rullst_orm::RullstValue>>(mut self, column: &str, min: T, max: T) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("or_where_between() — invalid column identifier: {}", e)));
                }
                self.wheres.push(("OR".to_string(), format!("{} BETWEEN ? AND ?", column)));
                self.bindings.push(min.into());
                self.bindings.push(max.into());
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

            /// Orders results by a column name ascending.
            ///
            /// # Panics
            /// Panics if `column` is not a valid SQL identifier.
            /// Column names must always be hardcoded — never pass user input here.
            pub fn order_by(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by() — invalid column identifier: {}", e)));
                }
                self.order_by = Some(format!("{} ASC", column));
                self
            }

            /// Orders results by a column name descending.
            ///
            /// # Panics
            /// Panics if `column` is not a valid SQL identifier.
            /// Column names must always be hardcoded — never pass user input here.
            pub fn order_by_desc(mut self, column: &str) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_desc() — invalid column identifier: {}", e)));
                }
                self.order_by = Some(format!("{} DESC", column));
                self
            }

            /// Orders results by L2 distance similarity using pgvector (`<->` operator).
            /// Safely formats the `vector` values directly into the query.
            pub fn order_by_similarity(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.order_by_l2_distance(column, vector)
            }

            /// Orders results by L2 distance similarity using pgvector (`<->` operator).
            pub fn order_by_l2_distance(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_l2_distance() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <-> '{}'", column, vec_str));
                self
            }

            /// Orders results by Cosine distance using pgvector (`<=>` operator).
            pub fn order_by_cosine_distance(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_cosine_distance() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <=> '{}'", column, vec_str));
                self
            }

            /// Orders results by Inner Product using pgvector (`<#>` operator).
            pub fn order_by_inner_product(mut self, column: &str, vector: Vec<f64>) -> Self {
                self.reject_skipped_column(column);
                if let Err(e) = rullst_orm::schema::validate_identifier(column) {
                    self.errors.push(rullst_orm::Error::Validation(format!("order_by_inner_product() — invalid column identifier: {}", e)));
                }
                let vec_str = rullst_orm::_serde_json::to_string(&vector).unwrap_or_else(|_| "[]".to_string());
                self.order_by = Some(format!("{} <#> '{}'", column, vec_str));
                self
            }

            /// Filters results by maximum L2 distance similarity using pgvector.
            /// Safely formats the `vector` values directly into the query.
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

            /// Disables the global max query limit for this specific query, allowing unlimited rows.
            pub fn unsafe_unlimited(mut self) -> Self {
                self.limit = None;
                self
            }

            pub fn offset(mut self, value: usize) -> Self {
                self.offset = Some(value);
                self
            }

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
                sql.push_str(#table_name);
            }

            fn push_joins(&self, sql: &mut String) {
                for join in &self.joins {
                    sql.push(' ');
                    sql.push_str(join);
                }
            }

            fn push_wheres(&self, sql: &mut String) -> bool {
                let mut first_where = true;
                if !self.wheres.is_empty() {
                    sql.push_str(" WHERE ");
                    for (op, cond) in &self.wheres {
                        if first_where {
                            sql.push('(');
                            sql.push_str(cond);
                            sql.push(')');
                            first_where = false;
                        } else {
                            sql.push(' ');
                            sql.push_str(op);
                            sql.push_str(" (");
                            sql.push_str(cond);
                            sql.push(')');
                        }
                    }
                }
                first_where
            }

            fn push_soft_deletes(&self, sql: &mut String, first_where: bool) {
                if #has_soft_deletes && !self.with_trashed {
                    if first_where {
                        sql.push_str(" WHERE ");
                    } else {
                        sql.push_str(" AND ");
                    }
                    // The column name and sentinel value come from the
                    // user's `#[orm(soft_delete(...))]` declaration (or
                    // the legacy `deleted_at` default), so this works for
                    // MySQL, PostgreSQL, and SQLite without any driver
                    // specific code.
                    if self.only_trashed {
                        sql.push_str(#soft_delete_filter_set);
                    } else {
                        sql.push_str(#soft_delete_filter_unset);
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
                if rullst_orm::Orm::driver() == "postgres" {
                    use std::fmt::Write;
                    let mut pg_sql = String::with_capacity(sql.len() + 10);
                    let mut counter = 1;
                    let mut last_idx = 0;
                    for (idx, _) in sql.match_indices('?') {
                        pg_sql.push_str(&sql[last_idx..idx]);
                        write!(pg_sql, "${}", counter).unwrap();
                        counter += 1;
                        last_idx = idx + 1;
                    }
                    pg_sql.push_str(&sql[last_idx..]);
                    pg_sql
                } else {
                    sql.to_string()
                }
            }

            /// WARNING: This generates the raw SQL query. Ensure all dynamic table names and column names are validated.
            pub fn to_sql(&self) -> String {
                let estimated_capacity = 50 + #table_name.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
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
                let estimated_capacity = 50 + #table_name.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
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
                // COUNT doesn't need ORDER BY, LIMIT, OFFSET

                self.format_postgres(&sql)
            }

            pub fn to_pluck_sql(&self, column: &str) -> String {
                let estimated_capacity = 50 + #table_name.len() + self.joins.iter().map(|j| j.len() + 1).sum::<usize>()
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

            #(#execution_methods)*
            #(#magic_methods)*
        }
    }
}

#[cfg_attr(test, mutants::skip)]
fn generate_execution_methods(
    parsed: &ParsedModel,
    _builder_name: &syn::Ident,
    eager_loads: &TokenStream,
) -> Vec<TokenStream> {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let hook_after_fetch = if !parsed.after_fetch.is_empty() {
        let method = syn::Ident::new(&parsed.after_fetch, name.span());
        quote! {
            let futures = results.iter_mut().map(|model| model.#method());
            rullst_orm::_futures::future::try_join_all(futures).await?;
        }
    } else {
        quote! {}
    };
    let hook_after_fetch_single = if !parsed.after_fetch.is_empty() {
        let method = syn::Ident::new(&parsed.after_fetch, name.span());
        quote! { row.#method().await?; }
    } else {
        quote! {}
    };
    let delete_all_logic = generate_delete_all_logic(parsed);

    vec![quote! {
    #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
    pub async fn get(&self) -> Result<Vec<#name>, rullst_orm::Error> {
        rullst_orm::dispatch_executor!(read_pool, |pool| self.get_with_tx_internal(pool).await)
    }

                pub async fn get_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Vec<#name>, rullst_orm::Error> {
                    self.get_with_tx_internal(&mut **tx).await
                }

                async fn get_with_tx_internal<'e, E>(&self, executor: E) -> Result<Vec<#name>, rullst_orm::Error>
                where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
                {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    let query_str = self.to_sql();

                    #[cfg(feature = "redis")]
                    {
                        if let Some(ttl) = self.remember_ttl {
                            use rullst_orm::_redis::AsyncCommands;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            query_str.hash(&mut hasher);
                            for binding in &self.bindings {
                                match binding {
                                    rullst_orm::RullstValue::String(s) => s.hash(&mut hasher),
                                    rullst_orm::RullstValue::Int(i) => i.hash(&mut hasher),
                                    rullst_orm::RullstValue::Float(f) => f.to_bits().hash(&mut hasher),
                                    rullst_orm::RullstValue::Bool(b) => b.hash(&mut hasher),
                                }
                            }
                            let cache_key = format!("orm:cache:{}:{:x}", #table_name, hasher.finish());
                            let mut conn = rullst_orm::Orm::redis_manager()?;
                            if let Ok(cached_data) = conn.get::<_, String>(&cache_key).await {
                                if !cached_data.is_empty() {
                                    if let Ok(mut results) = #name::from_cache_json_array(&cached_data) {
                                        #hook_after_fetch
                                        #eager_loads
                                        return Ok(results);
                                    }
                                }
                            }
                        }
                    }

                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                    }
                    let mut results: Vec<#name> = {
                        let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, query.fetch_all(executor))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            query.fetch_all(executor).await?
                        }
                    };

                    #[cfg(feature = "redis")]
                    {
                        if let Some(ttl) = self.remember_ttl {
                            use rullst_orm::_redis::AsyncCommands;
                            use std::hash::{Hash, Hasher};
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            query_str.hash(&mut hasher);
                            for binding in &self.bindings {
                                match binding {
                                    rullst_orm::RullstValue::String(s) => s.hash(&mut hasher),
                                    rullst_orm::RullstValue::Int(i) => i.hash(&mut hasher),
                                    rullst_orm::RullstValue::Float(f) => f.to_bits().hash(&mut hasher),
                                    rullst_orm::RullstValue::Bool(b) => b.hash(&mut hasher),
                                }
                            }
                            let cache_key = format!("orm:cache:{}:{:x}", #table_name, hasher.finish());
                            let serialized = #name::to_cache_json_array(&results);
                            let mut conn = rullst_orm::Orm::redis_manager()?;
                            let _: Result<(), rullst_orm::_redis::RedisError> = conn.set_ex(&cache_key, serialized, ttl as u64).await;
                        }
                    }

                    #hook_after_fetch
                    #eager_loads
                    Ok(results)
                }

                #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
                pub async fn first(&self) -> Result<Option<#name>, rullst_orm::Error> {
                    let mut builder = self.clone();
                    builder.limit = Some(1);
                    let results = builder.get().await?;
                    Ok(results.into_iter().next())
                }

                pub async fn first_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<Option<#name>, rullst_orm::Error> {
                    let mut builder = self.clone();
                    builder.limit = Some(1);
                    let results = builder.get_with_tx(tx).await?;
                    Ok(results.into_iter().next())
                }

                #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
                pub async fn paginate(&self, page: usize, per_page: usize) -> Result<rullst_orm::PaginationResult<#name>, rullst_orm::Error> {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    let mut total_builder = self.clone();
                    total_builder.selects = Some("COUNT(*)".to_string());
                    total_builder.limit = None;
                    total_builder.offset = None;
                    total_builder.order_by = None;

                    let query_str = total_builder.to_sql();
                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, total_builder.bindings.len());
                    }
                    let total_row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                        let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &total_builder.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, query.fetch_one(pool))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            query.fetch_one(pool).await?
                        }
                    });
                    let total = total_row.0;
                    let last_page = (total as f64 / per_page as f64).ceil() as usize;

                    let mut data_builder = self.clone();
                    data_builder.limit = Some(per_page);
                    if page > 1 {
                        data_builder.offset = Some((page - 1) * per_page);
                    }
                    let data = data_builder.get().await?;

                    Ok(rullst_orm::PaginationResult {
                        data,
                        total,
                        per_page,
                        current_page: if page == 0 { 1 } else { page },
                        last_page,
                    })
                }

                #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
                pub async fn count(&self) -> Result<i64, rullst_orm::Error> {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    let query_str = self.to_count_sql();
                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                    }
                    let row: (i64,) = rullst_orm::dispatch_executor!(read_pool, |pool| {
                        let mut query = rullst_orm::_sqlx::query_as::<_, (i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, query.fetch_one(pool))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            query.fetch_one(pool).await?
                        }
                    });
                    Ok(row.0)
                }

                #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self, handler))]
                pub async fn chunk<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rullst_orm::Error>
                where
                    F: FnMut(Vec<#name>) -> Fut + Send,
                    Fut: std::future::Future<Output = ()> + Send,
                {
                    let mut page = 1;
                    let mut builder = self.clone();
                    builder.limit = Some(size);
                    loop {
                        builder.offset = Some((page - 1) * size);
                        let results = builder.get().await?;
                        let count = results.len();
                        if count == 0 { break; }
                        handler(results).await;
                        if count < size { break; }
                        page += 1;
                    }
                    Ok(())
                }

                pub async fn chunk_with_tx<F, Fut>(&self, size: usize, tx: &mut rullst_orm::db::Transaction<'static>, mut handler: F) -> Result<(), rullst_orm::Error>
                where
                    F: FnMut(Vec<#name>) -> Fut + Send,
                    Fut: std::future::Future<Output = ()> + Send,
                {
                    let mut page = 1;
                    let mut builder = self.clone();
                    builder.limit = Some(size);
                    loop {
                        builder.offset = Some((page - 1) * size);
                        let results = builder.get_with_tx(tx).await?;
                        let count = results.len();
                        if count == 0 { break; }
                        handler(results).await;
                        if count < size { break; }
                        page += 1;
                    }
                    Ok(())
                }

                pub fn stream<'a>(&'a self) -> impl rullst_orm::_futures::Stream<Item = Result<#name, rullst_orm::Error>> + 'a {
                    rullst_orm::_async_stream::try_stream! {
                        if !self.errors.is_empty() {
                            Err(self.errors[0].clone())?;
                        }
                        let query_str = self.to_sql();
                        if rullst_orm::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                        }

                        let pool = rullst_orm::Orm::read_pool();
                        let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }

                        let mut db_stream = query.fetch(pool);
                        while let Some(row_res) = rullst_orm::_futures::StreamExt::next(&mut db_stream).await {
                            let mut row = row_res.map_err(|e| rullst_orm::Error::DatabaseError(e.to_string()))?;
                            #hook_after_fetch_single
                            yield row;
                        }
                    }
                }

                pub fn stream_with_tx<'a>(&'a self, tx: &'a mut rullst_orm::db::Transaction<'static>) -> impl rullst_orm::_futures::Stream<Item = Result<#name, rullst_orm::Error>> + 'a {
                    rullst_orm::_async_stream::try_stream! {
                        if !self.errors.is_empty() {
                            Err(self.errors[0].clone())?;
                        }
                        let query_str = self.to_sql();
                        if rullst_orm::schema::is_query_log_enabled() {
                            println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                        }

                        let mut query = rullst_orm::_sqlx::query_as::<_, #name>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }

                        let mut db_stream = query.fetch(&mut **tx);
                        while let Some(row_res) = rullst_orm::_futures::StreamExt::next(&mut db_stream).await {
                            let mut row = row_res.map_err(|e| rullst_orm::Error::DatabaseError(e.to_string()))?;
                            #hook_after_fetch_single
                            yield row;
                        }
                    }
                }

                #[rullst_orm::_tracing::instrument(name = "rullst_query", skip(self))]
                pub async fn delete_all(&self) -> Result<u64, rullst_orm::Error> {
                    rullst_orm::dispatch_executor!(pool, |pool| self.delete_all_with_tx_internal(pool).await)
                }

                pub async fn delete_all_with_tx(&self, tx: &mut rullst_orm::db::Transaction<'static>) -> Result<u64, rullst_orm::Error> {
                    self.delete_all_with_tx_internal(&mut **tx).await
                }

                async fn delete_all_with_tx_internal<'e, E>(&self, executor: E) -> Result<u64, rullst_orm::Error>
                where E: rullst_orm::_sqlx::Executor<'e, Database = rullst_orm::RullstDatabase>
                {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    #delete_all_logic

                    if !self.wheres.is_empty() {
                        query_str.push_str(" WHERE ");
                        let mut first = true;
                        for (operator, condition) in &self.wheres {
                            if first {
                                query_str.push('(');
                                query_str.push_str(condition);
                                query_str.push(')');
                                first = false;
                            } else {
                                query_str.push(' ');
                                query_str.push_str(operator);
                                query_str.push_str(" (");
                                query_str.push_str(condition);
                                query_str.push(')');
                            }
                        }
                    }
                    if rullst_orm::schema::is_query_log_enabled() {
                        println!("[SQL Debug] {:?} | Bindings: [{} parameter(s) redacted for security]", query_str, self.bindings.len());
                    }
                    let query_str = rullst_orm::replace_placeholders(&query_str);
                    let result = {
                        let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
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

                pub async fn pluck_string(&self, column: &str) -> Result<Vec<String>, rullst_orm::Error> {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    let pool = rullst_orm::Orm::read_pool();
                    let query_str = self.to_pluck_sql(column);
                    let rows: Vec<(String,)> = {
                        let mut query = rullst_orm::_sqlx::query_as::<_, (String,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, query.fetch_all(pool))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            query.fetch_all(pool).await?
                        }
                    };
                    Ok(rows.into_iter().map(|(s,)| s).collect())
                }

                pub async fn pluck_i32(&self, column: &str) -> Result<Vec<i32>, rullst_orm::Error> {
                    if !self.errors.is_empty() {
                        return Err(self.errors[0].clone());
                    }
                    let pool = rullst_orm::Orm::read_pool();
                    let query_str = self.to_pluck_sql(column);
                    let rows: Vec<(i32,)> = {
                        let mut query = rullst_orm::_sqlx::query_as::<_, (i32,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));
                        for binding in &self.bindings {
                            match binding {
                                rullst_orm::RullstValue::String(s) => { query = query.bind(s.clone()); }
                                rullst_orm::RullstValue::Int(i) => { query = query.bind(*i); }
                                rullst_orm::RullstValue::Float(f) => { query = query.bind(*f); }
                                rullst_orm::RullstValue::Bool(b) => { query = query.bind(*b); }
                            }
                        }
                        let timeout = rullst_orm::schema::get_query_timeout();
                        if let Some(t) = timeout {
                            tokio::time::timeout(t, query.fetch_all(pool))
                                .await
                                .map_err(|_| rullst_orm::Error::DatabaseError("Query execution timed out".to_string()))??
                        } else {
                            query.fetch_all(pool).await?
                        }
                    };
                    Ok(rows.into_iter().map(|(s,)| s).collect())
                }


        }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_delete_where_clause() {
        let cfg_null = SoftDeleteConfig {
            column: "deleted_at".into(),
            value: "null".into(),
            delval: "1".into(),
        };
        assert_eq!(
            soft_delete_where_clause(&cfg_null, false),
            "deleted_at IS NULL"
        );
        assert_eq!(
            soft_delete_where_clause(&cfg_null, true),
            "deleted_at IS NOT NULL"
        );

        let cfg_lit = SoftDeleteConfig {
            column: "is_deleted".into(),
            value: "0".into(),
            delval: "1".into(),
        };
        assert_eq!(soft_delete_where_clause(&cfg_lit, false), "is_deleted = 0");
        assert_eq!(soft_delete_where_clause(&cfg_lit, true), "is_deleted != 0");
    }
}
