use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::fmt::Write as _;
use std::sync::Arc;

use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};
use crate::nexus::ui::{render_shell, render_sidebar};

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub q: Option<String>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
}

#[derive(Deserialize)]
pub struct BatchActionForm {
    pub action: String,
    #[serde(default)]
    pub selected_ids: Vec<String>,
}

pub fn find_entry<'a>(state: &'a NexusState, table: &str) -> Option<&'a RegistryEntry> {
    state.registry.iter().find(|e| e.table == table)
}

pub fn field_kind_label(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Textarea => "textarea",
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Boolean => "boolean",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime",
        FieldKind::Password => "password",
        FieldKind::Json => "json",
        FieldKind::ForeignKey { .. } => "relation",
        FieldKind::Enum { .. } => "enum",
    }
}

#[cfg_attr(mutants, mutants::skip)]
#[allow(dead_code)]
pub fn field_kind_sql(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number => "INTEGER",
        FieldKind::Boolean => "INTEGER",
        FieldKind::ForeignKey { .. } => "INTEGER",
        FieldKind::Date | FieldKind::DateTime => "TEXT",
        FieldKind::Json => "TEXT",
        FieldKind::Enum { .. } => "TEXT",
        _ => "TEXT",
    }
}

#[cfg(all(test, not(miri)))]
#[cfg_attr(mutants, mutants::skip)]
#[allow(dead_code)]
pub fn field_kind_input_type(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Password => "password",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime-local",
        FieldKind::ForeignKey { .. } => "select",
        FieldKind::Enum { .. } => "select",
        _ => "text",
    }
}

pub fn sanitize_identifier(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

pub fn build_table_query(
    entry: &RegistryEntry,
    visible_fields: &[&FieldMeta],
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> (String, Vec<String>) {
    let clean_table = sanitize_identifier(entry.table);
    let limit = 15;
    let offset = (page - 1) * limit;

    let select_cols: Vec<String> = visible_fields
        .iter()
        .map(|f| sanitize_identifier(f.name))
        .collect();

    let mut select_list = select_cols.join(", ");
    if select_list.is_empty() {
        select_list = "*".to_string();
    }

    let mut sql = format!("SELECT {} FROM {}", select_list, clean_table);
    let mut binds = Vec::new();

    if !q.is_empty() {
        let text_fields: Vec<String> = entry
            .fields
            .iter()
            .filter(|f| matches!(f.kind, FieldKind::Text | FieldKind::Textarea | FieldKind::Email | FieldKind::Url))
            .map(|f| sanitize_identifier(f.name))
            .collect();

        if !text_fields.is_empty() {
            let where_clauses: Vec<String> = text_fields
                .iter()
                .enumerate()
                .map(|(idx, col)| format!("{} LIKE ${}", col, idx + 1))
                .collect();

            sql.push_str(" WHERE ");
            sql.push_str(&where_clauses.join(" OR "));

            let search_term = format!("%{}%", q);
            for _ in 0..text_fields.len() {
                binds.push(search_term.clone());
            }
        }
    }

    let sort_col = sort_by.unwrap_or(entry.pk);
    let sort_dir = order
        .filter(|&o| o.eq_ignore_ascii_case("asc") || o.eq_ignore_ascii_case("desc"))
        .unwrap_or("DESC");
    let clean_sort_col = sanitize_identifier(sort_col);

    let _ = std::fmt::Write::write_fmt(
        &mut sql,
        format_args!(
            " ORDER BY {} {} LIMIT {} OFFSET {}",
            clean_sort_col, sort_dir, limit, offset
        ),
    );

    (sql, binds)
}

fn render_empty_state_html(cols: usize, table: &str, q: &str) -> String {
    if q.is_empty() {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">No records found in table `{}`.</td></tr>",
            cols, table
        )
    } else {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#128269; No results matching \"{}\"</td></tr>",
            cols,
            rullst_core::html::escape_str(q)
        )
    }
}

