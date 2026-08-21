//! Table browser handler — schema inspection, paginated data view, search.

use super::super::db::*;
use super::super::layout::*;
use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse},
};
use sqlx::{QueryBuilder, Row};
use std::fmt::Write;

pub async fn handle_table(
    Path(table): Path<String>,
    Query(query): Query<TableQuery>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let pool = match ensure_pool_initialized().await {
        Ok(p) => p,
        Err(e) => return Html(format!("Database Error: {}", e)).into_response(),
    };

    let tables = fetch_tables().await.unwrap_or_default();
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(&table);

    if !tables.contains(&clean_table) {
        let not_found_html = format!("Table '{}' not found.", clean_table);
        if is_htmx {
            return Html(not_found_html).into_response();
        } else {
            return Html(studio_layout(not_found_html, None, &tables)).into_response();
        }
    }

    let (col_names, primary_keys) = match fetch_table_schema(pool, driver, &clean_table).await {
        Ok(res) => res,
        Err(err) => {
            let err_html = format!("Error loading schema: {}", err);
            if is_htmx {
                return Html(err_html).into_response();
            } else {
                return Html(studio_layout(err_html, Some(&clean_table), &tables)).into_response();
            }
        }
    };

    let search_str = query.search.as_deref().unwrap_or("").trim();
    let page = query.page.unwrap_or(1).max(1);
    let per_page = 25;
    let offset = (page - 1) * per_page;

    let total_records = count_table_rows(
        &clean_table,
        if search_str.is_empty() {
            None
        } else {
            Some(search_str)
        },
    )
    .await
    .unwrap_or(0);
    let total_pages = total_records.div_ceil(per_page);

    let quoted_table = quote_table_name(driver, &clean_table);
    let mut qb: QueryBuilder<rullst_orm::RullstDatabase> =
        QueryBuilder::new(format!("SELECT * FROM {}", quoted_table));

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

    let records = qb.build().fetch_all(pool).await.unwrap_or_default();

    let headers_html = build_headers_html(&col_names, &primary_keys);
    let rows_html = build_rows_html(&records, &col_names);

    let content_html = format!(
        r##"<div class="p-8 font-mono space-y-6 max-w-7xl mx-auto">
            <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-4 border-b border-slate-800">
                <div>
                    <h1 class="text-2xl font-bold text-white tracking-tight flex items-center gap-3">
                        <span>{}</span>
                        <span class="text-xs px-2.5 py-0.5 rounded-full bg-sky-500/10 text-sky-400 border border-sky-500/20 uppercase font-mono">Table View</span>
                    </h1>
                    <p class="text-slate-400 text-xs mt-1">Inspecting raw database schema records line by line</p>
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

pub(crate) async fn fetch_table_schema(
    pool: &rullst_orm::RullstPool,
    driver: &str,
    clean_table: &str,
) -> Result<(Vec<String>, Vec<usize>), rullst_orm::Error> {
    let columns_query = match driver {
        "postgres" => format!("
            SELECT CAST(c.column_name AS VARCHAR) as name,
            CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN 1 ELSE 0 END as pk
            FROM information_schema.columns c
            LEFT JOIN information_schema.key_column_usage kcu
              ON c.table_name = kcu.table_name AND CAST(c.column_name AS VARCHAR) = CAST(kcu.column_name AS VARCHAR) AND kcu.table_schema = 'public'
            LEFT JOIN information_schema.table_constraints tc
              ON kcu.constraint_name = tc.constraint_name AND tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = 'public'
            WHERE c.table_name = '{}' AND c.table_schema = 'public'
        ", clean_table),
        "mysql" => format!("
            SELECT column_name as name,
            CASE WHEN column_key = 'PRI' THEN 1 ELSE 0 END as pk
            FROM information_schema.columns
            WHERE table_name = '{}' AND table_schema = DATABASE()
        ", clean_table),
        _ => format!("PRAGMA table_info(\"{}\")", clean_table),
    };

    let columns_rows = QueryBuilder::<rullst_orm::RullstDatabase>::new(columns_query)
        .build()
        .fetch_all(pool)
        .await?;

    let mut col_names = Vec::new();
    let mut primary_keys = Vec::new();

    for (idx, r) in columns_rows.into_iter().enumerate() {
        let name: String = r.try_get("name").unwrap_or_default();
        let is_pk: i32 = r.try_get("pk").unwrap_or(0);
        col_names.push(name);
        if is_pk == 1 {
            primary_keys.push(idx);
        }
    }

    Ok((col_names, primary_keys))
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
