//! Tenant-scoped, transaction-coupled Nexus record mutations.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};

use rullst_orm::RullstValue;
use rullst_orm::value::BindRullstValue;

use super::dialect::{ValueSql, driver_value, placeholder, tenant_predicate, write_value_sql};
use super::handlers::tenant_for_entry;
use super::input::{FormInputError, FormMode, ValidatedFieldValue, validate_form_values};
use super::key::{RecordKey, key_predicate, record_address};
use super::query::sanitize_identifier;
use crate::nexus::audit::{MutationAudit, append_mutation, auditable_record_key, correlation_id};
use crate::nexus::{NexusAuditPolicy, NexusPrincipal, NexusState, RegistryEntry};
use rullst_core::security::TenantContext;

pub(super) async fn create_record(
    state: &NexusState,
    entry: &RegistryEntry,
    principal: &NexusPrincipal,
    tenant: Option<&TenantContext>,
    headers: &HeaderMap,
    data_vec: Vec<(String, String)>,
) -> Response {
    let tenant_id = match tenant_for_entry(entry, tenant) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let data = match validate_form_values(entry, data_vec, FormMode::Create) {
        Ok(data) => data,
        Err(error) => return invalid_form_response(entry, error),
    };

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let submitted_key = submitted_address(entry, &data);
    let Some((sql, values)) = insert_sql(entry, data, tenant_id, driver) else {
        return (StatusCode::BAD_REQUEST, "No writable values were provided.").into_response();
    };
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("create", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("create", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for value in &values {
        query = query.bind_rullst_value(value);
    }
    let result = match query.execute(&mut *transaction).await {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) | Err(_) => {
            let _ = transaction.rollback().await;
            return database_failure("create", entry.table);
        }
    };
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal,
                tenant_id,
                table_name: entry.table,
                action: "create",
                record_key: submitted_key.as_deref().and_then(auditable_record_key),
                record_count: result.rows_affected(),
                correlation_id: correlation_id(headers).as_deref(),
            },
        )
        .await
        .is_err()
    {
        let _ = transaction.rollback().await;
        return audit_failure("create", entry.table);
    }
    if transaction.commit().await.is_err() {
        return database_failure("create", entry.table);
    }

    Html(format!(
        "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
         &#9989; New {} record created successfully!</div>",
        rullst_core::html::escape_str(entry.label)
    ))
    .into_response()
}

pub(super) async fn update_record(
    state: &NexusState,
    entry: &RegistryEntry,
    id: &str,
    principal: &NexusPrincipal,
    tenant: Option<&TenantContext>,
    headers: &HeaderMap,
    data_vec: Vec<(String, String)>,
) -> Response {
    let tenant_id = match tenant_for_entry(entry, tenant) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let Some(key) = RecordKey::parse(entry, id) else {
        return record_not_found();
    };
    let data = match validate_form_values(entry, data_vec, FormMode::Update) {
        Ok(data) => data,
        Err(error) => return invalid_form_response(entry, error),
    };
    if data.is_empty() {
        return (StatusCode::BAD_REQUEST, "No writable fields were provided.").into_response();
    }

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let mut updates = Vec::with_capacity(data.len());
    let mut bound_values = Vec::with_capacity(data.len());
    for value in &data {
        let column = sanitize_identifier(value.field.name);
        match write_value_sql(
            &value.field.kind,
            value.value.as_deref(),
            bound_values.len() + 1,
            driver,
        ) {
            ValueSql::Bind(expression) => {
                updates.push(format!("{column} = {expression}"));
                bound_values.push(bound_value(value, driver));
            }
            ValueSql::Literal(literal) => updates.push(format!("{column} = {literal}")),
        }
    }
    let sql = update_sql(
        entry,
        &updates,
        bound_values.len(),
        tenant_id.is_some(),
        driver,
    );
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("update", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("update", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    // Only submitted fields are written; `Null` stores SQL NULL.
    for value in &bound_values {
        query = query.bind_rullst_value(value);
    }
    query = key.bind(query);
    if let Some(tenant_id) = tenant_id {
        query = query.bind(tenant_id);
    }
    let result = match query.execute(&mut *transaction).await {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) => {
            let _ = transaction.rollback().await;
            return record_not_found();
        }
        Err(_) => {
            let _ = transaction.rollback().await;
            return database_failure("update", entry.table);
        }
    };
    let record_key = key.text();
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal,
                tenant_id,
                table_name: entry.table,
                action: "update",
                record_key: auditable_record_key(record_key),
                record_count: result.rows_affected(),
                correlation_id: correlation_id(headers).as_deref(),
            },
        )
        .await
        .is_err()
    {
        let _ = transaction.rollback().await;
        return audit_failure("update", entry.table);
    }
    if transaction.commit().await.is_err() {
        return database_failure("update", entry.table);
    }

    Html(format!(
        "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
         &#9989; {} #{} updated successfully!</div>",
        rullst_core::html::escape_str(entry.label),
        rullst_core::html::escape_str(record_key)
    ))
    .into_response()
}

