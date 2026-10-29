//! The model's `FromRow` implementation.
//!
//! Since 13.0.0-alpha.4 `#[derive(Orm)]` implements `sqlx::FromRow` itself,
//! so applications need neither SQLx's `derive` feature nor its
//! `sqlx-macros` compile-time crate. The impl is generic over the row type
//! like SQLx's own derive, and reads the options `#[derive(Orm)]` accepts with
//! the same meaning: field `#[sqlx(skip)]`, `#[sqlx(default)]`,
//! `#[sqlx(json)]` and `#[sqlx(json(nullable))]`, and struct-level
//! `#[sqlx(default)]`. Relation fields are never columns and start empty.
//! `#[orm(custom_from_row)]` leaves `FromRow` to the model.

use crate::parser::{ParsedModel, RowRead};
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate(parsed: &ParsedModel) -> TokenStream {
    if parsed.custom_from_row {
        return TokenStream::new();
    }
    let name = &parsed.name;
    let database = quote! { <__RullstRow as rullst_orm::_sqlx::Row>::Database };
    let mut predicates = Vec::new();
    let mut reads = Vec::new();
    for field in &parsed.row_fields {
        let ident = &field.name;
        let ty = &field.ty;
        let RowRead::Column { json, default } = field.read else {
            reads.push(quote! {
                let #ident: #ty = ::core::default::Default::default();
            });
            continue;
        };
        let column = ident.to_string();
        let column = column.strip_prefix("r#").unwrap_or(&column);
        let (decoded, read) = match json {
            None => (
                quote! { #ty },
                quote! { rullst_orm::_sqlx::Row::try_get::<#ty, _>(__rullst_row, #column) },
            ),
            Some(false) => (
                quote! { rullst_orm::_sqlx::types::Json<#ty> },
                quote! {
                    rullst_orm::_sqlx::Row::try_get::<rullst_orm::_sqlx::types::Json<#ty>, _>(__rullst_row, #column)
                        .map(|value| value.0)
                },
            ),
            // As in SQLx: the field is an `Option`, and SQL NULL as well as
            // a JSON `null` decode to `None`.
            Some(true) => (
                quote! { ::core::option::Option<rullst_orm::_sqlx::types::Json<#ty>> },
                quote! {
                    rullst_orm::_sqlx::Row::try_get::<::core::option::Option<rullst_orm::_sqlx::types::Json<#ty>>, _>(__rullst_row, #column)
                        .map(|value| value.and_then(|json| json.0))
                },
            ),
        };
        predicates.push(quote! {
            #decoded: rullst_orm::_sqlx::Decode<'__rullst_row, #database>
                + rullst_orm::_sqlx::Type<#database>
        });
        let missing = if default {
            Some(quote! { ::core::default::Default::default() })
        } else if parsed.row_default {
            Some(quote! { __rullst_default.#ident })
        } else {
            None
        };
        reads.push(match missing {
            Some(missing) => quote! {
                let #ident: #ty = #read.or_else(|error| match error {
                    rullst_orm::_sqlx::Error::ColumnNotFound(_) => ::core::result::Result::Ok(#missing),
                    error => ::core::result::Result::Err(error),
                })?;
            },
            None => quote! { let #ident: #ty = #read?; },
        });
    }
    let default_instance = parsed.row_default.then(|| {
        predicates.push(quote! { #name: ::core::default::Default });
        quote! { let __rullst_default = <Self as ::core::default::Default>::default(); }
    });
    let fields = parsed.row_fields.iter().map(|field| &field.name);

    quote! {
        #[automatically_derived]
        impl<'__rullst_row, __RullstRow> rullst_orm::_sqlx::FromRow<'__rullst_row, __RullstRow> for #name
        where
            __RullstRow: rullst_orm::_sqlx::Row,
            &'__rullst_row ::core::primitive::str: rullst_orm::_sqlx::ColumnIndex<__RullstRow>,
            #(#predicates,)*
        {
            fn from_row(
                __rullst_row: &'__rullst_row __RullstRow,
            ) -> ::core::result::Result<Self, rullst_orm::_sqlx::Error> {
                #default_instance
                #(#reads)*
                ::core::result::Result::Ok(Self { #(#fields),* })
            }
        }
    }
}

#[cfg(test)]
#[path = "from_row_tests.rs"]
mod tests;
