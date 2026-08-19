// src/builder/magic_methods.rs — Generates typed dynamic magic methods for model fields.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the magic methods for each field (where_field, order_by_field, etc)
#[cfg_attr(test, mutants::skip)]
pub fn generate_magic_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
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
