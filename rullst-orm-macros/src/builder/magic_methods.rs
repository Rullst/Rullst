// src/builder/magic_methods.rs — Generates typed dynamic magic methods for model fields.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the magic methods for each field (where_field, order_by_field, etc)
#[cfg_attr(test, mutants::skip)]
pub fn generate_magic_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let mut magic_methods = vec![];
    for (field_name, field_type) in parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
    {
        let field_name_str = field_name.to_string();

        let where_method = quote::format_ident!("where_{}", field_name);
        let or_where_method = quote::format_ident!("or_where_{}", field_name);
        let where_not_method = quote::format_ident!("where_not_{}", field_name);

        let primitive_ident = match field_type {
            syn::Type::Path(path) if path.qself.is_none() => {
                path.path.segments.last().map(|segment| &segment.ident)
            }
            _ => None,
        };

        if primitive_ident.is_some_and(|ident| ident == "String") {
            magic_methods.push(quote! {
                pub fn #where_method(self, value: impl Into<String>) -> Self {
                    self.where_eq(#field_name_str, value.into())
                }
                pub fn #or_where_method(self, value: impl Into<String>) -> Self {
                    self.or_where(#field_name_str, value.into())
                }
                pub fn #where_not_method(self, value: impl Into<String>) -> Self {
                    self.where_not_eq(#field_name_str, value.into())
                }
            });
        } else if primitive_ident
            .is_some_and(|ident| ident == "i32" || ident == "f64" || ident == "bool")
        {
            magic_methods.push(quote! {
                pub fn #where_method(self, value: #field_type) -> Self {
                    self.where_eq(#field_name_str, value)
                }
                pub fn #or_where_method(self, value: #field_type) -> Self {
                    self.or_where(#field_name_str, value)
                }
                pub fn #where_not_method(self, value: #field_type) -> Self {
                    self.where_not_eq(#field_name_str, value)
                }
            });
        } else {
            // Custom SQLx types remain available through the explicit dynamic
            // value boundary until they implement a portable RullstValue
            // conversion. The column name itself is still generated.
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
        }

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

#[cfg(test)]
mod tests {
    use super::generate_magic_methods;
    use crate::parser;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn primitive_magic_filters_use_the_persisted_field_type() {
        let input: DeriveInput = parse_quote! {
            struct Record {
                id: i32,
                name: String,
                score: f64,
                active: bool,
                metadata: Json<Value>,
            }
        };
        let parsed = parser::parse(&input).expect("parse model");
        let generated = generate_magic_methods(&parsed)
            .into_iter()
            .map(|tokens| tokens.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(generated.contains("where_id (self , value : i32)"));
        assert!(generated.contains("where_name (self , value : impl Into < String >)"));
        assert!(generated.contains("where_score (self , value : f64)"));
        assert!(generated.contains("where_active (self , value : bool)"));
        assert!(generated.contains("where_metadata < T : Into < rullst_orm :: RullstValue"));
    }
}