#[cfg_attr(mutants, mutants::skip)]
pub async fn render_table_rows(
    entry: &RegistryEntry,
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();
    let (sql, binds) = build_table_query(entry, &visible_fields, q, page, sort_by, order);

    let pool = match rullst_core::db::safe_pool() {
        Some(p) => p,
        None => {
            return format!(
                "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#10071; Database not initialized. Please configure database_url.</td></tr>",
                visible_fields.len() + 1
            );
        }
    };

    let sql_safe = rullst_orm::_sqlx::AssertSqlSafe(sql.as_str());
    let mut query = rullst_orm::_sqlx::query(sql_safe);
    for bind in binds {
        query = query.bind(bind);
    }

    use rullst_orm::_sqlx::Row;
    let rows_result = query.fetch_all(pool).await;

    let db_rows = match rows_result {
        Ok(r) => r,
        Err(e) => {
            return format!(
                "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#10071; Database Error: {}</td></tr>",
                visible_fields.len() + 1,
                rullst_core::html::escape_str(&e.to_string())
            );
        }
    };

    if db_rows.is_empty() {
        return render_empty_state_html(visible_fields.len() + 1, entry.table, q);
    }

    let t = entry.table;
    let pk = entry.pk;

    db_rows.into_iter().fold(
        String::with_capacity(2048),
        |mut out, row| {
            let row_id: String = if let Ok(v) = row.try_get::<i64, _>(pk) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<i32, _>(pk) {
                v.to_string()
            } else {
                row.try_get::<String, _>(pk).unwrap_or_else(|_| "0".to_string())
            };

            let cells = visible_fields.iter().fold(String::new(), |mut cells, f| {
                let val_str: String = match &f.kind {
                    FieldKind::Boolean => {
                        let b = row.try_get::<bool, _>(f.name)
                            .or_else(|_| row.try_get::<i64, _>(f.name).map(|v| v != 0))
                            .unwrap_or(false);
                        if b {
                            "&#9989; Yes".to_string()
                        } else {
                            "&#10060; No".to_string()
                        }
                    }
                    FieldKind::Number | FieldKind::ForeignKey { .. } => {
                        if let Ok(v) = row.try_get::<i64, _>(f.name) {
                            v.to_string()
                        } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                            v.to_string()
                        } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                            v.to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    _ => row
                        .try_get::<String, _>(f.name)
                        .unwrap_or_else(|_| "-".to_string()),
                };

                let clean_val = if val_str.starts_with("&#") {
                    val_str
                } else {
                    rullst_core::html::escape_str(&val_str).to_string()
                };

                let _ = std::fmt::Write::write_fmt(&mut cells, format_args!("<td class=\"nexus-td\">{}</td>", clean_val));
                cells
            });

            let checkbox_cell = format!("<td class=\"nexus-td text-center\"><input type=\"checkbox\" name=\"selected_ids\" value=\"{row_id}\" class=\"nexus-batch-check\" /></td>");

            let _ = std::fmt::Write::write_fmt(&mut out, format_args!(
                "<tr id=\"row-{row_id}\" class=\"nexus-tr\">\
                 {checkbox_cell}\
                 {cells}\
                 <td class=\"nexus-td nexus-td-actions\">\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-edit\" \
                 hx-get=\"/nexus/table/{t}/{row_id}/edit\" \
                 hx-target=\"#nexus-modal-body\" \
                 hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">&#9999;&#65039;</button>\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-delete\" \
                 hx-delete=\"/nexus/table/{t}/{row_id}\" \
                 hx-target=\"#row-{row_id}\" \
                 hx-confirm=\"Delete this record?\">&#128465;&#65039;</button>\
                 </td></tr>"
            ));
            out
        }
    )
}

