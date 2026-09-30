//! HTML rendering components for Nexus CRUD views and forms.

use crate::nexus::crud::batch::supports_deactivation;
use crate::nexus::crud::query::build_table_query;
use crate::nexus::types::{FieldKind, FieldMeta, NexusState, RegistryEntry};
use std::fmt::Write as _;

pub use super::form::render_record_form;

/// Fixed placeholder shown instead of a stored `Password` value.
const PASSWORD_MASK: &str = "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}";

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
            let cells = visible_fields.iter().fold(String::new(), |mut cells, f| {
                let cell = match &f.kind {
                    // Password columns are not selected; never render them.
                    FieldKind::Password => Cell::Value(PASSWORD_MASK.to_string()),
                    FieldKind::Boolean => decode_cell(&row, f.name, |row| {
                        row.try_get::<bool, _>(f.name)
                            .or_else(|_| row.try_get::<i64, _>(f.name).map(|v| v != 0))
                            .ok()
                            .map(|b| if b { "✅ Yes" } else { "❌ No" }.to_string())
                    }),
                    FieldKind::Number | FieldKind::ForeignKey { .. } => decode_cell(&row, f.name, |row| {
                        row.try_get::<i64, _>(f.name)
                            .map(|v| v.to_string())
                            .or_else(|_| row.try_get::<f64, _>(f.name).map(|v| v.to_string()))
                            .or_else(|_| row.try_get::<i32, _>(f.name).map(|v| v.to_string()))
                            .ok()
                    }),
                    _ => Cell::Value(
                        row.try_get::<String, _>(f.name)
                            .unwrap_or_else(|_| "-".to_string()),
                    ),
                };
                let _ = match cell {
                    Cell::Value(value) => write!(
                        cells,
                        "<td class=\"nexus-td\">{}</td>",
                        rullst_core::html::escape_str(&value)
                    ),
                    Cell::Missing(marker) => write!(
                        cells,
                        "<td class=\"nexus-td nexus-muted\">{marker}</td>"
                    ),
                };
                cells
            });

            // A row whose key is NULL or undecodable has no address, so it
            // gets neither a batch checkbox nor edit/delete actions.
            let Some(row_id) = decode_row_key(&row, pk) else {
                let _ = write!(
                    out,
                    "<tr class=\"nexus-tr\"><td class=\"nexus-td\"></td>{cells}\
                     <td class=\"nexus-td nexus-td-actions nexus-muted\">No usable key</td></tr>"
                );
                return out;
            };
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
                 hx-target=\"#nexus-modal-body\">&#9999;&#65039;</button>\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-delete\" data-nexus-delete=\"true\" \
                 data-nexus-table=\"{}\" data-nexus-record=\"{safe_row_id}\">&#128465;&#65039;</button>\
                 </td></tr>"
                , rullst_core::html::escape_str(entry.table)
            ));
            out
        }
    )
}

/// A list cell: a decoded value, or a marker for NULL or undecodable data.
enum Cell {
    Value(String),
    Missing(&'static str),
}

type ListRow = <rullst_orm::RullstDatabase as rullst_orm::_sqlx::Database>::Row;

/// Distinguishes SQL NULL and undecodable values from real ones instead of
/// showing a fabricated `0` or `No`.
fn decode_cell(row: &ListRow, column: &str, decode: impl Fn(&ListRow) -> Option<String>) -> Cell {
    use rullst_orm::_sqlx::{Row, ValueRef};
    match row.try_get_raw(column) {
        Ok(raw) if raw.is_null() => Cell::Missing("NULL"),
        Ok(_) => decode(row).map_or(Cell::Missing("unreadable"), Cell::Value),
        Err(_) => Cell::Missing("unreadable"),
    }
}

/// The record key of a listed row, or `None` when it is NULL or cannot be
/// decoded exactly (a fractional or out-of-range floating-point key would
/// otherwise point the actions at a different record).
fn decode_row_key(row: &ListRow, pk: &str) -> Option<String> {
    use rullst_orm::_sqlx::{Row, ValueRef};
    if row.try_get_raw(pk).map_or(true, |raw| raw.is_null()) {
        return None;
    }
    if let Ok(value) = row.try_get::<i64, _>(pk) {
        return Some(value.to_string());
    }
    if let Ok(value) = row.try_get::<i32, _>(pk) {
        return Some(value.to_string());
    }
    if let Ok(value) = row.try_get::<f64, _>(pk) {
        // Exactly representable integers only; `as` saturates otherwise.
        let integral = value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0;
        return integral.then(|| (value as i64).to_string());
    }
    row.try_get::<String, _>(pk).ok()
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
    let view = TableView {
        page,
        q,
        sort_by,
        order,
        tenant_id,
        csrf_token: None,
    };
    table_view(entry, &view).await
}

/// Request state for one rendering of the table view.
pub(crate) struct TableView<'a> {
    pub(crate) page: u32,
    pub(crate) q: &'a str,
    pub(crate) sort_by: Option<&'a str>,
    pub(crate) order: Option<&'a str>,
    pub(crate) tenant_id: Option<&'a str>,
    /// The request's double-submit token. The bulk-action form is a plain
    /// browser POST that cannot send the `X-CSRF-Token` header, so it carries
    /// the token as the `_token` field Core's CSRF middleware accepts.
    pub(crate) csrf_token: Option<&'a str>,
}

