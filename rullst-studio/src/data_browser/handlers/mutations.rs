//! Fail-closed, primitive-value row mutations for the local Studio browser.

use super::super::db::{
    StudioColumn, StudioColumnKind, ensure_pool_initialized, escape_html_attr, fetch_table_schema,
    fetch_tables, get_any_value_as_string, is_safe_identifier, quote_table_name,
};
use crate::access::VerifiedLocalStudioAccess;
use axum::{
    Form,
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use sqlx::QueryBuilder;
use std::collections::BTreeMap;
use std::fmt::Write;

const MAX_FORM_FIELDS: usize = 260;
const MAX_CELL_BYTES: usize = 16 * 1024;

#[derive(Debug)]
enum MutationFailure {
    Invalid(&'static str),
    NotFound,
    Conflict,
    Database,
}

enum BoundValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null(StudioColumnKind),
}

pub(crate) async fn handle_table_update(
    Path(table): Path<String>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(fields): Form<Vec<(String, String)>>,
) -> Response {
    if verified.is_none() {
        return (
            StatusCode::FORBIDDEN,
            "Verified local Studio access is required",
        )
            .into_response();
    }
    match update_row(&table, fields).await {
        Ok(()) => {
            Redirect::to(&format!("/studio/tables/{}", urlencoding::encode(&table))).into_response()
        }
        Err(error) => mutation_error_response(error),
    }
}

pub(crate) async fn handle_table_delete(
    Path(table): Path<String>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
    Form(fields): Form<Vec<(String, String)>>,
) -> Response {
    if verified.is_none() {
        return (
            StatusCode::FORBIDDEN,
            "Verified local Studio access is required",
        )
            .into_response();
    }
    match delete_row(&table, fields).await {
        Ok(()) => {
            Redirect::to(&format!("/studio/tables/{}", urlencoding::encode(&table))).into_response()
        }
        Err(error) => mutation_error_response(error),
    }
}

async fn update_row(table: &str, fields: Vec<(String, String)>) -> Result<(), MutationFailure> {
    let (pool, driver, columns) = mutation_context(table).await?;
    let mut fields = unique_fields(fields)?;
    let column_name = take_required(&mut fields, "column")?;
    let set_null = match fields.remove("set_null") {
        None => false,
        Some(value) if matches!(value.as_str(), "true" | "on" | "1") => true,
        Some(_) => {
            return Err(MutationFailure::Invalid(
                "The NULL selector has an unsupported value",
            ));
        }
    };
    let raw_value = fields
        .remove("value")
        .ok_or(MutationFailure::Invalid("A replacement value is required"))?;
    let column = columns
        .iter()
        .find(|candidate| candidate.name == column_name)
        .ok_or(MutationFailure::Invalid("Unknown table column"))?;
    if column.primary_key || !column.kind.is_editable() {
        return Err(MutationFailure::Invalid(
            "Primary keys and backend-specific values are read-only",
        ));
    }
    let value = if set_null {
        if !column.nullable {
            return Err(MutationFailure::Invalid("This column does not accept NULL"));
        }
        BoundValue::Null(column.kind)
    } else {
        parse_bound_value(column.kind, raw_value)?
    };
    let primary_key = take_primary_key(&mut fields, &columns)?;
    if !fields.is_empty() {
        return Err(MutationFailure::Invalid("Unexpected mutation fields"));
    }

    let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new("UPDATE ");
    query.push(quote_table_name(driver, table));
    query.push(" SET ");
    query.push(quote_table_name(driver, &column.name));
    query.push(" = ");
    push_bound_value(&mut query, value);
    push_primary_key_predicate(&mut query, driver, primary_key);
    let result = query
        .build()
        .execute(pool)
        .await
        .map_err(|_| MutationFailure::Database)?;
    match result.rows_affected() {
        1 => Ok(()),
        0 => Err(MutationFailure::NotFound),
        _ => Err(MutationFailure::Conflict),
    }
}

async fn delete_row(table: &str, fields: Vec<(String, String)>) -> Result<(), MutationFailure> {
    let (pool, driver, columns) = mutation_context(table).await?;
    let mut fields = unique_fields(fields)?;
    let confirmation = take_required(&mut fields, "confirm")?;
    if confirmation != format!("DELETE {table}") {
        return Err(MutationFailure::Invalid(
            "Deletion confirmation does not match this table",
        ));
    }
    let primary_key = take_primary_key(&mut fields, &columns)?;
    if !fields.is_empty() {
        return Err(MutationFailure::Invalid("Unexpected mutation fields"));
    }

    let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new("DELETE FROM ");
    query.push(quote_table_name(driver, table));
    push_primary_key_predicate(&mut query, driver, primary_key);
    let result = query
        .build()
        .execute(pool)
        .await
        .map_err(|_| MutationFailure::Database)?;
    match result.rows_affected() {
        1 => Ok(()),
        0 => Err(MutationFailure::NotFound),
        _ => Err(MutationFailure::Conflict),
    }
}

async fn mutation_context(
    table: &str,
) -> Result<
    (
        &'static rullst_orm::RullstPool,
        &'static str,
        Vec<StudioColumn>,
    ),
    MutationFailure,
> {
    if !is_safe_identifier(table) {
        return Err(MutationFailure::NotFound);
    }
    let tables = fetch_tables()
        .await
        .map_err(|_| MutationFailure::Database)?;
    if !tables.iter().any(|candidate| candidate == table) {
        return Err(MutationFailure::NotFound);
    }
    let pool = ensure_pool_initialized()
        .await
        .map_err(|_| MutationFailure::Database)?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let columns = fetch_table_schema(pool, driver, table)
        .await
        .map_err(|_| MutationFailure::Database)?;
    let primary_keys = columns
        .iter()
        .filter(|column| column.primary_key)
        .collect::<Vec<_>>();
    if primary_keys.is_empty() || primary_keys.iter().any(|column| !column.kind.is_editable()) {
        return Err(MutationFailure::Invalid(
            "Mutations require primitive-valued primary keys",
        ));
    }
    Ok((pool, driver, columns))
}

fn unique_fields(
    fields: Vec<(String, String)>,
) -> Result<BTreeMap<String, String>, MutationFailure> {
    if fields.len() > MAX_FORM_FIELDS {
        return Err(MutationFailure::Invalid("Too many mutation fields"));
    }
    let mut unique = BTreeMap::new();
    for (name, value) in fields {
        if name.len() > 80 || unique.insert(name, value).is_some() {
            return Err(MutationFailure::Invalid(
                "Mutation field names must be unique and bounded",
            ));
        }
    }
    Ok(unique)
}

fn take_required(
    fields: &mut BTreeMap<String, String>,
    name: &str,
) -> Result<String, MutationFailure> {
    fields
        .remove(name)
        .filter(|value| !value.is_empty())
        .ok_or(MutationFailure::Invalid(
            "A required mutation field is missing",
        ))
}

fn take_primary_key(
    fields: &mut BTreeMap<String, String>,
    columns: &[StudioColumn],
) -> Result<Vec<(String, BoundValue)>, MutationFailure> {
    let mut values = Vec::new();
    for column in columns.iter().filter(|column| column.primary_key) {
        let form_name = format!("pk_{}", column.name);
        let raw = take_required(fields, &form_name)?;
        values.push((column.name.clone(), parse_bound_value(column.kind, raw)?));
    }
    Ok(values)
}

fn parse_bound_value(kind: StudioColumnKind, raw: String) -> Result<BoundValue, MutationFailure> {
    if raw.len() > MAX_CELL_BYTES || raw.contains('\0') {
        return Err(MutationFailure::Invalid(
            "Cell values must be bounded UTF-8 without NUL bytes",
        ));
    }
    match kind {
        StudioColumnKind::Text => Ok(BoundValue::Text(raw)),
        StudioColumnKind::Integer => raw
            .trim()
            .parse::<i64>()
            .map(BoundValue::Integer)
            .map_err(|_| MutationFailure::Invalid("Expected a signed integer")),
        StudioColumnKind::Float => raw
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(BoundValue::Float)
            .ok_or(MutationFailure::Invalid("Expected a finite number")),
        StudioColumnKind::Boolean => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(BoundValue::Boolean(true)),
            "false" | "0" => Ok(BoundValue::Boolean(false)),
            _ => Err(MutationFailure::Invalid("Expected true, false, 1, or 0")),
        },
        StudioColumnKind::Unsupported => Err(MutationFailure::Invalid(
            "This database type is read-only in Studio",
        )),
    }
}

