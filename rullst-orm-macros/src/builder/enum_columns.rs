//! Typed bind markers for `#[derive(Enum)]` columns on PostgreSQL.
//!
//! Filter values reach SQLx as `RullstValue::String`, a `text` parameter, and
//! PostgreSQL has no `enum = text` operator for a typed parameter. A column
//! whose persisted field type implements `DatabaseEnum` therefore renders its
//! marker as `CAST(? AS "<type_name>")` there. The macro cannot see whether a
//! field type is such an enum, so the generated code asks the compiler: local
//! autoref dispatch selects the `DatabaseEnum` impl for the concrete field
//! type when it exists and a `None` fallback otherwise.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

/// Field types that are never database enums; they get no probe arm.
const PLAIN_TYPES: &[&str] = &[
    "String",
    "SecretString",
    "bool",
    "i8",
    "i16",
    "i32",
    "i64",
    "u8",
    "u16",
    "u32",
    "u64",
    "f32",
    "f64",
];

/// The type a column's enum probe inspects: `T` for `T` and `Option<T>`.
fn probed_type(field_type: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(path) = field_type else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident == "Option" {
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return None;
        };
        return match arguments.args.first() {
            Some(syn::GenericArgument::Type(inner)) if arguments.args.len() == 1 => {
                probed_type(inner)
            }
            _ => None,
        };
    }
    if PLAIN_TYPES.iter().any(|plain| segment.ident == plain) {
        return None;
    }
    Some(field_type)
}

#[cfg_attr(test, mutants::skip)]
pub(super) fn generate(parsed: &ParsedModel) -> TokenStream {
    let table_name = &parsed.table_name;
    let arms = parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
        .filter_map(|(field, field_type)| {
            let probed = probed_type(field_type)?;
            let column = field.to_string();
            Some(quote! {
                #column => (&__RullstEnumProbe::<#probed>(::core::marker::PhantomData)).__rullst_enum_type(),
            })
        })
        .collect::<Vec<_>>();
    let lookup = if arms.is_empty() {
        quote! {
            let _ = column;
            None
        }
    } else {
        quote! {
            struct __RullstEnumProbe<T>(::core::marker::PhantomData<T>);
            trait __RullstDatabaseEnum {
                fn __rullst_enum_type(&self) -> Option<&'static str>;
            }
            impl<T: rullst_orm::DatabaseEnum> __RullstDatabaseEnum for __RullstEnumProbe<T> {
                fn __rullst_enum_type(&self) -> Option<&'static str> {
                    Some(T::TYPE_NAME)
                }
            }
            trait __RullstPlainColumn {
                fn __rullst_enum_type(&self) -> Option<&'static str>;
            }
            impl<T> __RullstPlainColumn for &__RullstEnumProbe<T> {
                fn __rullst_enum_type(&self) -> Option<&'static str> {
                    None
                }
            }
            let column = match column.rsplit_once('.') {
                Some((table, column)) if table == #table_name => column,
                Some(_) => return None,
                None => column,
            };
            match column {
                #(#arms)*
                _ => None,
            }
        }
    };
    quote! {
        /// The database enum type of one of this model's columns, if any.
        fn __rullst_enum_type(column: &str) -> Option<&'static str> {
            #lookup
        }

        /// The bind marker for a value compared with `column`: a cast to
        /// the column's named enum type on PostgreSQL, `?` otherwise.
        fn __rullst_bind_marker(column: &str) -> std::borrow::Cow<'static, str> {
            let enum_type = Self::__rullst_enum_type(column).filter(|type_name| {
                !type_name.is_empty()
                    && type_name.len() <= 63
                    && type_name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            });
            match (enum_type, rullst_orm::Orm::driver()) {
                (Some(type_name), Ok("postgres")) => {
                    std::borrow::Cow::Owned(format!("CAST(? AS \"{}\")", type_name))
                }
                _ => std::borrow::Cow::Borrowed("?"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn only_non_primitive_columns_are_probed() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "accounts")]
            struct Account {
                id: i32,
                name: String,
                status: AccountStatus,
                previous: Option<AccountStatus>,
                score: Option<f64>,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let generated = super::generate(&parsed).to_string();
        assert!(generated.contains("\"status\" => (& __RullstEnumProbe :: < AccountStatus >"));
        assert!(generated.contains("\"previous\" => (& __RullstEnumProbe :: < AccountStatus >"));
        for plain in ["\"id\" =>", "\"name\" =>", "\"score\" =>"] {
            assert!(!generated.contains(plain), "{plain}");
        }
        assert!(generated.contains("CAST(? AS \\\"{}\\\")"));

        let plain: DeriveInput = parse_quote! {
            struct Plain { id: i32, name: String }
        };
        let parsed = crate::parser::parse(&plain).expect("test model should parse");
        assert!(
            !super::generate(&parsed)
                .to_string()
                .contains("__RullstEnumProbe")
        );
    }
}