#[cfg_attr(mutants, mutants::skip)]
pub async fn render_table_view(
    _state: &NexusState,
    entry: &RegistryEntry,
    page: u32,
    q: &str,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();

    let th_cells = visible_fields.iter().fold(String::new(), |mut acc, f| {
        let col = f.name;
        let lb = f.label;
        let is_sorted = sort_by == Some(col);
        let next_order = if is_sorted && order == Some("asc") { "desc" } else { "asc" };
        let arrow = if is_sorted {
            if order == Some("asc") { " &#9650;" } else { " &#9660;" }
        } else {
            ""
        };
        let t = entry.table;
        let _ = write!(
            acc,
            "<th class=\"nexus-th\">\
             <a href=\"/nexus/table/{t}?sort_by={col}&order={next_order}&q={q}\" \
             hx-get=\"/nexus/table/{t}?sort_by={col}&order={next_order}&q={q}\" \
             hx-target=\"#nexus-content\" hx-push-url=\"true\" style=\"color: inherit; text-decoration: none;\">\
             {lb}{arrow}</a></th>"
        );
        acc
    });

    let rows_html = render_table_rows(entry, q, page, sort_by, order).await;

    let t = entry.table;
    let lb = entry.label;
    let prev_page = if page > 1 { page - 1 } else { 1 };
    let next_page = page + 1;

    let mut out = String::new();
    let _ = write!(
        out,
        "<div class=\"nexus-page-header\">\
         <div><h1 class=\"nexus-page-title\">{lb}</h1>\
         <p class=\"nexus-page-subtitle\">Manage <code>{t}</code> collection records.</p></div>\
         <button type=\"button\" class=\"nexus-btn nexus-btn-primary\" \
         hx-get=\"/nexus/table/{t}/new\" hx-target=\"#nexus-modal-body\" \
         hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">\
         &#43; New {lb}</button></div>"
    );

    let _ = write!(
        out,
        "<form id=\"batch-form-{t}\" method=\"POST\" action=\"/nexus/table/{t}/batch\">\
         <div class=\"nexus-toolbar\">\
         <div class=\"nexus-search-wrap\">\
         <span class=\"nexus-search-icon\">&#128269;</span>\
         <input type=\"text\" class=\"nexus-search-input\" name=\"q\" value=\"{}\" placeholder=\"Search {lb}...\" \
         hx-get=\"/nexus/table/{t}/search\" hx-trigger=\"keyup changed delay:300ms\" \
         hx-target=\"#nexus-table-body\" hx-include=\"[name='q']\" />\
         </div>\
         <select name=\"action\" class=\"nexus-btn nexus-btn-ghost\" style=\"padding: 8px 12px; font-size: 12px;\">\
         <option value=\"\">Bulk Actions</option>\
         <option value=\"delete\">Delete Selected</option>\
         </select>\
         <button type=\"submit\" class=\"nexus-btn nexus-btn-ghost\" onclick=\"return confirm('Apply bulk action?')\">Apply</button>\
         </div>\
         <div class=\"nexus-table-wrap\">\
         <table class=\"nexus-table\">\
         <thead><tr class=\"nexus-thead-row\">\
         <th class=\"nexus-th text-center\" style=\"width: 40px;\"><input type=\"checkbox\" onclick=\"document.querySelectorAll('.nexus-batch-check').forEach(c => c.checked = this.checked)\" /></th>\
         {th_cells}\
         <th class=\"nexus-th nexus-th-actions\">Actions</th>\
         </tr></thead>\
         <tbody id=\"nexus-table-body\">{rows_html}</tbody>\
         </table></div></form>",
        rullst_core::html::escape_str(q)
    );

    let sort_param = sort_by.map(|s| format!("&sort_by={s}")).unwrap_or_default();
    let order_param = order.map(|o| format!("&order={o}")).unwrap_or_default();

    let _ = write!(
        out,
        "<div class=\"nexus-pagination\">\
         <div class=\"nexus-page-indicator\">Page {page}</div>\
         <div style=\"display: flex; gap: 8px;\">\
         <a href=\"/nexus/table/{t}?page={prev_page}&q={q}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{t}?page={prev_page}&q={q}{sort_param}{order_param}\" \
         hx-target=\"#nexus-content\" hx-push-url=\"true\">&larr; Prev</a>\
         <a href=\"/nexus/table/{t}?page={next_page}&q={q}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{t}?page={next_page}&q={q}{sort_param}{order_param}\" \
         hx-target=\"#nexus-content\" hx-push-url=\"true\">Next &rarr;</a>\
         </div></div>"
    );

    out.push_str(
        "<dialog id=\"nexus-modal\" class=\"nexus-modal\">\
         <button type=\"button\" class=\"nexus-modal-close\" onclick=\"document.getElementById('nexus-modal').close()\">&times;</button>\
         <div class=\"nexus-modal-inner\" id=\"nexus-modal-body\"></div>\
         </dialog>",
    );

    out
}

