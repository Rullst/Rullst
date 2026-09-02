//! Bounded batch mutations for registered Nexus models.

use axum::{
    extract::{Extension, Path, RawForm, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::nexus::NexusPrincipal;
use crate::nexus::audit::{MutationAudit, append_mutation, correlation_id};
use crate::nexus::crud::handlers::tenant_for_entry;
use crate::nexus::crud::query::{BatchActionForm, find_entry, sanitize_identifier};
use crate::nexus::types::{FieldKind, NexusAuditPolicy, NexusState, RegistryEntry};
use rullst_core::security::TenantContext;

const MAX_BATCH_RECORDS: usize = 1_000;
const MAX_BATCH_FORM_BYTES: usize = 64 * 1024;

fn decode_form_component(value: &str) -> Result<String, &'static str> {
    let spaces_restored = value.replace('+', " ");
    urlencoding::decode(&spaces_restored)
        .map(|decoded| decoded.into_owned())
        .map_err(|_| "Batch form contains invalid UTF-8 encoding")
}

fn parse_batch_form(bytes: &[u8]) -> Result<BatchActionForm, &'static str> {
    if bytes.len() > MAX_BATCH_FORM_BYTES {
        return Err("Batch form exceeds the 64 KiB limit");
    }
    let body = std::str::from_utf8(bytes).map_err(|_| "Batch form must be valid UTF-8")?;
    let mut action = None;
    let mut selected_ids = Vec::new();

    for pair in body.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_form_component(key)?;
        let value = decode_form_component(value)?;
        match key.as_str() {
            "action" if action.is_none() => action = Some(value),
            "action" => return Err("Batch form contains duplicate actions"),
            "selected_ids" => {
                if selected_ids.len() >= MAX_BATCH_RECORDS {
                    return Err("Too many records selected for one batch operation");
                }
                if value.is_empty() || value.len() > 256 {
                    return Err("Batch record IDs must contain between 1 and 256 bytes");
                }
                selected_ids.push(value);
            }
            // The search input shares the surrounding HTML form; a CSRF form token may also be
            // supplied by hosts that do not use the header-based flow.
            "q" | "_token" => {}
            _ => return Err("Batch form contains an unsupported field"),
        }
    }

    Ok(BatchActionForm {
        action: action.ok_or("Batch action is required")?,
        selected_ids,
    })
}

fn deactivation_field(entry: &RegistryEntry) -> Option<&'static str> {
    entry
        .fields
        .iter()
        .find(|field| {
            matches!(field.kind, FieldKind::Boolean)
                && matches!(field.name, "is_active" | "active")
                && !field.readonly
        })
        .map(|field| field.name)
}

/// Reports whether a registered model exposes a conventional writable active flag.
pub(crate) fn supports_deactivation(entry: &RegistryEntry) -> bool {
    deactivation_field(entry).is_some()
}

fn build_batch_sql(
    entry: &RegistryEntry,
    action: &str,
    selected_count: usize,
    driver: &str,
    tenant_scoped: bool,
) -> Option<String> {
    if selected_count == 0 || selected_count > MAX_BATCH_RECORDS {
        return None;
    }

    let placeholders = (1..=selected_count)
        .map(|index| {
            if driver == "postgres" {
                format!("${index}")
            } else {
                "?".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let table = sanitize_identifier(entry.table);
    let primary_key = sanitize_identifier(entry.pk);

    let tenant_predicate = if tenant_scoped {
        let tenant_column = entry.tenant_column?;
        let placeholder = if driver == "postgres" {
            format!("${}", selected_count + 1)
        } else {
            "?".to_string()
        };
        format!(
            " AND {} = {placeholder}",
            sanitize_identifier(tenant_column)
        )
    } else {
        String::new()
    };

    match action {
        "delete" => Some(format!(
            "DELETE FROM {table} WHERE {primary_key} IN ({placeholders}){tenant_predicate}"
        )),
        "deactivate" => deactivation_field(entry).map(|field| {
            let field = sanitize_identifier(field);
            format!(
                "UPDATE {table} SET {field} = FALSE WHERE {primary_key} IN ({placeholders}){tenant_predicate}"
            )
        }),
        _ => None,
    }
}

/// POST /nexus/table/{table}/batch — applies a bounded delete or deactivate operation.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_batch_action(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Extension(principal): Extension<NexusPrincipal>,
    tenant: Option<Extension<TenantContext>>,
    headers: axum::http::HeaderMap,
    RawForm(raw_form): RawForm,
) -> Response {
    let form = match parse_batch_form(&raw_form) {
        Ok(form) => form,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let entry = match find_entry(&state, &table) {
        Some(entry) => entry,
        None => return (StatusCode::NOT_FOUND, "Table not found").into_response(),
    };
    let tenant_id = match tenant_for_entry(entry, tenant.as_ref().map(|value| &value.0)) {
        Ok(tenant_id) => tenant_id,
        Err(error) => return error.into_response(),
    };
    let redirect = format!("/nexus/table/{}", urlencoding::encode(entry.table));

    if form.selected_ids.is_empty() {
        return axum::response::Redirect::to(&redirect).into_response();
    }
    if form.selected_ids.len() > MAX_BATCH_RECORDS {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "Too many records selected for one batch operation",
        )
            .into_response();
    }

    let Some(pool) = rullst_core::db::safe_pool() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database not configured").into_response();
    };
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let Some(sql) = build_batch_sql(
        entry,
        &form.action,
        form.selected_ids.len(),
        driver,
        tenant_id.is_some(),
    ) else {
        return (StatusCode::BAD_REQUEST, "Unsupported batch action").into_response();
    };

    let query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return batch_failure(entry.table),
    };
    let result = if form.selected_ids.iter().all(|id| id.parse::<i64>().is_ok()) {
        let mut query = query;
        for id in &form.selected_ids {
            if let Ok(id) = id.parse::<i64>() {
                query = query.bind(id);
            }
        }
        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id);
        }
        query.execute(&mut *transaction).await
    } else {
        let mut query = query;
        for id in &form.selected_ids {
            query = query.bind(id);
        }
        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id);
        }
        query.execute(&mut *transaction).await
    };

    let result = match result {
        Ok(result) if result.rows_affected() > 0 => result,
        Ok(_) => {
            let _ = transaction.rollback().await;
            return (StatusCode::NOT_FOUND, "No selected records were found.").into_response();
        }
        Err(_) => {
            let _ = transaction.rollback().await;
            return batch_failure(entry.table);
        }
    };
    let action = if form.action == "delete" {
        "batch_delete"
    } else {
        "batch_deactivate"
    };
    if state.audit_policy == NexusAuditPolicy::Required
        && append_mutation(
            &mut transaction,
            &MutationAudit {
                principal: &principal,
                tenant_id,
                table_name: entry.table,
                action,
                record_key: None,
                record_count: result.rows_affected(),
                correlation_id: correlation_id(&headers).as_deref(),
            },
        )
        .await
        .is_err()
    {
        let _ = transaction.rollback().await;
        tracing::error!(table = entry.table, action, "Nexus batch audit failed");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "The operation was not committed because durable audit is unavailable.",
        )
            .into_response();
    }
    if transaction.commit().await.is_err() {
        return batch_failure(entry.table);
    }

    axum::response::Redirect::to(&redirect).into_response()
}

