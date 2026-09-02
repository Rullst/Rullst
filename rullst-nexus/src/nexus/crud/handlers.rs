//! Axum HTTP request handlers for Nexus CRUD dashboard and model administration.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt::Write as _;
use std::sync::Arc;

use crate::nexus::NexusPrincipal;
use crate::nexus::crud::mutation::{create_record, delete_record, update_record};
use crate::nexus::crud::query::{PaginationParams, find_entry};
use crate::nexus::crud::views::{render_record_form, render_table_rows, render_table_view};
use crate::nexus::types::{NexusState, RegistryEntry};
use crate::nexus::ui::{render_shell, render_sidebar, safe_icon_html};
use rullst_core::security::TenantContext;

#[derive(Debug, Clone, Copy)]
pub(crate) struct MissingTenantContext;

impl IntoResponse for MissingTenantContext {
    fn into_response(self) -> Response {
        (
            StatusCode::FORBIDDEN,
            "This model requires authenticated tenant context.",
        )
            .into_response()
    }
}

pub(crate) fn tenant_for_entry<'a>(
    entry: &RegistryEntry,
    context: Option<&'a TenantContext>,
) -> Result<Option<&'a str>, MissingTenantContext> {
    match (entry.tenant_column, context) {
        (Some(_), Some(context)) => Ok(Some(context.tenant_id.as_str())),
        (Some(_), None) => Err(MissingTenantContext),
        (None, _) => Ok(None),
    }
}

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
    tenant: Option<Extension<TenantContext>>,
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
    let tenant_id = match tenant_for_entry(entry, tenant.as_ref().map(|value| &value.0)) {
        Ok(tenant_id) => tenant_id,
        Err(error) => return error.into_response(),
    };

    let page = params.page.unwrap_or(1).max(1);
    let q = params.q.clone().unwrap_or_default();
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();

    let content = render_table_view(&state, entry, page, &q, sort_by, order, tenant_id).await;
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
    tenant: Option<Extension<TenantContext>>,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (StatusCode::NOT_FOUND, "Table not found.").into_response();
        }
    };
    let tenant_id = match tenant_for_entry(entry, tenant.as_ref().map(|value| &value.0)) {
        Ok(tenant_id) => tenant_id,
        Err(error) => return error.into_response(),
    };
    let q = params.q.clone().unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();
    Html(render_table_rows(entry, &q, page, sort_by, order, tenant_id).await).into_response()
}

/// GET /nexus/table/{table}/new — New record form.
pub async fn nexus_new_form(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    tenant: Option<Extension<TenantContext>>,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found.").into_response(),
    };
    let tenant_id = match tenant_for_entry(entry, tenant.as_ref().map(|value| &value.0)) {
        Ok(tenant_id) => tenant_id,
        Err(error) => return error.into_response(),
    };
    Html(render_record_form(&state, entry, None, tenant_id).await).into_response()
}

/// POST /nexus/table/{table} — Create a new record.
#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_create_record(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Extension(principal): Extension<NexusPrincipal>,
    tenant: Option<Extension<TenantContext>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(data_vec): axum::extract::Form<Vec<(String, String)>>,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found.").into_response(),
    };
    create_record(
        &state,
        entry,
        &principal,
        tenant.as_ref().map(|value| &value.0),
        &headers,
        data_vec,
    )
    .await
}

/// GET /nexus/table/{table}/{id}/edit — Edit record form.
pub async fn nexus_edit_form(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
    tenant: Option<Extension<TenantContext>>,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found.").into_response(),
    };
    let tenant_id = match tenant_for_entry(entry, tenant.as_ref().map(|value| &value.0)) {
        Ok(tenant_id) => tenant_id,
        Err(error) => return error.into_response(),
    };
    Html(render_record_form(&state, entry, Some(&id), tenant_id).await).into_response()
}

/// PUT /nexus/table/{table}/{id} — Update a record.
pub async fn nexus_update_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
    Extension(principal): Extension<NexusPrincipal>,
    tenant: Option<Extension<TenantContext>>,
    headers: axum::http::HeaderMap,
    axum::extract::Form(data_vec): axum::extract::Form<Vec<(String, String)>>,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found.").into_response(),
    };
    update_record(
        &state,
        entry,
        &id,
        &principal,
        tenant.as_ref().map(|value| &value.0),
        &headers,
        data_vec,
    )
    .await
}

/// DELETE /nexus/table/{table}/{id} — Delete a record.
pub async fn nexus_delete_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
    Extension(principal): Extension<NexusPrincipal>,
    tenant: Option<Extension<TenantContext>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found.").into_response(),
    };
    delete_record(
        &state,
        entry,
        &id,
        &principal,
        tenant.as_ref().map(|value| &value.0),
        &headers,
    )
    .await
}
