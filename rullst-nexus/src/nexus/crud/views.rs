//! HTML rendering components for Nexus CRUD views and forms.

use std::fmt::Write as _;
use crate::nexus::crud::query::{build_table_query, sanitize_identifier};
use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};

/// Renders a fallback HTML row for empty database tables or empty search results.
pub fn render_empty_state_html(cols: usize, table: &str, q: &str) -> String {
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

/// Renders HTML table `<tr>` rows for the paginated collection view.
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

            let checkbox_cell = format!("<td class=\"nexus-td text-center\"><input type=\"checkbox\" name=\"selected_ids\" value=\"{row_id}\" class=\"nexus-batch-check\" /></td>");

            let _ = std::fmt::Write::write_fmt(&mut out, format_args!(
                "<tr id=\"row-{row_id}\" class=\"nexus-tr\">\
                 {checkbox_cell}\
                 {cells}\
                 <td class=\"nexus-td nexus-td-actions\">\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-edit\" \
                 hx-get=\"/nexus/table/{t}/{row_id}/edit\" \
                 hx-target=\"#nexus-modal-body\" \
                 hx-on:htmx:after-request=\"document.getElementById(&apos;nexus-modal&apos;).showModal()\">&#9999;&#65039;</button>\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-delete\" \
                 onclick=\"nexusDelete(&apos;{t}&apos;, &apos;{row_id}&apos;)\">&#128465;&#65039;</button>\
                 </td></tr>"
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
        let safe_q = rullst_core::html::escape_str(q);
        let safe_col = rullst_core::html::escape_str(col);
        let safe_order = rullst_core::html::escape_str(next_order);
        let _ = write!(
            acc,
            "<th class=\"nexus-th\">\
             <a href=\"/nexus/table/{t}?sort_by={safe_col}&order={safe_order}&q={safe_q}\" \
             hx-get=\"/nexus/table/{t}?sort_by={safe_col}&order={safe_order}&q={safe_q}\" \
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
         hx-on:htmx:after-request=\"document.getElementById('nexus-modal').showModal()\">\
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

    let safe_q = rullst_core::html::escape_str(q);
    let sort_param = sort_by
        .map(|s| format!("&sort_by={}", rullst_core::html::escape_str(s)))
        .unwrap_or_default();
    let order_param = order
        .map(|o| format!("&order={}", rullst_core::html::escape_str(o)))
        .unwrap_or_default();

    let _ = write!(
        out,
        "<div class=\"nexus-pagination\">\
         <div class=\"nexus-page-indicator\">Page {page}</div>\
         <div style=\"display: flex; gap: 8px;\">\
         <a href=\"/nexus/table/{t}?page={prev_page}&q={safe_q}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{t}?page={prev_page}&q={safe_q}{sort_param}{order_param}\" \
         hx-target=\"#nexus-content\" hx-push-url=\"true\">&larr; Prev</a>\
         <a href=\"/nexus/table/{t}?page={next_page}&q={safe_q}{sort_param}{order_param}\" \
         class=\"nexus-btn nexus-btn-ghost\" hx-get=\"/nexus/table/{t}?page={next_page}&q={safe_q}{sort_param}{order_param}\" \
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
            let sql = if driver == "postgres" {
                format!(
                    "SELECT * FROM {} WHERE {} = $1 LIMIT 1",
                    clean_table, clean_pk
                )
            } else {
                format!(
                    "SELECT * FROM {} WHERE {} = ? LIMIT 1",
                    clean_table, clean_pk
                )
            };
            let mut q = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            if let Ok(num_id) = id.parse::<i64>() {
                q = q.bind(num_id);
            } else {
                q = q.bind(id);
            }
            q.fetch_optional(pool).await.unwrap_or(None)
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

    let action_url = if let Some(id) = record_id {
        format!("/nexus/table/{t}/{id}")
    } else {
        format!("/nexus/table/{t}")
    };
    let action_url_id = action_url.trim_start_matches('/').replace('/', "-");
    let form_id = action_url_id.replace('-', "_");

    format!(
        "<h3 class=\"nexus-modal-title\">{title}</h3>\
         <form id=\"nexus-frm-{form_id}\" autocomplete=\"off\" onsubmit=\"return false;\">\
         <div class=\"nexus-fields-grid\">{fields_html}</div>\
         <div class=\"nexus-form-actions\">\
         <button type=\"button\" class=\"nexus-btn nexus-btn-ghost\" \
         onclick=\"document.getElementById(&apos;nexus-modal&apos;).close()\">Cancel</button>\
         <button id=\"nexus-save-{form_id}\" type=\"button\" class=\"nexus-btn nexus-btn-primary\" \
         onclick=\"nexusSave(&apos;nexus-frm-{form_id}&apos;, &apos;{action_url}&apos;, this)\">Save Record</button>\
         </div></form>"
    )
}
