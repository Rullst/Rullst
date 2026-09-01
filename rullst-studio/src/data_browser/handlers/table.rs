//! Table browser handler — schema inspection, paginated data view, search.

use super::super::db::*;
use super::super::layout::*;
use super::mutations::build_mutable_rows_html;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use sqlx::QueryBuilder;
use std::fmt::Write;

pub async fn handle_table(
    Path(table): Path<String>,
    Query(query): Query<TableQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let pool = match ensure_pool_initialized().await {
        Ok(p) => p,
        Err(error) => {
            return table_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("Database unavailable: {error}"),
                is_htmx,
                None,
                &[],
            );
        }
    };

    let tables = match fetch_tables().await {
        Ok(tables) => tables,
        Err(error) => {
            return table_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Could not inspect database tables: {error}"),
                is_htmx,
                None,
                &[],
            );
        }
    };
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(&table);

    if clean_table != table || !is_safe_identifier(&clean_table) || !tables.contains(&clean_table) {
        return table_error_response(
            StatusCode::NOT_FOUND,
            "The requested table is unavailable or uses an unsupported identifier.",
            is_htmx,
            None,
            &tables,
        );
    }

    let columns = match fetch_table_schema(pool, driver, &clean_table).await {
        Ok(columns) => columns,
        Err(err) => {
            return table_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Could not inspect the table schema: {err}"),
                is_htmx,
                Some(&clean_table),
                &tables,
            );
        }
    };

    let search_str = query.search.as_deref().unwrap_or("").trim();
    const MAX_PAGE: usize = 1_000_000;
    let page = query.page.unwrap_or(1).clamp(1, MAX_PAGE);
    let per_page = 25;
    let offset = (page - 1).saturating_mul(per_page);

    let total_records = match count_table_rows(
        &clean_table,
        if search_str.is_empty() {
            None
        } else {
            Some(search_str)
        },
    )
    .await
    {
        Ok(total) => total,
        Err(error) => {
            return table_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Could not count table records: {error}"),
                is_htmx,
                Some(&clean_table),
                &tables,
            );
        }
    };
    let total_pages = total_records.div_ceil(per_page);

    if columns.is_empty() {
        return table_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "This table has no columns inside Studio's supported identifier boundary.",
            is_htmx,
            Some(&clean_table),
            &tables,
        );
    }

    let col_names = columns
        .iter()
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    let primary_keys = columns
        .iter()
        .enumerate()
        .filter_map(|(index, column)| column.primary_key.then_some(index))
        .collect::<Vec<_>>();
    let supports_mutations = !primary_keys.is_empty()
        && primary_keys
            .iter()
            .all(|index| columns[*index].kind.is_editable());

    let quoted_table = quote_table_name(driver, &clean_table);
    let selected_columns = col_names
        .iter()
        .map(|column| {
            let quoted = quote_table_name(driver, column);
            if driver == "mysql" {
                format!("CAST({quoted} AS CHAR) AS {quoted}")
            } else {
                format!("CAST({quoted} AS TEXT) AS {quoted}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut qb: QueryBuilder<rullst_orm::RullstDatabase> =
        QueryBuilder::new(format!("SELECT {selected_columns} FROM {quoted_table}"));

    if !search_str.is_empty() && !col_names.is_empty() {
        qb.push(" WHERE ");
        let mut separated = qb.separated(" OR ");
        for col in &col_names {
            separated.push(build_search_clause(driver, col));
            separated.push_bind_unseparated(format!("%{}%", search_str));
        }
    }

    qb.push(" LIMIT ");
    qb.push_bind(per_page as i64);
    qb.push(" OFFSET ");
    qb.push_bind(offset as i64);

    let records = match qb.build().fetch_all(pool).await {
        Ok(records) => records,
        Err(error) => {
            return table_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Could not load table records: {error}"),
                is_htmx,
                Some(&clean_table),
                &tables,
            );
        }
    };

    let mut headers_html = build_headers_html(&col_names, &primary_keys);
    if supports_mutations {
        headers_html.push_str(
            "<th scope=\"col\" class=\"px-6 py-3.5 text-left text-xs font-bold text-slate-400 tracking-wider uppercase border-b border-slate-800/80\">Actions</th>",
        );
    }
    let rows_html = build_mutable_rows_html(&records, &columns, &clean_table);

    let content_html = format!(
        r##"<div class="p-8 font-mono space-y-6 max-w-7xl mx-auto">
            <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-4 border-b border-slate-800">
                <div>
                    <h1 class="text-2xl font-bold text-white tracking-tight flex items-center gap-3">
                        <span>{}</span>
                        <span class="text-xs px-2.5 py-0.5 rounded-full bg-sky-500/10 text-sky-400 border border-sky-500/20 uppercase font-mono">Table View</span>
                    </h1>
                    <p class="text-slate-400 text-xs mt-1">Inspect rows; primitive values may be changed only through the verified local Studio boundary</p>
                </div>
                <div class="flex items-center gap-3">
                    <input type="text"
                           name="search"
                           value="{}"
                           placeholder="Search records..."
                           hx-get="/studio/tables/{}"
                           hx-target="#studio-content"
                           hx-trigger="keyup changed delay:300ms"
                           hx-push-url="true"
                           class="bg-slate-900 border border-slate-800 rounded-lg px-3.5 py-1.5 text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-sky-500 transition w-64" />
                </div>
            </div>

            <div class="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-lg">
                <div class="overflow-x-auto">
                    <table class="w-full text-left border-collapse">
                        <thead class="bg-slate-950">
                            <tr>{}</tr>
                        </thead>
                        <tbody class="divide-y divide-slate-800/40 font-mono text-xs">
                            {}
                        </tbody>
                    </table>
                </div>
            </div>

            <div class="flex items-center justify-between pt-2 text-xs text-slate-400 font-mono">
                <div>
                    Showing <strong>{}</strong> to <strong>{}</strong> of <strong>{}</strong> records
                </div>
                <div class="flex items-center gap-2">
                    {}
                </div>
            </div>
        </div>"##,
        escape_html_attr(&clean_table),
        escape_html_attr(search_str),
        urlencoding::encode(&clean_table),
        headers_html,
        rows_html,
        if total_records == 0 { 0 } else { offset + 1 },
        (offset + records.len()).min(total_records),
        total_records,
        build_pagination_html(&clean_table, search_str, page, total_pages)
    );

    if is_htmx {
        Html(format!(
            "{}{}",
            content_html,
            render_sidebar_oob(&tables, Some(&clean_table))
        ))
        .into_response()
    } else {
        Html(studio_layout(content_html, Some(&clean_table), &tables)).into_response()
    }
}

fn table_error_response(
    status: StatusCode,
    message: &str,
    is_htmx: bool,
    active_table: Option<&str>,
    tables: &[String],
) -> axum::response::Response {
    let message = format!(
        "<p class=\"p-8 text-sm text-red-300\">{}</p>",
        rullst_core::html::escape_str(message)
    );
    let body = if is_htmx {
        message
    } else {
        studio_layout(message, active_table, tables)
    };
    (status, Html(body)).into_response()
}

fn build_pagination_html(
    clean_table: &str,
    search_str: &str,
    page: usize,
    total_pages: usize,
) -> String {
    if total_pages <= 1 {
        return String::new();
    }

    let mut html = String::new();
    let encoded_tbl = urlencoding::encode(clean_table);

    if page > 1 {
        let _ = write!(
            html,
            r##"<a href="#" hx-get="/studio/tables/{}?page={}&search={}" hx-target="#studio-content" hx-push-url="true" class="px-3 py-1 bg-slate-900 border border-slate-800 rounded hover:bg-slate-800 text-slate-300">Previous</a>"##,
            encoded_tbl,
            page - 1,
            urlencoding::encode(search_str)
        );
    }

    let _ = write!(
        html,
        r##"<span class="px-2 py-1 text-slate-500">Page {} of {}</span>"##,
        page, total_pages
    );

    if page < total_pages {
        let _ = write!(
            html,
            r##"<a href="#" hx-get="/studio/tables/{}?page={}&search={}" hx-target="#studio-content" hx-push-url="true" class="px-3 py-1 bg-slate-900 border border-slate-800 rounded hover:bg-slate-800 text-slate-300">Next</a>"##,
            encoded_tbl,
            page + 1,
            urlencoding::encode(search_str)
        );
    }

    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn pagination_is_bounded_encoded_and_directional() {
        assert!(build_pagination_html("records", "", 1, 1).is_empty());

        let first = build_pagination_html("audit records", "a&b", 1, 3);
        assert!(first.contains("Page 1 of 3"));
        assert!(first.contains("Next"));
        assert!(!first.contains("Previous"));
        assert!(first.contains("audit%20records"));
        assert!(first.contains("a%26b"));

        let middle = build_pagination_html("records", "", 2, 3);
        assert!(middle.contains("Previous"));
        assert!(middle.contains("Next"));

        let last = build_pagination_html("records", "", 3, 3);
        assert!(last.contains("Previous"));
        assert!(!last.contains("Next"));
    }

    #[tokio::test]
    async fn table_errors_preserve_status_escape_text_and_distinguish_htmx() {
        let htmx = table_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid <script>shape</script>",
            true,
            None,
            &[],
        );
        assert_eq!(htmx.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(htmx.into_body(), 8 * 1024)
            .await
            .expect("bounded HTMX error body");
        let body = String::from_utf8(body.to_vec()).expect("UTF-8 HTMX error body");
        assert!(body.contains("invalid &lt;script&gt;shape&lt;/script&gt;"));
        assert!(!body.contains("<!DOCTYPE html>"));

        let page = table_error_response(
            StatusCode::NOT_FOUND,
            "missing",
            false,
            Some("records"),
            &["records".to_string()],
        );
        assert_eq!(page.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(page.into_body(), 64 * 1024)
            .await
            .expect("bounded full-page error body");
        let body = String::from_utf8(body.to_vec()).expect("UTF-8 full-page error body");
        assert!(body.contains("<!DOCTYPE html>"));
        assert!(body.contains("missing"));
        assert!(body.contains("records"));
    }
}
