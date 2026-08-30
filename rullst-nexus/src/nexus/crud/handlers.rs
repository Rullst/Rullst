//! Axum HTTP request handlers for Nexus CRUD dashboard and model administration.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt::Write as _;
use std::sync::Arc;

use crate::nexus::crud::query::{PaginationParams, find_entry, sanitize_identifier};
use crate::nexus::crud::views::{render_record_form, render_table_rows, render_table_view};
use crate::nexus::types::NexusState;
use crate::nexus::ui::{render_shell, render_sidebar, safe_icon_html};

/// GET /nexus — Dashboard overview.
pub async fn nexus_dashboard(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let models_sidebar = render_sidebar(&state, None);

    let stats_cards = state.registry.iter().fold(
        String::with_capacity(state.registry.len().saturating_mul(256).min(64 * 1024)),
        |mut acc, m| {
            let table_path = urlencoding::encode(m.table);
            let icon = safe_icon_html(m.icon);
            let label = rullst_core::html::escape_str(m.label);
            let _ = write!(
                acc,
                "<a href=\"/nexus/table/{table_path}\" class=\"nexus-stat-card\" \
                 hx-get=\"/nexus/table/{table_path}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
                 <div class=\"nexus-stat-icon\">{icon}</div>\
                 <div class=\"nexus-stat-label\">{label}</div>\
                 <div class=\"nexus-stat-hint\">Click to manage &rarr;</div>\
                 </a>"
            );
            acc
        },
    );

    let mut content = String::new();
    content.push_str("<div class=\"nexus-page-header\">");
    content.push_str("<h1 class=\"nexus-page-title\">&#127963;&#65039; Dashboard</h1>");
    content.push_str("<p class=\"nexus-page-subtitle\">Welcome to the Rullst Nexus Panel. Select a model to begin.</p>");
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-stat-grid\">");
    content.push_str(&stats_cards);
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-welcome-box\">");
    content.push_str("<div class=\"nexus-welcome-icon\">&#9889;</div>");
    content.push_str("<h2>Registered-Model Admin</h2>");
    content.push_str("<p>Models explicitly registered by the application appear here with the CRUD, search, and pagination capabilities allowed by their metadata and panel policy.</p>");
    content.push_str("<a href=\"/nexus/chat\" class=\"nexus-btn nexus-btn-ai\" hx-get=\"/nexus/chat\" hx-target=\"#nexus-content\" hx-push-url=\"true\">&#129302; Open AI Query Assistant</a>");
    content.push_str("</div>");

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(&state, &models_sidebar, &content))
    }
}

/// GET /nexus/table/{table} — Model list view.
pub async fn nexus_table_view(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Query(params): Query<PaginationParams>,
    headers: axum::http::HeaderMap,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<p>Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let q = params.q.clone().unwrap_or_default();
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();

    let content = render_table_view(&state, entry, page, &q, sort_by, order).await;
    if headers.contains_key("hx-request") {
        Html(content).into_response()
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, Some(&table)),
            &content,
        ))
        .into_response()
    }
}

/// GET /nexus/table/{table}/search — HTMX search fragment.
pub async fn nexus_table_search(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    let q = params.q.clone().unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();
    Html(render_table_rows(entry, &q, page, sort_by, order).await)
}

/// GET /nexus/table/{table}/new — New record form.
pub async fn nexus_new_form(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(&state, entry, None).await)
}

