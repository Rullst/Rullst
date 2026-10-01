//! Generated search relevance order: `search()` keeps the ranking of the
//! IDs a Scout engine returned.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn generate(parsed: &ParsedModel) -> TokenStream {
    let id_column = format!("{}.id", parsed.table_name);
    quote! {
        /// Orders rows like the ranked `ids` of a search engine, through the
        /// `order_by` slot with bound IDs, so an explicit `order_by` replaces
        /// it. DISTINCT and GROUP BY statements, whose select list cannot
        /// carry the expression, omit it.
        fn __rullst_order_by_relevance(mut self, ids: &[i32]) -> Self {
            if ids.is_empty() {
                return self;
            }
            let mut order = String::with_capacity(32 + ids.len() * 20);
            order.push_str("CASE ");
            order.push_str(#id_column);
            for position in 0..ids.len() {
                order.push_str(" WHEN ? THEN ");
                order.push_str(&position.to_string());
            }
            order.push_str(" ELSE ");
            order.push_str(&ids.len().to_string());
            order.push_str(" END");
            self.order_bindings = ids.iter().map(|id| rullst_orm::RullstValue::Int(*id)).collect();
            self.order_by = Some(order.clone());
            self.relevance_order = Some(order);
            self
        }

        /// Whether `order_by` still holds the search relevance order.
        fn __rullst_has_relevance_order(&self) -> bool {
            self.relevance_order.is_some() && self.order_by == self.relevance_order
        }

        /// Whether the rendered statement omits the relevance order.
        fn __rullst_relevance_omitted(&self) -> bool {
            self.__rullst_has_relevance_order() && (self.is_distinct || self.group_by.is_some())
        }

        /// Values of the rendered ORDER BY clause.
        fn __rullst_order_bindings(&self) -> &[rullst_orm::RullstValue] {
            if self.__rullst_relevance_omitted() {
                &[]
            } else {
                &self.order_bindings
            }
        }
    }
}
