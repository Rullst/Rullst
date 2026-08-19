// src/builder/where_clauses.rs — Generates WHERE, OR WHERE, BETWEEN, and IN filter clauses.

use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_where_clause_methods(column_enum_name: &syn::Ident) -> TokenStream {
    quote! {
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
    }
}
