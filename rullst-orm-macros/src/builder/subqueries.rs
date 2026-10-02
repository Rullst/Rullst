//! Typed and raw subqueries retain their distinct validation boundaries.
//!
//! Typed subqueries are embedded with portable `?` markers. PostgreSQL `$n`
//! numbering happens exactly once, when the outermost statement is rendered,
//! so it follows textual order and therefore the ordered binding vector.

use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_subquery_methods() -> TokenStream {
    quote! {
            /// Converts a typed subquery into a portable SQL fragment plus its
            /// bindings in textual order. Failures are recorded on the builder.
            fn __rullst_portable_subquery<B: rullst_orm::schema::SubqueryBuilder>(
                &mut self,
                subquery: &B,
            ) -> Option<(String, Vec<rullst_orm::RullstValue>)> {
                if let Some(error) = subquery.validation_error() {
                    self.errors.push(error);
                }
                match rullst_orm::portable_subquery(&subquery.to_sql(), subquery.ordered_bindings()) {
                    Ok(fragment) => Some(fragment),
                    Err(error) => {
                        self.errors.push(error);
                        None
                    }
                }
            }

            pub fn where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let Some((sql, bindings)) = self.__rullst_portable_subquery(&subquery) else {
                    self.wheres.push(("AND".to_string(), "1 = 0".to_string()));
                    return self;
                };
                self.wheres.push(("AND".to_string(), format!("EXISTS ({})", sql)));
                self.bindings.extend(bindings);
                self
            }

            pub fn or_where_exists<B: rullst_orm::schema::SubqueryBuilder>(mut self, subquery: B) -> Self {
                let Some((sql, bindings)) = self.__rullst_portable_subquery(&subquery) else {
                    self.wheres.push(("OR".to_string(), "1 = 0".to_string()));
                    return self;
                };
                self.wheres.push(("OR".to_string(), format!("EXISTS ({})", sql)));
                self.bindings.extend(bindings);
                self
            }

            /// Adds a caller-owned raw CTE. On a query that already holds a
            /// tenant/model-wide scope, JOIN or WHERE binding the fragment must
            /// not contain bind markers: `bind()` values are appended after
            /// those bindings, while the CTE renders before them.
            pub fn with_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_raw() — invalid CTE identifier: {}", e)));
                }
                self.__rullst_reject_displaced_raw_markers("with_raw", query);
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self
            }

            /// Recursive variant of [`Self::with_raw`], with the same bind
            /// marker restriction.
            pub fn with_recursive_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive_raw() — invalid CTE identifier: {}", e)));
                }
                self.__rullst_reject_displaced_raw_markers("with_recursive_raw", query);
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self.has_recursive_cte = true;
                self
            }

            /// A raw fragment rendered before FROM can only take `bind()`
            /// values, which are appended after the scope, JOIN and WHERE
            /// bindings already present. Its markers would then consume those
            /// earlier values (the mandatory tenant binding first), so such a
            /// fragment is rejected while any of them exists.
            fn __rullst_reject_displaced_raw_markers(&mut self, method: &str, query: &str) {
                if self.scope_bindings.is_empty()
                    && self.join_bindings.is_empty()
                    && self.bindings.is_empty()
                {
                    return;
                }
                let has_markers = rullst_orm::replace_placeholders(query) != query
                    || rullst_orm::portable_subquery(query, Vec::new()).is_err();
                if has_markers {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "{}() SQL contains bind markers, but this query already holds tenant, scope, JOIN or WHERE bindings that bind() values cannot precede; use a typed with_cte()/where_raw() with explicit bindings instead",
                        method,
                    )));
                }
            }

            pub fn with_cte<B: rullst_orm::schema::SubqueryBuilder>(mut self, cte_name: &str, subquery: B) -> Self {
                let fragment = self.__rullst_portable_subquery(&subquery);
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_cte() — invalid CTE identifier: {}", e)));
                }
                if let Some((sql, bindings)) = fragment {
                    self.ctes.push(format!("{} AS ({})", cte_name, sql));
                    self.cte_bindings.extend(bindings);
                }
                self
            }

            pub fn with_recursive<B: rullst_orm::schema::SubqueryBuilder>(mut self, cte_name: &str, subquery: B) -> Self {
                let fragment = self.__rullst_portable_subquery(&subquery);
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive() — invalid CTE identifier: {}", e)));
                }
                if let Some((sql, bindings)) = fragment {
                    self.ctes.push(format!("{} AS ({})", cte_name, sql));
                    self.cte_bindings.extend(bindings);
                }
                self.has_recursive_cte = true;
                self
            }
    }
}
