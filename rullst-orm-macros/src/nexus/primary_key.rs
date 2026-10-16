//! The record key of a `#[derive(Nexus)]` model: one column or several.

use syn::LitStr;

use super::FieldOptions;

/// Most columns Nexus accepts in one primary key.
const MAX_KEY_COLUMNS: usize = 8;

/// Parses a struct-level `primary_key = "user_id, lesson_id"`: distinct
/// column names separated by commas, at most eight.
pub(super) fn parse_key_list(literal: &LitStr) -> syn::Result<Vec<String>> {
    let value = literal.value();
    let columns = value
        .split(',')
        .map(|column| column.trim().to_string())
        .collect::<Vec<_>>();
    if columns.iter().any(String::is_empty) {
        return Err(syn::Error::new_spanned(
            literal,
            "Nexus `primary_key` lists one or more column names separated by commas",
        ));
    }
    if columns.len() > MAX_KEY_COLUMNS {
        return Err(syn::Error::new_spanned(
            literal,
            "a Nexus primary key has at most 8 columns",
        ));
    }
    for (index, column) in columns.iter().enumerate() {
        if columns[..index].contains(column) {
            return Err(syn::Error::new_spanned(
                literal,
                format!("Nexus primary-key column `{column}` is listed more than once"),
            ));
        }
    }
    Ok(columns)
}

/// The key columns: the struct-level `primary_key`, else every field
/// annotated `#[nexus(primary_key)]` in declaration order, else `id`. An
/// annotation outranks the `id` naming convention whatever the field order;
/// annotations that do not name exactly the struct-level columns, in order,
/// fail compilation.
pub(super) fn resolve_primary_key(
    configured: Option<Vec<String>>,
    columns: &[(&syn::Field, String, FieldOptions)],
) -> syn::Result<Vec<String>> {
    let annotated = columns
        .iter()
        .filter(|(_, _, options)| options.primary_key)
        .map(|(field, name, _)| (*field, name))
        .collect::<Vec<_>>();
    let annotated_names = annotated
        .iter()
        .map(|(_, name)| (*name).clone())
        .collect::<Vec<_>>();
    let Some(configured) = configured else {
        if annotated_names.is_empty() {
            return Ok(vec!["id".to_string()]);
        }
        return Ok(annotated_names);
    };
    if annotated_names.is_empty() || annotated_names == configured {
        return Ok(configured);
    }
    // Point at an annotated field outside the list, else at the first one.
    let (field, name) = annotated
        .iter()
        .find(|(_, name)| !configured.contains(name))
        .or(annotated.first())
        .copied()
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing primary key"))?;
    Err(syn::Error::new_spanned(
        field,
        format!(
            "#[nexus(primary_key)] on `{name}` contradicts the struct-level primary key `{}`",
            configured.join(", ")
        ),
    ))
}

#[cfg(test)]
#[path = "primary_key_tests.rs"]
mod tests;