fn push_bound_value(query: &mut QueryBuilder<rullst_orm::RullstDatabase>, value: BoundValue) {
    match value {
        BoundValue::Text(value) => {
            query.push_bind(value);
        }
        BoundValue::Integer(value) => {
            query.push_bind(value);
        }
        BoundValue::Float(value) => {
            query.push_bind(value);
        }
        BoundValue::Boolean(value) => {
            query.push_bind(value);
        }
        BoundValue::Null(StudioColumnKind::Text) => {
            query.push_bind(Option::<String>::None);
        }
        BoundValue::Null(StudioColumnKind::Integer) => {
            query.push_bind(Option::<i64>::None);
        }
        BoundValue::Null(StudioColumnKind::Float) => {
            query.push_bind(Option::<f64>::None);
        }
        BoundValue::Null(StudioColumnKind::Boolean) => {
            query.push_bind(Option::<bool>::None);
        }
        BoundValue::Null(StudioColumnKind::Unsupported) => {
            query.push_bind(Option::<String>::None);
        }
    }
}

fn push_primary_key_predicate(
    query: &mut QueryBuilder<rullst_orm::RullstDatabase>,
    driver: &str,
    primary_key: Vec<(String, BoundValue)>,
) {
    query.push(" WHERE ");
    for (index, (column, value)) in primary_key.into_iter().enumerate() {
        if index > 0 {
            query.push(" AND ");
        }
        query.push(quote_table_name(driver, &column));
        query.push(" = ");
        push_bound_value(query, value);
    }
}

