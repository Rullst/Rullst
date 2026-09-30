// src/builder/magic_methods.rs — Generates typed dynamic magic methods for model fields.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Inherent query-builder methods whose names a per-column helper could
/// reproduce (`where_{column}`, `or_where_{column}`, `where_not_{column}`,
/// `order_by_{column}` or `order_by_{column}_desc`). A column such as `raw` or
/// `desc` would otherwise redefine `where_raw` or `order_by_desc`.
const FIXED_BUILDER_METHODS: &[&str] = &[
    "or_where_between",
    "or_where_exists",
    "or_where_gt",
    "or_where_in",
    "or_where_like",
    "or_where_lt",
    "or_where_not_eq",
    "or_where_not_null",
    "or_where_null",
    "or_where_raw",
    "order_by_col",
    "order_by_cosine_distance",
    "order_by_desc",
    "order_by_desc_col",
    "order_by_inner_product",
    "order_by_l2_distance",
    "order_by_similarity",
    "where_between",
    "where_col",
    "where_column",
    "where_eq",
    "where_exists",
    "where_gt",
    "where_in",
    "where_like",
    "where_lt",
    "where_not_between",
    "where_not_eq",
    "where_not_in",
    "where_not_like",
    "where_not_null",
    "where_null",
    "where_raw",
    "where_similar",
];

fn helper_names(field_name: &syn::Ident) -> [syn::Ident; 5] {
    [
        quote::format_ident!("where_{}", field_name),
        quote::format_ident!("or_where_{}", field_name),
        quote::format_ident!("where_not_{}", field_name),
        quote::format_ident!("order_by_{}", field_name),
        quote::format_ident!("order_by_{}_desc", field_name),
    ]
}

/// Generates the magic methods for each field (where_field, order_by_field, etc).
///
/// A helper whose name matches a fixed builder method, or that two columns
/// would both produce (`x` gives `where_not_x`, and so does a `not_x` column
/// through `where_{column}`), is not generated; the column stays reachable
/// through the string-column builders such as `where_eq("raw", ...)`.
#[cfg_attr(test, mutants::skip)]
pub fn generate_magic_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let mut produced = std::collections::HashMap::<String, usize>::new();
    for field_name in &parsed.normal_fields {
        for helper in helper_names(field_name) {
            *produced.entry(helper.to_string()).or_default() += 1;
        }
    }
    let available = |helper: &syn::Ident| {
        let helper = helper.to_string();
        !FIXED_BUILDER_METHODS.contains(&helper.as_str()) && produced.get(&helper) == Some(&1)
    };

    let mut magic_methods = vec![];
    for (field_name, field_type) in parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
    {
        let field_name_str = field_name.to_string();
        let [
            where_method,
            or_where_method,
            where_not_method,
            order_by_method,
            order_by_desc_method,
        ] = helper_names(field_name);

        let primitive_ident = match field_type {
            syn::Type::Path(path) if path.qself.is_none() => {
                path.path.segments.last().map(|segment| &segment.ident)
            }
            _ => None,
        };

        let (generics, value_type, value) =
            if primitive_ident.is_some_and(|ident| ident == "String") {
                (
                    quote! {},
                    quote! { impl Into<String> },
                    quote! { value.into() },
                )
            } else if primitive_ident
                .is_some_and(|ident| ident == "i32" || ident == "f64" || ident == "bool")
            {
                (quote! {}, quote! { #field_type }, quote! { value })
            } else {
                // Custom SQLx types remain available through the explicit dynamic
                // value boundary until they implement a portable RullstValue
                // conversion. The column name itself is still generated.
                (
                    quote! { <T: Into<rullst_orm::RullstValue>> },
                    quote! { T },
                    quote! { value },
                )
            };

        for (method, target) in [
            (where_method, quote! { where_eq }),
            (or_where_method, quote! { or_where }),
            (where_not_method, quote! { where_not_eq }),
        ] {
            if available(&method) {
                magic_methods.push(quote! {
                    pub fn #method #generics(self, value: #value_type) -> Self {
                        self.#target(#field_name_str, #value)
                    }
                });
            }
        }
        for (method, target) in [
            (order_by_method, quote! { order_by }),
            (order_by_desc_method, quote! { order_by_desc }),
        ] {
            if available(&method) {
                magic_methods.push(quote! {
                    pub fn #method(self) -> Self {
                        self.#target(#field_name_str)
                    }
                });
            }
        }
    }
    magic_methods
}

#[cfg(test)]
mod tests {
    use super::{FIXED_BUILDER_METHODS, generate_magic_methods};
    use crate::parser;
    use proc_macro2::{TokenStream, TokenTree};
    use syn::{DeriveInput, parse_quote};

    fn function_names(tokens: TokenStream, names: &mut Vec<String>) {
        let mut after_fn = false;
        for token in tokens {
            match token {
                TokenTree::Ident(ident) => {
                    if after_fn {
                        names.push(ident.to_string());
                    }
                    after_fn = ident == "fn";
                }
                TokenTree::Group(group) => {
                    after_fn = false;
                    function_names(group.stream(), names);
                }
                _ => after_fn = false,
            }
        }
    }

    #[test]
    fn fixed_builder_methods_match_the_generated_builder() {
        let input: DeriveInput = parse_quote! {
            struct Record { id: i32 }
        };
        let parsed = parser::parse(&input).expect("parse model");
        let builder = crate::builder::generate(&parsed, &[], &[], &[], &TokenStream::new());
        let mut names = Vec::new();
        function_names(builder, &mut names);
        let magic = [
            "where_id",
            "or_where_id",
            "where_not_id",
            "order_by_id",
            "order_by_id_desc",
        ];
        let mut fixed = names
            .into_iter()
            .filter(|name| {
                ["where_", "or_where_", "order_by_"]
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
                    && !magic.contains(&name.as_str())
            })
            .collect::<Vec<_>>();
        fixed.sort();
        fixed.dedup();
        assert_eq!(fixed, FIXED_BUILDER_METHODS);
    }

    #[test]
    fn colliding_magic_helpers_are_not_generated() {
        let input: DeriveInput = parse_quote! {
            struct Record {
                id: i32,
                raw: String,
                desc: String,
                exists: String,
                column: String,
                similar: String,
                col: String,
                x: i32,
                not_x: i32,
                created: String,
                created_desc: String,
            }
        };
        let parsed = parser::parse(&input).expect("parse model");
        let generated = generate_magic_methods(&parsed)
            .into_iter()
            .map(|tokens| tokens.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        for skipped in [
            "where_raw",
            "or_where_raw",
            "order_by_desc",
            "where_exists",
            "or_where_exists",
            "where_column",
            "where_similar",
            "order_by_similarity",
            "where_col",
            "order_by_col",
            "where_not_x",
            "order_by_created_desc",
        ] {
            assert!(!generated.contains(&format!("fn {skipped} ")), "{skipped}");
        }
        for kept in [
            "where_not_raw",
            "order_by_raw",
            "where_desc",
            "order_by_desc_desc",
            "where_x",
            "where_not_not_x",
            "order_by_created",
            "where_created_desc",
        ] {
            assert!(generated.contains(&format!("fn {kept} ")), "{kept}");
        }
    }

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