fn batch_failure(table: &str) -> Response {
    tracing::error!(table, "Nexus batch database operation failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "The requested batch operation could not be completed.",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nexus::types::FieldMeta;

    fn entry(fields: Vec<FieldMeta>) -> RegistryEntry {
        RegistryEntry {
            table: "users",
            label: "Users",
            icon: "👥",
            pk: "id",
            tenant_column: None,
            fields,
        }
    }

    #[test]
    fn builds_parameterized_delete_for_each_sql_dialect() {
        let entry = entry(vec![]);
        assert_eq!(
            build_batch_sql(&entry, "delete", 2, "postgres", false).as_deref(),
            Some("DELETE FROM users WHERE id IN ($1,$2)")
        );
        assert_eq!(
            build_batch_sql(&entry, "delete", 2, "sqlite", false).as_deref(),
            Some("DELETE FROM users WHERE id IN (?,?)")
        );
    }

    #[test]
    fn deactivate_requires_a_writable_conventional_boolean_field() {
        let active = entry(vec![FieldMeta::new(
            "is_active",
            "Active",
            FieldKind::Boolean,
        )]);
        assert!(supports_deactivation(&active));
        assert_eq!(
            build_batch_sql(&active, "deactivate", 1, "mysql", false).as_deref(),
            Some("UPDATE users SET is_active = FALSE WHERE id IN (?)")
        );

        let absent = entry(vec![FieldMeta::new("status", "Status", FieldKind::Text)]);
        assert!(!supports_deactivation(&absent));
        assert!(build_batch_sql(&absent, "deactivate", 1, "sqlite", false).is_none());
        assert!(build_batch_sql(&active, "archive", 1, "sqlite", false).is_none());
    }

    #[test]
    fn batch_size_is_bounded() {
        let entry = entry(vec![]);
        assert!(build_batch_sql(&entry, "delete", 0, "sqlite", false).is_none());
        assert!(
            build_batch_sql(&entry, "delete", MAX_BATCH_RECORDS + 1, "sqlite", false).is_none()
        );
    }

    #[test]
    fn tenant_scoped_batch_adds_one_bound_scope_predicate() {
        let mut entry = entry(vec![]);
        entry.tenant_column = Some("tenant_id");
        assert_eq!(
            build_batch_sql(&entry, "delete", 2, "postgres", true).as_deref(),
            Some("DELETE FROM users WHERE id IN ($1,$2) AND tenant_id = $3")
        );
        assert_eq!(
            build_batch_sql(&entry, "delete", 2, "sqlite", true).as_deref(),
            Some("DELETE FROM users WHERE id IN (?,?) AND tenant_id = ?")
        );
    }

    #[test]
    fn parses_repeated_html_checkbox_values_with_strict_bounds() {
        let form = parse_batch_form(
            b"q=active+users&action=deactivate&selected_ids=100&selected_ids=user%2Ftwo",
        )
        .expect("valid browser form");
        assert_eq!(form.action, "deactivate");
        assert_eq!(form.selected_ids, ["100", "user/two"]);

        assert!(parse_batch_form(b"action=delete&action=deactivate").is_err());
        assert!(parse_batch_form(b"action=delete&admin=true").is_err());
        assert!(parse_batch_form(&vec![b'x'; MAX_BATCH_FORM_BYTES + 1]).is_err());
    }
}