#[cfg_attr(mutants, mutants::skip)]
pub async fn render_record_form(
    _state: &NexusState,
    entry: &RegistryEntry,
    record_id: Option<&str>,
) -> String {
    let is_edit = record_id.is_some();
    let title = if is_edit {
        format!("Edit {}", entry.label)
    } else {
        format!("New {}", entry.label)
    };

    let t = entry.table;
    let pk = entry.pk;

    use rullst_orm::_sqlx::Row;
    let row_data = if let Some(id) = record_id {
        if let Some(pool) = rullst_core::db::safe_pool() {
            let clean_table = sanitize_identifier(t);
            let clean_pk = sanitize_identifier(pk);
            let sql = format!("SELECT * FROM {} WHERE {} = ? LIMIT 1", clean_table, clean_pk);
            rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
                .bind(id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        }
    } else {
        None
    };

    let fields_html = entry.fields.iter().fold(String::new(), |mut acc, f| {
        let fname = f.name;
        let flabel = f.label;

        let cur_val: String = if let Some(ref r) = row_data {
            match &f.kind {
                FieldKind::Boolean => {
                    let b = r.try_get::<bool, _>(fname)
                        .or_else(|_| r.try_get::<i64, _>(fname).map(|v| v != 0))
                        .unwrap_or(false);
                    if b { "1".to_string() } else { "0".to_string() }
                }
                FieldKind::Number | FieldKind::ForeignKey { .. } => {
                    if let Ok(v) = r.try_get::<i64, _>(fname) {
                        v.to_string()
                    } else if let Ok(v) = r.try_get::<f64, _>(fname) {
                        v.to_string()
                    } else if let Ok(v) = r.try_get::<i32, _>(fname) {
                        v.to_string()
                    } else {
                        "".to_string()
                    }
                }
                _ => r.try_get::<String, _>(fname).unwrap_or_default(),
            }
        } else {
            "".to_string()
        };

        let readonly_attr = if f.readonly || (is_edit && fname == pk) {
            " readonly style=\"opacity: 0.6; cursor: not-allowed;\""
        } else {
            ""
        };

        let input_widget = match &f.kind {
            FieldKind::Textarea | FieldKind::Json => {
                format!(
                    "<textarea name=\"{fname}\" class=\"nexus-input\" rows=\"4\"{readonly_attr}>{}</textarea>",
                    rullst_core::html::escape_str(&cur_val)
                )
            }
            FieldKind::Boolean => {
                let checked = if cur_val == "1" || cur_val == "true" { " checked" } else { "" };
                format!(
                    "<input type=\"hidden\" name=\"{fname}\" value=\"0\" />\
                     <input type=\"checkbox\" name=\"{fname}\" value=\"1\"{checked}{readonly_attr} style=\"width: 20px; height: 20px; accent-color: var(--accent);\" />"
                )
            }
            FieldKind::Enum { options } => {
                let opts = options.iter().fold(String::new(), |mut acc, &opt| {
                    let sel = if opt == cur_val { " selected" } else { "" };
                    let _ = write!(acc, "<option value=\"{opt}\"{sel}>{opt}</option>");
                    acc
                });
                format!("<select name=\"{fname}\" class=\"nexus-input\"{readonly_attr}>{opts}</select>")
            }
            _ => {
                let input_type = match f.kind {
                    FieldKind::Email => "email",
                    FieldKind::Url => "url",
                    FieldKind::Number => "number",
                    FieldKind::Password => "password",
                    FieldKind::Date => "date",
                    FieldKind::DateTime => "datetime-local",
                    _ => "text",
                };
                format!(
                    "<input type=\"{input_type}\" name=\"{fname}\" value=\"{}\" class=\"nexus-input\"{readonly_attr} />",
                    rullst_core::html::escape_str(&cur_val)
                )
            }
        };

        let _ = write!(
            acc,
            "<div class=\"nexus-form-group\">\
             <label class=\"nexus-label\">{flabel}</label>\
             {input_widget}\
             </div>"
        );
        acc
    });

    let (action_url, method_attr) = if let Some(id) = record_id {
        (format!("/nexus/table/{t}/{id}"), "hx-put")
    } else {
        (format!("/nexus/table/{t}"), "hx-post")
    };

    format!(
        "<h3 class=\"nexus-modal-title\">{title}</h3>\
         <form {method_attr}=\"{action_url}\" hx-target=\"#nexus-toast\" \
         hx-on::after-request=\"if(event.detail.successful) {{ document.getElementById('nexus-modal').close(); htmx.trigger('#nexus-table-body', 'keyup'); }}\">\
         <div class=\"nexus-fields-grid\">{fields_html}</div>\
         <div class=\"nexus-form-actions\">\
         <button type=\"button\" class=\"nexus-btn nexus-btn-ghost\" onclick=\"document.getElementById('nexus-modal').close()\">Cancel</button>\
         <button type=\"submit\" class=\"nexus-btn nexus-btn-primary\">Save Record</button>\
         </div></form>"
    )
}

/// GET /nexus — Dashboard overview.
pub async fn nexus_dashboard(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let models_sidebar = render_sidebar(&state, None);

    let stats_cards = state.registry.iter().fold(
        String::with_capacity(state.registry.len() * 256),
        |mut acc, m| {
            let t = m.table;
            let ic = m.icon;
            let lb = m.label;
            let _ = write!(
                acc,
                "<a href=\"/nexus/table/{t}\" class=\"nexus-stat-card\" \
                 hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
                 <div class=\"nexus-stat-icon\">{ic}</div>\
                 <div class=\"nexus-stat-label\">{lb}</div>\
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
    content.push_str("<h2>Auto-Generated CMS</h2>");
    content.push_str("<p>Every model you register appears here with full CRUD, search, and pagination &mdash; zero configuration required.</p>");
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
    axum::extract::Form(data): axum::extract::Form<std::collections::HashMap<String, String>>,
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

    let mut keys = Vec::new();
    let mut values = Vec::new();
    for f in &entry.fields {
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
                entry.label
            ))
        ).into_response();
    }

    let clean_table = sanitize_identifier(&table);
    let clean_keys: Vec<String> = keys.iter().map(|k| sanitize_identifier(k)).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        clean_table,
        clean_keys.join(", "),
        (0..clean_keys.len())
            .map(|i| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", ")
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
                entry.label
            ))
        ).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to create {}: {}\
                 </div>",
                entry.label,
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
    axum::extract::Form(data): axum::extract::Form<std::collections::HashMap<String, String>>,
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

    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let mut updates = Vec::new();
    let mut values = Vec::new();
    for f in &entry.fields {
        if f.name != entry.pk
            && let Some(val) = data.get(f.name)
        {
            let clean_field = sanitize_identifier(f.name);
            updates.push(format!("{} = ${}", clean_field, updates.len() + 1));
            values.push(val);
        }
    }

    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ${}",
        clean_table,
        updates.join(", "),
        clean_pk,
        updates.len() + 1
    );
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for v in values {
        query = query.bind(v);
    }
    query = query.bind(id.clone());

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
                 &#9989; {} #{} updated successfully!\
                 </div>",
                entry.label,
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
                entry.label,
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

    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let sql = format!("DELETE FROM {} WHERE {} = ?", clean_table, clean_pk);
    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
            .bind(&id)
            .execute(pool)
            .await
        {
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
                "<tr id=\"row-{id}\" class=\"nexus-row-deleted\">\
                 <td colspan=\"99\">\
                 <div class=\"nexus-toast nexus-toast-warning\">\
                 &#128465;&#65039; {} #{} deleted.\
                 </div></td></tr>",
                entry.label,
                rullst_core::html::escape_str(&id)
            )),
        )
            .into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to delete {} #{}: {}\
                 </div>",
                entry.label,
                rullst_core::html::escape_str(&id),
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

