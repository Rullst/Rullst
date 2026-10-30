//! `FromRow` beside `#[derive(Orm)]`, and SQLx derive and query macros that
//! Rullst crates stopped enabling in 13.0.0-alpha.4.

use super::super::Rule;
use super::super::catalog::{ORM_FROM_ROW, SQLX_MACROS};
use super::{entries, has_key, line};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Token};

/// SQLx's function-like query and migration macros.
const QUERY_MACROS: &[&str] = &[
    "query",
    "query_as",
    "query_scalar",
    "query_unchecked",
    "query_as_unchecked",
    "query_scalar_unchecked",
    "query_file",
    "query_file_as",
    "query_file_scalar",
    "query_file_unchecked",
    "query_file_as_unchecked",
    "query_file_scalar_unchecked",
    "migrate",
];

fn last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default()
}

fn names_sqlx(path: &syn::Path) -> bool {
    path.segments.iter().any(|segment| segment.ident == "sqlx")
}

/// The rule and line of each derive that 13.0.0-alpha.4 breaks: `FromRow` on
/// an `Orm` model without `#[orm(custom_from_row)]`, and otherwise a SQLx
/// derive (`FromRow`, `sqlx::Type`, `sqlx::Encode`, `sqlx::Decode`) when no
/// manifest enables SQLx's macros (`sqlx_macros`).
pub(super) fn derive_findings(
    attributes: &[Attribute],
    sqlx_macros: bool,
) -> Vec<(&'static Rule, usize)> {
    let paths = attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("derive"))
        .filter_map(|attribute| {
            attribute
                .parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .collect::<Vec<_>>();
    let orm = paths.iter().any(|path| last(path) == "Orm");
    let custom = has_key(&entries(attributes, "orm"), &["custom_from_row"]);
    let mut found = Vec::new();
    for path in &paths {
        let at = line(path.span());
        match last(path).as_str() {
            "FromRow" if orm && !custom => found.push((&ORM_FROM_ROW, at)),
            "FromRow" if !sqlx_macros => found.push((&SQLX_MACROS, at)),
            "Type" | "Encode" | "Decode" if !sqlx_macros && names_sqlx(path) => {
                found.push((&SQLX_MACROS, at));
            }
            _ => {}
        }
    }
    found
}

/// `sqlx::query!` and the other SQLx macros, named through a `sqlx` path.
pub(super) fn is_query_macro(path: &syn::Path) -> bool {
    names_sqlx(path) && QUERY_MACROS.contains(&last(path).as_str())
}
