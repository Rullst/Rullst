//! Fail-closed, primitive-value row mutations for the local Studio browser.

use super::super::db::{
    StudioColumn, StudioColumnKind, ensure_pool_initialized, fetch_table_schema, fetch_tables,
    is_safe_identifier, qualified_table_name, quote_table_name,
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

use super::super::limits::MAX_CELL_BYTES;
use super::super::portable::build_for_driver;

mod forms;
pub(crate) use forms::build_mutable_rows_html;

const MAX_FORM_FIELDS: usize = 260;

/// Request-body limit of the row mutation routes.
pub(crate) const MUTATION_BODY_LIMIT: usize = 64 * 1024;

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
    query.push(qualified_table_name(driver, table));
    query.push(" SET ");
    query.push(quote_table_name(driver, &column.name));
    query.push(" = ");
    push_bound_value(&mut query, value);
    push_primary_key_predicate(&mut query, driver, primary_key);
    execute_single_row_mutation(pool, driver, &mut query).await
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
    query.push(qualified_table_name(driver, table));
    push_primary_key_predicate(&mut query, driver, primary_key);
    execute_single_row_mutation(pool, driver, &mut query).await
}

/// Runs one row mutation in a transaction and commits it only when exactly one
/// row changed. Any other count is rolled back before the failure is reported,
/// so a predicate that unexpectedly matches several rows changes none of them.
async fn execute_single_row_mutation(
    pool: &rullst_orm::RullstPool,
    driver: &str,
    query: &mut QueryBuilder<rullst_orm::RullstDatabase>,
) -> Result<(), MutationFailure> {
    let mut transaction = pool.begin().await.map_err(|_| MutationFailure::Database)?;
    let executed = match build_for_driver(query, driver) {
        Ok(statement) => statement.execute(&mut *transaction).await,
        Err(error) => Err(error),
    };
    let affected = match executed {
        Ok(result) => result.rows_affected(),
        Err(_) => {
            let _ = transaction.rollback().await;
            return Err(MutationFailure::Database);
        }
    };
    let failure = match affected {
        1 => {
            return transaction
                .commit()
                .await
                .map_err(|_| MutationFailure::Database);
        }
        0 => MutationFailure::NotFound,
        _ => MutationFailure::Conflict,
    };
    transaction
        .rollback()
        .await
        .map_err(|_| MutationFailure::Database)?;
    Err(failure)
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
    let schema = fetch_table_schema(pool, driver, table)
        .await
        .map_err(|_| MutationFailure::Database)?;
    if !schema.supports_mutations() {
        return Err(MutationFailure::Invalid(
            "Mutations require a complete text, integer or Boolean primary key",
        ));
    }
    Ok((pool, driver, schema.columns))
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
        // A text key may legitimately be empty, so only presence is required.
        let raw = fields
            .remove(&format!("pk_{}", column.name))
            .ok_or(MutationFailure::Invalid(
                "A required mutation field is missing",
            ))?;
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

#[cfg(test)]
mod tests;
