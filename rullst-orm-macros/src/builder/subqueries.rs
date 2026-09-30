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

            /// Adds a caller-owned raw CTE without bind markers; use
            /// [`Self::with_raw_bindings`] for a parameterized fragment.
            pub fn with_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_raw() — invalid CTE identifier: {}", e)));
                }
                self.__rullst_reject_raw_markers("with_raw", query);
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self
            }

            /// Recursive variant of [`Self::with_raw`].
            pub fn with_recursive_raw(mut self, cte_name: &str, query: &str) -> Self {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("with_recursive_raw() — invalid CTE identifier: {}", e)));
                }
                self.__rullst_reject_raw_markers("with_recursive_raw", query);
                self.ctes.push(format!("{} AS ({})", cte_name, query));
                self.has_recursive_cte = true;
                self
            }

            /// Adds a caller-owned raw CTE whose `?` (or `$n`) markers take
            /// `bindings`; they are bound at the CTE's textual position.
            pub fn with_raw_bindings<V: Into<rullst_orm::RullstValue>>(mut self, cte_name: &str, query: &str, bindings: Vec<V>) -> Self {
                self.__rullst_push_raw_cte("with_raw_bindings", cte_name, query, bindings.into_iter().map(Into::into).collect());
                self
            }

            /// Recursive variant of [`Self::with_raw_bindings`].
            pub fn with_recursive_raw_bindings<V: Into<rullst_orm::RullstValue>>(mut self, cte_name: &str, query: &str, bindings: Vec<V>) -> Self {
                self.__rullst_push_raw_cte("with_recursive_raw_bindings", cte_name, query, bindings.into_iter().map(Into::into).collect());
                self.has_recursive_cte = true;
                self
            }

            /// A raw fragment rendered before FROM cannot take `bind()` values:
            /// those are WHERE bindings, so the scope binding would move into
            /// the fragment's marker. Its values need the `_bindings` variant.
            fn __rullst_reject_raw_markers(&mut self, method: &str, query: &str) {
                if rullst_orm::raw_fragment(query, Vec::new()).is_err() {
                    self.errors.push(rullst_orm::Error::Validation(format!(
                        "{}() SQL contains bind markers, but bind() values belong to WHERE fragments; pass the fragment's values to {}_bindings(...)",
                        method,
                        method,
                    )));
                }
            }

            fn __rullst_push_raw_cte(
                &mut self,
                method: &str,
                cte_name: &str,
                query: &str,
                bindings: Vec<rullst_orm::RullstValue>,
            ) {
                if let Err(e) = rullst_orm::schema::validate_identifier(cte_name) {
                    self.errors.push(rullst_orm::Error::Validation(format!("{}() — invalid CTE identifier: {}", method, e)));
                }
                match rullst_orm::raw_fragment(query, bindings) {
                    Ok((sql, ordered)) => {
                        self.ctes.push(format!("{} AS ({})", cte_name, sql));
                        self.cte_bindings.extend(ordered);
                    }
                    Err(error) => self.errors.push(error),
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
