//! HTML rendering components for Nexus CRUD views and forms.

use crate::nexus::crud::batch::supports_deactivation;
use crate::nexus::crud::query::{build_table_query, sanitize_identifier};
use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};
use std::fmt::Write as _;

/// Renders a fallback HTML row for empty database tables or empty search results.
pub fn render_empty_state_html(cols: usize, table: &str, q: &str) -> String {
    if q.is_empty() {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">No records found in table `{}`.</td></tr>",
            cols,
            rullst_core::html::escape_str(table)
        )
    } else {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#128269; No results matching \"{}\"</td></tr>",
            cols,
            rullst_core::html::escape_str(q)
        )
    }
}

/// Renders HTML table `<tr>` rows for the paginated collection view.
#[cfg_attr(mutants, mutants::skip)]
pub async fn render_table_rows(
    entry: &RegistryEntry,
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
    tenant_id: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();
    let (sql, binds) =
        build_table_query(entry, &visible_fields, q, page, sort_by, order, tenant_id);

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
        Err(_) => {
            tracing::error!(table = entry.table, "Nexus table query failed");
            return format!(
                "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#10071; The data store is temporarily unavailable.</td></tr>",
                visible_fields.len() + 1
            );
        }
    };

    if db_rows.is_empty() {
        return render_empty_state_html(visible_fields.len() + 1, entry.table, q);
    }

    let table_path = urlencoding::encode(entry.table);
    let pk = entry.pk;

    db_rows.into_iter().fold(
        String::with_capacity(2048),
        |mut out, row| {
            let row_id: String = if let Ok(v) = row.try_get::<i64, _>(pk) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<i32, _>(pk) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<f64, _>(pk) {
                (v as i64).to_string()
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

            let safe_row_id = rullst_core::html::escape_str(&row_id);
            let row_path = urlencoding::encode(&row_id);
            let checkbox_cell = format!("<td class=\"nexus-td text-center\"><input type=\"checkbox\" name=\"selected_ids\" value=\"{safe_row_id}\" class=\"nexus-batch-check\" /></td>");

            let _ = std::fmt::Write::write_fmt(&mut out, format_args!(
                "<tr data-nexus-row-id=\"{safe_row_id}\" class=\"nexus-tr\">\
                 {checkbox_cell}\
                 {cells}\
                 <td class=\"nexus-td nexus-td-actions\">\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-edit\" \
                 hx-get=\"/nexus/table/{table_path}/{row_path}/edit\" \
                 hx-target=\"#nexus-modal-body\" \
                 hx-on:htmx:after-request=\"document.getElementById(&apos;nexus-modal&apos;).showModal()\">&#9999;&#65039;</button>\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-delete\" data-nexus-delete=\"true\" \
                 data-nexus-table=\"{}\" data-nexus-record=\"{safe_row_id}\">&#128465;&#65039;</button>\
                 </td></tr>"
                , rullst_core::html::escape_str(entry.table)
            ));
            out
        }
    )
}