/// POST /nexus/table/{table} — Create a new record.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_create_record(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    axum::extract::Form(data_vec): axum::extract::Form<Vec<(String, String)>>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let mut data = std::collections::HashMap::new();
    for (k, v) in data_vec {
        if !v.is_empty() || !data.contains_key(&k) {
            data.insert(k, v);
        }
    }

    let mut keys = Vec::new();
    let mut values = Vec::new();
    for f in entry
        .fields
        .iter()
        .filter(|field| !field.hidden && !field.readonly)
    {
        if let Some(val) = data.get(f.name) {
            if f.name == entry.pk && val.trim().is_empty() {
                continue;
            }
            keys.push(f.name);
            values.push(val);
        }
    }

    if keys.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; No values provided to create {}\
                 </div>",
                rullst_core::html::escape_str(entry.label)
            ))
        ).into_response();
    }

    let clean_table = sanitize_identifier(&table);
    let clean_keys: Vec<String> = keys.iter().map(|k| sanitize_identifier(k)).collect();
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let placeholders = (0..clean_keys.len())
        .map(|i| {
            if driver == "postgres" {
                format!("${}", i + 1)
            } else {
                "?".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        clean_table,
        clean_keys.join(", "),
        placeholders
    );

    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for v in values {
        query = query.bind(v);
    }

    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match query.execute(pool).await {
            Ok(_) => {
                success = true;
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (
            StatusCode::OK,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#9989; New {} record created successfully!\
                 </div>",
                rullst_core::html::escape_str(entry.label)
            ))
        ).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to create {}: {}\
                 </div>",
                rullst_core::html::escape_str(entry.label),
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

/// GET /nexus/table/{table}/{id}/edit — Edit record form.
pub async fn nexus_edit_form(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(&state, entry, Some(&id)).await)
}

/// PUT /nexus/table/{table}/{id} — Update a record.
pub async fn nexus_update_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
    axum::extract::Form(data_vec): axum::extract::Form<Vec<(String, String)>>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let mut data = std::collections::HashMap::new();
    for (k, v) in data_vec {
        if !v.is_empty() || !data.contains_key(&k) {
            data.insert(k, v);
        }
    }

    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let mut updates = Vec::new();
    let mut values = Vec::new();
    for f in &entry.fields {
        if !f.hidden
            && !f.readonly
            && f.name != entry.pk
            && let Some(val) = data.get(f.name)
        {
            let clean_field = sanitize_identifier(f.name);
            if driver == "postgres" {
                updates.push(format!("{} = ${}", clean_field, updates.len() + 1));
            } else {
                updates.push(format!("{} = ?", clean_field));
            }
            values.push(val);
        }
    }

    if updates.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html("<p class=\"nexus-error\">No writable fields were provided.</p>".to_string()),
        )
            .into_response();
    }

    let pk_placeholder = if driver == "postgres" {
        format!("${}", updates.len() + 1)
    } else {
        "?".to_string()
    };

    let sql = format!(
        "UPDATE {} SET {} WHERE {} = {}",
        clean_table,
        updates.join(", "),
        clean_pk,
        pk_placeholder
    );
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for v in values {
        query = query.bind(v);
    }
    if let Ok(num_id) = id.parse::<i64>() {
        query = query.bind(num_id);
    } else {
        query = query.bind(id.clone());
    }

    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match query.execute(pool).await {
            Ok(result) => {
                success = result.rows_affected() > 0;
                if !success {
                    err_msg = "Record not found".to_string();
                }
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (
            StatusCode::OK,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#9989; {} #{} updated successfully!\
                 </div>",
                rullst_core::html::escape_str(entry.label),
                rullst_core::html::escape_str(&id)
            ))
        ).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to update {}: {}\
                 </div>",
                rullst_core::html::escape_str(entry.label),
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

/// DELETE /nexus/table/{table}/{id} — Delete a record.
pub async fn nexus_delete_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let sql = if driver == "postgres" {
        format!("DELETE FROM {} WHERE {} = $1", clean_table, clean_pk)
    } else {
        format!("DELETE FROM {} WHERE {} = ?", clean_table, clean_pk)
    };
    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        let mut q = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
        if let Ok(num_id) = id.parse::<i64>() {
            q = q.bind(num_id);
        } else {
            q = q.bind(&id);
        }
        match q.execute(pool).await {
            Ok(result) => {
                success = result.rows_affected() > 0;
                if !success {
                    err_msg = "Record not found".to_string();
                }
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (StatusCode::OK, "Record deleted successfully.").into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Failed to delete {} #{}: {}",
                rullst_core::html::escape_str(entry.label),
                rullst_core::html::escape_str(&id),
                rullst_core::html::escape_str(&err_msg)
            ),
        )
            .into_response()
    }
}