pub(super) async fn delete_record(
    state: &NexusState,
    entry: &RegistryEntry,
    id: &str,
    principal: &NexusPrincipal,
    tenant: Option<&TenantContext>,
    headers: &HeaderMap,
) -> Response {
    let tenant_id = match tenant_for_entry(entry, tenant) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let Some(key) = RecordKey::parse(entry, id) else {
        return record_not_found();
    };
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let sql = delete_sql(entry, tenant_id.is_some(), driver);
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("delete", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("delete", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    query = key.bind(query);
    if let Some(tenant_id) = tenant_id {
        query = query.bind(tenant_id);
    }
    let result = match query.execute(&mut *transaction).await {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) => {
            let _ = transaction.rollback().await;
            return record_not_found();
        }
        Err(_) => {
            let _ = transaction.rollback().await;
            return database_failure("delete", entry.table);
        }
    };
    let record_key = key.text();
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal,
                tenant_id,
                table_name: entry.table,
                action: "delete",
                record_key: auditable_record_key(record_key),
                record_count: result.rows_affected(),
                correlation_id: correlation_id(headers).as_deref(),
            },
        )
        .await
        .is_err()
    {
        let _ = transaction.rollback().await;
        return audit_failure("delete", entry.table);
    }
    if transaction.commit().await.is_err() {
        return database_failure("delete", entry.table);
    }
    (StatusCode::OK, "Record deleted successfully.").into_response()
}

/// The value bound for a validated field: its typed date or time, else its
/// text, else SQL NULL.
pub(super) fn bound_value(value: &ValidatedFieldValue<'_>, driver: &str) -> RullstValue {
    match (&value.typed, &value.value) {
        (Some(typed), _) => driver_value(typed.clone(), driver),
        (None, Some(text)) => RullstValue::String(text.clone()),
        (None, None) => RullstValue::Null,
    }
}

/// The `INSERT` of a create form and the values bound to it, in placeholder
/// order, or `None` when no column would be written. An empty key column is
/// left to the database (an identity column); a tenant-scoped request also
/// writes the trusted tenant.
pub(super) fn insert_sql(
    entry: &RegistryEntry,
    data: Vec<ValidatedFieldValue<'_>>,
    tenant_id: Option<&str>,
    driver: &str,
) -> Option<(String, Vec<RullstValue>)> {
    let mut columns = Vec::new();
    let mut expressions = Vec::new();
    let mut values: Vec<RullstValue> = Vec::new();
    for value in data {
        if entry.is_key_column(value.field.name) && blank(value.value.as_deref()) {
            continue;
        }
        columns.push(value.field.name);
        match write_value_sql(
            &value.field.kind,
            value.value.as_deref(),
            values.len() + 1,
            driver,
        ) {
            ValueSql::Bind(expression) => {
                expressions.push(expression);
                values.push(bound_value(&value, driver));
            }
            ValueSql::Literal(literal) => expressions.push(literal.to_string()),
        }
    }
    if let (Some(tenant_column), Some(tenant_id)) = (entry.tenant_column, tenant_id) {
        columns.push(tenant_column);
        expressions.push(placeholder(values.len() + 1, driver));
        values.push(RullstValue::String(tenant_id.to_string()));
    }
    if columns.is_empty() {
        return None;
    }
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        sanitize_identifier(entry.table),
        columns
            .iter()
            .map(|column| sanitize_identifier(column))
            .collect::<Vec<_>>()
            .join(", "),
        expressions.join(", ")
    );
    Some((sql, values))
}

