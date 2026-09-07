//! Typed and raw subqueries retain their distinct validation boundaries.

use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_subquery_methods() -> TokenStream {
    quote! {
            pub fn where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                if let Some(error) = subquery.validation_error() {
                    self.errors.push(error);
                }
                let sql = subquery.to_sql();
                self.wheres.push(("AND".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.ordered_bindings() {
                    self.bindings.push(binding);
                }
                self
            }

            pub fn or_where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                if let Some(error) = subquery.validation_error() {
                    self.errors.push(error);
                }
                let sql = subquery.to_sql();
                self.wheres.push(("OR".to_string(), format!("EXISTS ({})", sql)));
                for binding in subquery.ordered_bindings() {
                    self.bindings.push(binding);
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
                if let Some(error) = subquery.validation_error() {
                    self.errors.push(error);
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_cte() — invalid CTE identifier: {}", e)));
                }
                let sql = subquery.to_sql();
                self.ctes.push(format!("{} AS ({})", cte_name, sql));
                for binding in subquery.ordered_bindings() {
                    self.cte_bindings.push(binding);
                }
                self
            }

            pub fn with_recursive<B: rullst_orm::schema::SubqueryBuilder>(mut self, cte_name: &str, subquery: B) -> Self {
                if let Some(error) = subquery.validation_error() {
                    self.errors.push(error);
                }
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive() — invalid CTE identifier: {}", e)));
                }
                let sql = subquery.to_sql();
                self.ctes.push(format!("{} AS ({})", cte_name, sql));
                self.has_recursive_cte = true;
                for binding in subquery.ordered_bindings() {
                    self.cte_bindings.push(binding);
                }
                self
            }
    }
}
