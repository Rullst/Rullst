//! Bounded `FieldKind::Integer` metadata for Rust integer field types.

use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;
use syn::Type;

/// The inclusive range Nexus accepts for an integer type (`Option<T>`
/// already unwrapped). `isize`/`usize` follow the 64-bit server targets, and
/// `u64`/`usize` stop at `i64::MAX`, the largest value an SQL `BIGINT` and the
/// SQLx drivers Rullst uses can hold.
fn integer_range(type_name: &str) -> Option<(i64, i64)> {
    Some(match type_name {
        "i8" => (i8::MIN.into(), i8::MAX.into()),
        "i16" => (i16::MIN.into(), i16::MAX.into()),
        "i32" => (i32::MIN.into(), i32::MAX.into()),
        "i64" | "isize" => (i64::MIN, i64::MAX),
        "u8" => (0, u8::MAX.into()),
        "u16" => (0, u16::MAX.into()),
        "u32" => (0, u32::MAX.into()),
        "u64" | "usize" => (0, i64::MAX),
        _ => return None,
    })
}

pub(super) fn is_integer_type(field_type: &Type) -> bool {
    integer_range(&super::unwrapped_type_name(field_type)).is_some()
}

/// One bound as tokens; the extremes use the named constants rather than a
/// literal whose magnitude only fits after negation.
fn bound(value: i64) -> TokenStream2 {
    match value {
        i64::MIN => quote!(::core::primitive::i64::MIN),
        i64::MAX => quote!(::core::primitive::i64::MAX),
        _ => {
            let literal = Literal::i64_suffixed(value);
            quote!(#literal)
        }
    }
}

/// `FieldKind::Integer` with the range of `field_type`, when it is a Rust
/// integer type.
pub(super) fn integer_field_kind(field_type: &Type) -> Option<TokenStream2> {
    let (min, max) = integer_range(&super::unwrapped_type_name(field_type))?;
    let (min, max) = (bound(min), bound(max));
    Some(quote!(::rullst::nexus::FieldKind::Integer { min: #min, max: #max }))
}
