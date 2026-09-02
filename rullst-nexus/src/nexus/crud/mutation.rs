//! Tenant-scoped, transaction-coupled Nexus record mutations.

use axum::{
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};

use super::handlers::tenant_for_entry;
use super::input::{FormInputError, FormMode, validate_form_values};
use super::query::sanitize_identifier;
use crate::nexus::audit::{MutationAudit, append_mutation, correlation_id};
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

    let mut keys = Vec::new();
    let mut values = Vec::new();
    for value in data {
        if value.field.name == entry.pk && value.value.trim().is_empty() {
            continue;
        }
        keys.push(value.field.name);
        values.push(value.value);
    }
    if let (Some(tenant_column), Some(tenant_id)) = (entry.tenant_column, tenant_id) {
        keys.push(tenant_column);
        values.push(tenant_id.to_string());
    }
    if keys.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html("<p class=\"nexus-error\">No writable values were provided.</p>".to_string()),
        )
            .into_response();
    }

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let placeholders = placeholders(1, keys.len(), driver);
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        sanitize_identifier(entry.table),
        keys.iter()
            .map(|key| sanitize_identifier(key))
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("create", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("create", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for value in &values {
        query = query.bind(value);
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
                record_key: None,
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
    let data = match validate_form_values(entry, data_vec, FormMode::Update) {
        Ok(data) => data,
        Err(error) => return invalid_form_response(entry, error),
    };
    if data.is_empty() {
        return (StatusCode::BAD_REQUEST, "No writable fields were provided.").into_response();
    }

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let updates = data
        .iter()
        .enumerate()
        .map(|(index, value)| {
            format!(
                "{} = {}",
                sanitize_identifier(value.field.name),
                placeholder(index + 1, driver)
            )
        })
        .collect::<Vec<_>>();
    let pk_position = data.len() + 1;
    let tenant_predicate = tenant_id.map(|_| {
        format!(
            " AND {} = {}",
            sanitize_identifier(entry.tenant_column.unwrap_or_default()),
            placeholder(pk_position + 1, driver)
        )
    });
    let sql = format!(
        "UPDATE {} SET {} WHERE {} = {}{}",
        sanitize_identifier(entry.table),
        updates.join(", "),
        sanitize_identifier(entry.pk),
        placeholder(pk_position, driver),
        tenant_predicate.as_deref().unwrap_or_default()
    );
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("update", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("update", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for value in &data {
        query = query.bind(&value.value);
    }
    if let Ok(numeric_id) = id.parse::<i64>() {
        query = query.bind(numeric_id);
    } else {
        query = query.bind(id);
    }
    if let Some(tenant_id) = tenant_id {
        query = query.bind(tenant_id);
    }
    let result = match query.execute(&mut *transaction).await {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) => {
            let _ = transaction.rollback().await;
            return (StatusCode::NOT_FOUND, "Record not found.").into_response();
        }
        Err(_) => {
            let _ = transaction.rollback().await;
            return database_failure("update", entry.table);
        }
    };
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal,
                tenant_id,
                table_name: entry.table,
                action: "update",
                record_key: Some(id),
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
        rullst_core::html::escape_str(id)
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
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let tenant_predicate = tenant_id.map(|_| {
        format!(
            " AND {} = {}",
            sanitize_identifier(entry.tenant_column.unwrap_or_default()),
            placeholder(2, driver)
        )
    });
    let sql = format!(
        "DELETE FROM {} WHERE {} = {}{}",
        sanitize_identifier(entry.table),
        sanitize_identifier(entry.pk),
        placeholder(1, driver),
        tenant_predicate.as_deref().unwrap_or_default()
    );
    let Some(pool) = rullst_core::db::safe_pool() else {
        return database_failure("delete", entry.table);
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return database_failure("delete", entry.table),
    };
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    if let Ok(numeric_id) = id.parse::<i64>() {
        query = query.bind(numeric_id);
    } else {
        query = query.bind(id);
    }
    if let Some(tenant_id) = tenant_id {
        query = query.bind(tenant_id);
    }
    let result = match query.execute(&mut *transaction).await {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) => {
            let _ = transaction.rollback().await;
            return (StatusCode::NOT_FOUND, "Record not found.").into_response();
        }
        Err(_) => {
            let _ = transaction.rollback().await;
            return database_failure("delete", entry.table);
        }
    };
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal,
                tenant_id,
                table_name: entry.table,
                action: "delete",
                record_key: Some(id),
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

fn placeholders(start: usize, count: usize, driver: &str) -> Vec<String> {
    (start..start.saturating_add(count))
        .map(|position| placeholder(position, driver))
        .collect()
}

fn placeholder(position: usize, driver: &str) -> String {
    if driver == "postgres" {
        format!("${position}")
    } else {
        "?".to_string()
    }
}

fn invalid_form_response(entry: &RegistryEntry, error: FormInputError) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Html(format!(
            "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
             &#10060; Invalid {} form: {}</div>",
            rullst_core::html::escape_str(entry.label),
            rullst_core::html::escape_str(&error.to_string())
        )),
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
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_dialect_correct() {
        assert_eq!(placeholders(1, 3, "postgres"), ["$1", "$2", "$3"]);
        assert_eq!(placeholders(2, 2, "sqlite"), ["?", "?"]);
    }
}