/// Renders the table view for one request.
pub(crate) async fn table_view(entry: &RegistryEntry, view: &TableView<'_>) -> String {
    let TableView {
        page,
        q,
        sort_by,
        order,
        tenant_id,
        csrf_token,
    } = *view;
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();

    let th_cells = visible_fields.iter().fold(String::new(), |mut acc, f| {
        let col = f.name;
        let label = rullst_core::html::escape_str(f.label);
        if matches!(f.kind, FieldKind::Password) {
            // Sorting would reveal the order of stored secrets.
            let _ = write!(acc, "<th class=\"nexus-th\">{label}</th>");
            return acc;
        }
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
             hx-target=\"#nexus-content\" hx-push-url=\"true\" class=\"nexus-th-link\">\
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
    let token_field = csrf_token
        .map(|token| {
            format!(
                "<input type=\"hidden\" name=\"_token\" value=\"{}\" />",
                rullst_core::html::escape_str(token)
            )
        })
        .unwrap_or_default();

    let mut out = String::new();
    let _ = write!(
        out,
        "<div class=\"nexus-page-header\">\
         <div><h1 class=\"nexus-page-title\">{safe_label}</h1>\
         <p class=\"nexus-page-subtitle\">Manage <code>{safe_table}</code> collection records.</p></div>\
         <button type=\"button\" class=\"nexus-btn nexus-btn-primary\" \
         hx-get=\"/nexus/table/{table_path}/new\" hx-target=\"#nexus-modal-body\">\
         &#43; New {safe_label}</button></div>"
    );

    let _ = write!(
        out,
        "<form id=\"batch-form-{table_path}\" method=\"POST\" action=\"/nexus/table/{table_path}/batch\" \
         data-nexus-confirm=\"Apply bulk action?\">{token_field}\
         <div class=\"nexus-toolbar\">\
         <div class=\"nexus-search-wrap\">\
         <span class=\"nexus-search-icon\">&#128269;</span>\
         <input type=\"text\" class=\"nexus-search-input\" name=\"q\" value=\"{}\" placeholder=\"Search {safe_label}...\" \
         hx-get=\"/nexus/table/{table_path}/search\" hx-trigger=\"keyup changed delay:300ms\" \
         hx-target=\"#nexus-table-body\" hx-include=\"[name='q']\" />\
         </div>\
         <select name=\"action\" class=\"nexus-btn nexus-btn-ghost nexus-bulk-select\">\
         <option value=\"\">Bulk Actions</option>\
         <option value=\"delete\">Delete Selected</option>\
         {deactivate_option}\
         </select>\
         <button type=\"submit\" class=\"nexus-btn nexus-btn-ghost\">Apply</button>\
         </div>\
         <div class=\"nexus-table-wrap\">\
         <table class=\"nexus-table\">\
         <thead><tr class=\"nexus-thead-row\">\
         <th class=\"nexus-th nexus-th-check text-center\">\
         <input type=\"checkbox\" data-nexus-select-all=\"true\" aria-label=\"Select all rows\" /></th>\
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
         <div class=\"nexus-pagination-links\">\
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
         <button type=\"button\" class=\"nexus-modal-close\" data-nexus-modal-close=\"true\" aria-label=\"Close\">&times;</button>\
         <div class=\"nexus-modal-inner\" id=\"nexus-modal-body\"></div>\
         </dialog>",
    );

    out
}

#[cfg(test)]
mod tests;