#[cfg_attr(mutants, mutants::skip)]
pub async fn nexus_batch_action(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    axum::extract::Form(form): axum::extract::Form<BatchActionForm>,
) -> Response {
    if form.selected_ids.is_empty() {
        return axum::response::Redirect::to(&format!("/nexus/table/{}", table)).into_response();
    }

    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found").into_response(),
    };

    let Some(pool) = rullst_core::db::safe_pool() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database not configured").into_response();
    };

    let clean_table = sanitize_identifier(entry.table);
    let clean_pk = sanitize_identifier(entry.pk);
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let mut placeholders = Vec::new();
    for i in 1..=form.selected_ids.len() {
        if driver == "postgres" {
            placeholders.push(format!("${}", i));
        } else {
            placeholders.push("?".to_string());
        }
    }
    let placeholders_str = placeholders.join(",");

    match form.action.as_str() {
        "delete" => {
            let sql = format!(
                "DELETE FROM {} WHERE {} IN ({})",
                clean_table, clean_pk, placeholders_str
            );
            let mut query =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            for id in &form.selected_ids {
                query = query.bind(id);
            }
            let _ = query.execute(pool).await;
        }
        _ => {}
    }

    axum::response::Redirect::to(&format!("/nexus/table/{}", table)).into_response()
}