/// Renders the complete HTML table view container including search toolbar and pagination.
#[cfg_attr(mutants, mutants::skip)]
pub async fn render_table_view(
    _state: &NexusState,
    entry: &RegistryEntry,
    page: u32,
    q: &str,
    sort_by: Option<&str>,
    order: Option<&str>,
    tenant_id: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();

    let th_cells = visible_fields.iter().fold(String::new(), |mut acc, f| {
        let col = f.name;
        let label = rullst_core::html::escape_str(f.label);
        let is_sorted = sort_by == Some(col);
        let next_order = if is_sorted && order == Some("asc") { "desc" } else { "asc" };
        let arrow = if is_sorted {
            if order == Some("asc") { " &#9650;" } else { " &#9660;" }
        } else {
            ""
        };
        let table_path = urlencoding::encode(entry.table);
        let query_param = urlencoding::encode(q);
        let column_param = urlencoding::encode(col);
        let order_param = urlencoding::encode(next_order);
        let _ = write!(
            acc,
            "<th class=\"nexus-th\">\
             <a href=\"/nexus/table/{table_path}?sort_by={column_param}&amp;order={order_param}&amp;q={query_param}\" \
             hx-get=\"/nexus/table/{table_path}?sort_by={column_param}&amp;order={order_param}&amp;q={query_param}\" \
             hx-target=\"#nexus-content\" hx-push-url=\"true\" style=\"color: inherit; text-decoration: none;\">\
             {label}{arrow}</a></th>"
        );
        acc
    });

    let rows_html = render_table_rows(entry, q, page, sort_by, order, tenant_id).await;

    let table_path = urlencoding::encode(entry.table);
    let safe_table = rullst_core::html::escape_str(entry.table);
    let safe_label = rullst_core::html::escape_str(entry.label);
    let prev_page = if page > 1 { page - 1 } else { 1 };
    let next_page = page.saturating_add(1);
    let deactivate_option = if supports_deactivation(entry) {
        "<option value=\"deactivate\">Deactivate Selected</option>"
    } else {
        ""
    };

    let mut out = String::new();
    let _ = write!(
        out,
        "<div class=\"nexus-page-header\">\
         <div><h1 class=\"nexus-page-title\">{safe_label}</h1>\
         <p class=\"nexus-page-subtitle\">Manage <code>{safe_table}</code> collection records.</p></div>\
         <button type=\"button\" class=\"nexus-btn nexus-btn-primary\" \
         hx-get=\"/nexus/table/{table_path}/new\" hx-target=\"#nexus-modal-body\" \
         hx-on:htmx:after-request=\"document.getElementById('nexus-modal').showModal()\">\
         &#43; New {safe_label}</button></div>"
    );

    let _ = write!(
        out,
        "<form id=\"batch-form-{table_path}\" method=\"POST\" action=\"/nexus/table/{table_path}/batch\">\
         <div class=\"nexus-toolbar\">\
         <div class=\"nexus-search-wrap\">\
         <span class=\"nexus-search-icon\">&#128269;</span>\
         <input type=\"text\" class=\"nexus-search-input\" name=\"q\" value=\"{}\" placeholder=\"Search {safe_label}...\" \
         hx-get=\"/nexus/table/{table_path}/search\" hx-trigger=\"keyup changed delay:300ms\" \
         hx-target=\"#nexus-table-body\" hx-include=\"[name='q']\" />\
         </div>\
         <select name=\"action\" class=\"nexus-btn nexus-btn-ghost\" style=\"padding: 8px 12px; font-size: 12px;\">\
         <option value=\"\">Bulk Actions</option>\
         <option value=\"delete\">Delete Selected</option>\
         {deactivate_option}\
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

    let query_param = urlencoding::encode(q);
    let sort_param = sort_by
        .map(|sort| format!("&amp;sort_by={}", urlencoding::encode(sort)))
        .unwrap_or_default();
    let order_param = order
        .map(|order| format!("&amp;order={}", urlencoding::encode(order)))
        .unwrap_or_default();

    let _ = write!(
        out,
        "<div class=\"nexus-pagination\">\
         <div class=\"nexus-page-indicator\">Page {page}</div>\
         <div style=\"display: flex; gap: 8px;\">\
         <a href=\"/nexus/table/{table_path}?page={prev_page}&amp;q={query_param}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{table_path}?page={prev_page}&amp;q={query_param}{sort_param}{order_param}\" \
         hx-target=\"#nexus-content\" hx-push-url=\"true\">&larr; Prev</a>\
         <a href=\"/nexus/table/{table_path}?page={next_page}&amp;q={query_param}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{table_path}?page={next_page}&amp;q={query_param}{sort_param}{order_param}\" \
         hx-target=\"#nexus-content\" hx-push-url=\"true\">Next &rarr;</a>\
         </div></div>"
    );

    out.push_str(
        "<dialog id=\"nexus-modal\" class=\"nexus-modal\">\
         <button type=\"button\" class=\"nexus-modal-close\" onclick=\"document.getElementById('nexus-modal').close()\">&times;</button>\
         <div class=\"nexus-modal-inner\" id=\"nexus-modal-body\" hx-on:htmx:after-swap=\"document.getElementById('nexus-modal').showModal()\"></div>\
         </dialog>",
    );

    out
}

/// Renders HTML form for creating or editing records in the modal dialog.
#[cfg_attr(mutants, mutants::skip)]
pub async fn render_record_form(
    _state: &NexusState,
    entry: &RegistryEntry,
    record_id: Option<&str>,
    tenant_id: Option<&str>,
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
            let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
            let clean_table = sanitize_identifier(t);
            let clean_pk = sanitize_identifier(pk);
            let pk_placeholder = if driver == "postgres" { "$1" } else { "?" };
            let tenant_predicate = match (entry.tenant_column, tenant_id) {
                (Some(column), Some(_)) if driver == "postgres" => {
                    format!(" AND {} = $2", sanitize_identifier(column))
                }
                (Some(column), Some(_)) => {
                    format!(" AND {} = ?", sanitize_identifier(column))
                }
                (Some(_), None) => " AND 1 = 0".to_string(),
                (None, _) => String::new(),
            };
            let sql = format!(
                "SELECT * FROM {} WHERE {} = {}{} LIMIT 1",
                clean_table, clean_pk, pk_placeholder, tenant_predicate
            );
            let mut q = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            if let Ok(num_id) = id.parse::<i64>() {
                q = q.bind(num_id);
            } else {
                q = q.bind(id);
            }
            if entry.tenant_column.is_some()
                && let Some(tenant_id) = tenant_id
            {
                q = q.bind(tenant_id);
            }
            match q.fetch_optional(pool).await {
                Ok(row) => row,
                Err(_) => {
                    tracing::error!(table = entry.table, "Nexus record query failed");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    let fields_html = entry
        .fields
        .iter()
        .filter(|field| !field.hidden)
        .fold(String::new(), |mut acc, f| {
        let fname = f.name;
        let flabel = f.label;
        let safe_fname = rullst_core::html::escape_str(fname);
        let safe_flabel = rullst_core::html::escape_str(flabel);

        let cur_val: String = if let Some(ref r) = row_data {
            match &f.kind {
                FieldKind::Boolean => {
                    let b = r.try_get::<bool, _>(fname)
                        .or_else(|_| r.try_get::<i64, _>(fname).map(|v| v != 0))
                        .unwrap_or(false);
                    if b { "1".to_string() } else { "0".to_string() }
                }
                FieldKind::Number | FieldKind::ForeignKey { .. } => {
                    if let Ok(v) = r.try_get::<i32, _>(fname) {
                        v.to_string()
                    } else if let Ok(v) = r.try_get::<i64, _>(fname) {
                        v.to_string()
                    } else if let Ok(v) = r.try_get::<f64, _>(fname) {
                        v.to_string()
                    } else if let Ok(v) = r.try_get::<String, _>(fname) {
                        v
                    } else {
                        "".to_string()
                    }
                }
                _ => r.try_get::<String, _>(fname).unwrap_or_default(),
            }
        } else {
            "".to_string()
        };

        let is_readonly = f.readonly || (is_edit && fname == pk);
        let readonly_attr = if is_readonly {
            " readonly style=\"opacity: 0.6; cursor: not-allowed;\""
        } else {
            ""
        };
        let name_attr = if is_readonly {
            String::new()
        } else {
            format!(" name=\"{safe_fname}\"")
        };

        let input_widget = match &f.kind {
            FieldKind::Textarea | FieldKind::Json => {
                format!(
                    "<textarea{name_attr} class=\"nexus-input\" rows=\"4\"{readonly_attr}>{}</textarea>",
                    rullst_core::html::escape_str(&cur_val)
                )
            }
            FieldKind::Boolean => {
                let checked = if cur_val == "1" || cur_val == "true" { " checked" } else { "" };
                format!(
                    "<input type=\"hidden\"{name_attr} value=\"0\" />\
                     <input type=\"checkbox\"{name_attr} value=\"1\"{checked}{readonly_attr} style=\"width: 20px; height: 20px; accent-color: var(--accent);\" />"
                )
            }
            FieldKind::Enum { options } => {
                let opts = options.iter().fold(String::new(), |mut acc, &opt| {
                    let sel = if opt == cur_val { " selected" } else { "" };
                    let safe_opt = rullst_core::html::escape_str(opt);
                    let _ = write!(acc, "<option value=\"{safe_opt}\"{sel}>{safe_opt}</option>");
                    acc
                });
                format!("<select{name_attr} class=\"nexus-input\"{readonly_attr}>{opts}</select>")
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
                    "<input type=\"{input_type}\"{name_attr} value=\"{}\" class=\"nexus-input\"{readonly_attr} />",
                    rullst_core::html::escape_str(&cur_val)
                )
            }
        };

        let _ = write!(
            acc,
            "<div class=\"nexus-form-group\">\
             <label class=\"nexus-label\">{safe_flabel}</label>\
             {input_widget}\
             </div>"
        );
        acc
    });

    let table_path = urlencoding::encode(t);
    let action_url = if let Some(id) = record_id {
        format!("/nexus/table/{table_path}/{}", urlencoding::encode(id))
    } else {
        format!("/nexus/table/{table_path}")
    };
    let safe_action_url = rullst_core::html::escape_str(&action_url);
    let safe_title = rullst_core::html::escape_str(&title);

    format!(
        "<h3 class=\"nexus-modal-title\">{safe_title}</h3>\
         <form id=\"nexus-record-form\" data-nexus-action=\"{safe_action_url}\" autocomplete=\"off\" onsubmit=\"return false;\">\
         <div class=\"nexus-fields-grid\">{fields_html}</div>\
         <div class=\"nexus-form-actions\">\
         <button type=\"button\" class=\"nexus-btn nexus-btn-ghost\" \
         onclick=\"document.getElementById(&apos;nexus-modal&apos;).close()\">Cancel</button>\
         <button type=\"button\" class=\"nexus-btn nexus-btn-primary\" data-nexus-save=\"true\">Save Record</button>\
         </div></form>"
    )
}

#[cfg(test)]
mod tests;
