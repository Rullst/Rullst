use std::collections::HashSet;
use syn::LitStr;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;

#[cfg(test)]
pub(in crate::parser) fn split_top_level(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut depth = 0_i32;
    for character in input.chars() {
        match character {
            '(' => {
                depth += 1;
                buffer.push(character);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                buffer.push(character);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut buffer)),
            other => buffer.push(other),
        }
    }
    if !buffer.is_empty() {
        parts.push(buffer);
    }
    parts
}

#[cfg(test)]
pub(in crate::parser) fn strip_outer_call(input: &str, name: &str) -> Option<String> {
    let rest = input.trim().strip_prefix(name)?.trim_start();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    Some(rest[1..rest.len() - 1].trim().to_string())
}

pub(in crate::parser) fn validate_relation_attribute(
    key: &str,
    value: &str,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    match key {
        "has_many" | "has_one" | "belongs_to" | "belongs_to_many" | "morph_many" | "morph_one"
        | "morph_to" => {
            let portable_model = value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                && value.bytes().all(|byte| byte.is_ascii_alphanumeric());
            if !portable_model || syn::parse_str::<syn::Ident>(value).is_err() {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "relation `{key}` requires one PascalCase Rust model identifier (for example `#[orm({key} = \"User\")]`)"
                    ),
                ));
            }
        }
        "foreign_key" | "related_key" | "pivot_table" | "local_key" | "name" | "morph_name"
            if value.is_empty() =>
        {
            return Err(syn::Error::new(
                span,
                format!("attribute `{key}` requires a non-empty string value"),
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn identifier_value(
    meta: &ParseNestedMeta<'_>,
    seen: &mut HashSet<&'static str>,
    key: &'static str,
) -> Result<String, syn::Error> {
    mark_once(seen, key, meta)?;
    let value = string_value(meta)?;
    validate_rust_identifier(&value, key, meta.path.span())?;
    Ok(value)
}

pub(super) fn relation_identifier(
    meta: &ParseNestedMeta<'_>,
    seen: &mut HashSet<&'static str>,
    key: &'static str,
) -> Result<String, syn::Error> {
    mark_once(seen, key, meta)?;
    let value = string_value(meta)?;
    validate_relation_attribute(key, &value, meta.path.span())?;
    validate_sql_identifier(&value, key, meta.path.span())?;
    validate_rust_identifier(&value, key, meta.path.span())?;
    Ok(value)
}

pub(super) fn string_value(meta: &ParseNestedMeta<'_>) -> Result<String, syn::Error> {
    let literal = meta.value()?.parse::<LitStr>()?;
    let value = literal.value();
    if value.is_empty() {
        return Err(meta.error("attribute value cannot be empty"));
    }
    if value.len() > 256 {
        return Err(meta.error("attribute value exceeds 256 bytes"));
    }
    Ok(value)
}

pub(super) fn path_name(meta: &ParseNestedMeta<'_>) -> Result<String, syn::Error> {
    meta.path
        .get_ident()
        .map(ToString::to_string)
        .ok_or_else(|| meta.error("attribute option must be a single identifier"))
}

pub(super) fn mark_once(
    seen: &mut HashSet<&'static str>,
    key: &'static str,
    meta: &ParseNestedMeta<'_>,
) -> Result<(), syn::Error> {
    if !seen.insert(key) {
        return Err(meta.error(format!("duplicate `{key}` attribute option")));
    }
    Ok(())
}

pub(super) fn validate_rust_identifier(
    value: &str,
    label: &str,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    if syn::parse_str::<syn::Ident>(value).is_err() {
        return Err(syn::Error::new(
            span,
            format!("{label} must be a single valid Rust identifier"),
        ));
    }
    Ok(())
}

pub(super) fn validate_sql_identifier(
    value: &str,
    label: &str,
    span: proc_macro2::Span,
) -> Result<(), syn::Error> {
    let valid = value.len() <= 64
        && value
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    if !valid {
        return Err(syn::Error::new(
            span,
            format!(
                "{label} must contain 1-64 ASCII letters, digits, or underscores and cannot start with a digit"
            ),
        ));
    }
    Ok(())
}

pub(super) fn typo(meta: &ParseNestedMeta<'_>, actual: &str, expected: &str) -> syn::Error {
    meta.error(format!(
        "unknown attribute `#[orm({actual} = ...)]`; did you mean `#[orm({expected} = \"...\")]`?"
    ))
}