fn mutation_error_response(error: MutationFailure) -> Response {
    match error {
        MutationFailure::Invalid(message) => {
            (StatusCode::UNPROCESSABLE_ENTITY, message).into_response()
        }
        MutationFailure::NotFound => {
            (StatusCode::NOT_FOUND, "The requested row was not found").into_response()
        }
        MutationFailure::Conflict => (
            StatusCode::CONFLICT,
            "The mutation did not identify exactly one row",
        )
            .into_response(),
        MutationFailure::Database => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "The database rejected the Studio mutation",
        )
            .into_response(),
    }
}

pub(crate) fn build_mutable_rows_html(
    records: &[<rullst_orm::RullstDatabase as sqlx::Database>::Row],
    columns: &[StudioColumn],
    table: &str,
) -> String {
    let column_names = columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    let primary_keys = columns
        .iter()
        .enumerate()
        .filter_map(|(index, column)| column.primary_key.then_some(index))
        .collect::<Vec<_>>();
    let supports_mutations = !primary_keys.is_empty()
        && primary_keys
            .iter()
            .all(|index| columns[*index].kind.is_editable());
    if !supports_mutations {
        return super::super::db::build_rows_html(records, &column_names);
    }
    if records.is_empty() {
        return format!(
            "<tr><td colspan=\"{}\" class=\"px-6 py-16 text-center text-sm text-slate-500 font-medium bg-slate-900/20\">No records found inside this table.</td></tr>",
            columns.len() + 1
        );
    }

    let encoded_table = urlencoding::encode(table);
    let mut html = String::new();
    for row in records {
        html.push_str("<tr class=\"border-b border-slate-800/40 hover:bg-slate-900/30 transition duration-150\">");
        for index in 0..columns.len() {
            let value = get_any_value_as_string(row, index);
            let class = if value == "NULL" {
                "text-slate-600 font-mono italic"
            } else {
                "text-slate-300"
            };
            let _ = write!(
                html,
                "<td class=\"px-6 py-4 text-sm truncate max-w-xs {class}\">{}</td>",
                escape_html_attr(&value)
            );
        }

        let mut primary_inputs = String::new();
        for index in &primary_keys {
            let column = &columns[*index];
            let value = get_any_value_as_string(row, *index);
            let _ = write!(
                primary_inputs,
                "<input type=\"hidden\" name=\"pk_{}\" value=\"{}\">",
                escape_html_attr(&column.name),
                escape_html_attr(&value)
            );
        }
        let mut options = String::from("<option value=\"\">Choose column</option>");
        let mut editable_columns = 0usize;
        for column in columns
            .iter()
            .filter(|column| !column.primary_key && column.kind.is_editable())
        {
            editable_columns += 1;
            let null_hint = if column.nullable { " (nullable)" } else { "" };
            let _ = write!(
                options,
                "<option value=\"{}\">{}{}</option>",
                escape_html_attr(&column.name),
                escape_html_attr(&column.name),
                null_hint
            );
        }
        let update = if editable_columns > 0 {
            format!(
                "<details class=\"mb-2\"><summary class=\"cursor-pointer text-sky-400\">Edit</summary>\
                 <form method=\"post\" action=\"/studio/tables/{encoded_table}/rows/update\" class=\"mt-2 space-y-2\">\
                 {primary_inputs}<select name=\"column\" required class=\"w-full bg-slate-950 border border-slate-700 rounded p-1\">{options}</select>\
                 <input name=\"value\" maxlength=\"16384\" class=\"w-full bg-slate-950 border border-slate-700 rounded p-1\" placeholder=\"replacement value\">\
                 <label class=\"block text-slate-500\"><input type=\"checkbox\" name=\"set_null\" value=\"true\"> set NULL</label>\
                 <button class=\"text-sky-300 border border-sky-800 rounded px-2 py-1\" type=\"submit\">Apply</button></form></details>"
            )
        } else {
            String::new()
        };
        let confirmation = escape_html_attr(&format!("DELETE {table}"));
        let delete = format!(
            "<details><summary class=\"cursor-pointer text-red-400\">Delete</summary>\
             <form method=\"post\" action=\"/studio/tables/{encoded_table}/rows/delete\" class=\"mt-2 space-y-2\">\
             {primary_inputs}<input name=\"confirm\" required maxlength=\"80\" class=\"w-full bg-slate-950 border border-red-900 rounded p-1\" placeholder=\"{confirmation}\">\
             <button class=\"text-red-300 border border-red-900 rounded px-2 py-1\" type=\"submit\">Delete row</button></form></details>"
        );
        let _ = write!(
            html,
            "<td class=\"px-6 py-4 text-xs min-w-56\">{update}{delete}</td></tr>"
        );
    }
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_values_are_typed_and_bounded() {
        assert!(matches!(
            parse_bound_value(StudioColumnKind::Integer, "42".to_string()),
            Ok(BoundValue::Integer(42))
        ));
        assert!(matches!(
            parse_bound_value(StudioColumnKind::Boolean, "false".to_string()),
            Ok(BoundValue::Boolean(false))
        ));
        assert!(parse_bound_value(StudioColumnKind::Float, "NaN".to_string()).is_err());
        assert!(parse_bound_value(StudioColumnKind::Text, "x".repeat(MAX_CELL_BYTES + 1)).is_err());
        assert!(parse_bound_value(StudioColumnKind::Text, "a\0b".to_string()).is_err());
    }

    #[test]
    fn duplicate_or_excessive_form_fields_fail_closed() {
        assert!(
            unique_fields(vec![
                ("column".to_string(), "name".to_string()),
                ("column".to_string(), "email".to_string()),
            ])
            .is_err()
        );
        let excessive = (0..=MAX_FORM_FIELDS)
            .map(|index| (format!("f{index}"), String::new()))
            .collect();
        assert!(unique_fields(excessive).is_err());
    }
}