/// Whether a submitted key value is absent or only whitespace.
fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|value| value.trim().is_empty())
}

/// The address of the record a create form names when the administrator
/// entered every key column, so the audit can record it before the INSERT.
/// A value that is not canonical for its column (`01` for an integer key,
/// which the database may store as `1`) names no address.
pub(super) fn submitted_address(
    entry: &RegistryEntry,
    data: &[ValidatedFieldValue<'_>],
) -> Option<String> {
    let values = entry
        .key_columns()
        .iter()
        .map(|column| {
            data.iter()
                .find(|value| value.field.name == *column)
                .and_then(|value| value.value.clone())
                .filter(|value| !blank(Some(value)))
        })
        .collect::<Option<Vec<_>>>()?;
    let address = record_address(&values);
    RecordKey::parse(entry, &address)
        .is_some()
        .then_some(address)
}

/// `UPDATE … SET <assignments> WHERE <key>[ AND <tenant>]`. The key values
/// are bound after the `bound` values of the assignments, then the tenant.
pub(super) fn update_sql(
    entry: &RegistryEntry,
    assignments: &[String],
    bound: usize,
    tenant_scoped: bool,
    driver: &str,
) -> String {
    format!(
        "UPDATE {} SET {} WHERE {}{}",
        sanitize_identifier(entry.table),
        assignments.join(", "),
        key_predicate(entry, bound + 1, driver),
        tenant_scope(
            entry,
            bound + entry.key_columns().len() + 1,
            tenant_scoped,
            driver
        )
    )
}

/// `DELETE FROM … WHERE <key>[ AND <tenant>]`: the key values, then the tenant.
pub(super) fn delete_sql(entry: &RegistryEntry, tenant_scoped: bool, driver: &str) -> String {
    format!(
        "DELETE FROM {} WHERE {}{}",
        sanitize_identifier(entry.table),
        key_predicate(entry, 1, driver),
        tenant_scope(entry, entry.key_columns().len() + 1, tenant_scoped, driver)
    )
}

/// ` AND <tenant> = <placeholder>` for a scoped request, else nothing.
fn tenant_scope(
    entry: &RegistryEntry,
    position: usize,
    tenant_scoped: bool,
    driver: &str,
) -> String {
    match (entry.tenant_column, tenant_scoped) {
        (Some(column), true) => format!(
            " AND {}",
            tenant_predicate(column, &placeholder(position, driver), driver)
        ),
        _ => String::new(),
    }
}

fn record_not_found() -> Response {
    (StatusCode::NOT_FOUND, "Record not found.").into_response()
}

/// A plain-text `422`: `nexus.js` shows error bodies as text, so markup
/// would reach the administrator literally.
/// The `422` answer to a rejected form.
pub(super) fn invalid_form_response(entry: &RegistryEntry, error: FormInputError) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        format!("Invalid {} form: {error}", entry.label),
    )
        .into_response()
}

fn database_failure(operation: &'static str, table: &str) -> Response {
    tracing::error!(operation, table, "Nexus database mutation failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "The requested database operation could not be completed.",
    )
        .into_response()
}

fn audit_failure(operation: &'static str, table: &str) -> Response {
    tracing::error!(
        operation,
        table,
        "Nexus audit append failed; mutation rolled back"
    );
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "The operation was not committed because durable audit is unavailable.",
    )
        .into_response()
}

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod tests;
