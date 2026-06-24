use crate::html::RawHtml;
use axum::{
    Router,
    extract::{Path, Query},
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_macros::html;

use serde::Deserialize;
use sqlx::{Any, QueryBuilder, Row};
use std::fmt::Write;

/// Query parameters for the Studio table viewer, supporting pagination and live search.
#[derive(Deserialize, Debug)]
pub struct TableQuery {
    page: Option<usize>,
    search: Option<String>,
}

/// Helper function to escape standard strings manually when building raw strings
fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Helper to decode any SQL Column value to String
fn get_any_value_as_string(row: &sqlx::any::AnyRow, index: usize) -> String {
    if let Ok(val) = row.try_get::<String, _>(index) {
        val
    } else if let Ok(val) = row.try_get::<i64, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<i32, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<f64, _>(index) {
        val.to_string()
    } else if let Ok(val) = row.try_get::<bool, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<String>, _>(index) {
        val
    } else if let Ok(Some(val)) = row.try_get::<Option<i64>, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<i32>, _>(index) {
        val.to_string()
    } else if let Ok(Some(val)) = row.try_get::<Option<bool>, _>(index) {
        val.to_string()
    } else {
        "NULL".to_string()
    }
}

/// Dynamic SQLite schema tables finder
async fn fetch_tables() -> Result<Vec<String>, sqlx::Error> {
    let pool = crate::db::safe_pool()
        .ok_or_else(|| sqlx::Error::Configuration("Database pool not initialized".into()))?;
    let driver = crate::db::safe_driver()
        .ok_or_else(|| sqlx::Error::Configuration("Database driver not initialized".into()))?;

    let query = match driver {
        "postgres" => {
            "SELECT CAST(table_name AS VARCHAR) as name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name ASC"
        }
        "mysql" => {
            "SELECT table_name as name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name ASC"
        }
        _ => {
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC"
        }
    };

    let rows = sqlx::query(query).fetch_all(pool).await?;

    let mut tables = Vec::new();
    for row in rows {
        if let Ok(name) = row.try_get::<String, _>(0) {
            tables.push(name);
        }
    }
    Ok(tables)
}

/// Dynamic SQLite table row counter
async fn count_table_rows(table: &str, search_query: Option<&str>) -> Result<usize, sqlx::Error> {
    let pool = crate::db::safe_pool()
        .ok_or_else(|| sqlx::Error::Configuration("Database pool not initialized".into()))?;
    let driver = crate::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(table);

    let quoted_table = if driver == "mysql" {
        format!("`{}`", clean_table)
    } else {
        format!("\"{}\"", clean_table)
    };

    let mut qb: QueryBuilder<Any> =
        QueryBuilder::new(format!("SELECT COUNT(*) FROM {}", quoted_table));

    if let Some(search) = search_query {
        if !search.is_empty() {
            let schema_query = match driver {
                "postgres" => format!(
                    "SELECT CAST(column_name AS VARCHAR) as name FROM information_schema.columns WHERE table_name = '{}' AND table_schema = 'public'",
                    clean_table
                ),
                "mysql" => format!(
                    "SELECT column_name as name FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
                    clean_table
                ),
                _ => format!("PRAGMA table_info(\"{}\")", clean_table),
            };
            if let Ok(columns_rows) = QueryBuilder::<Any>::new(schema_query)
                .build()
                .fetch_all(pool)
                .await
            {
                let mut col_names = Vec::new();
                for r in columns_rows {
                    if let Ok(name) = r.try_get::<String, _>("name") {
                        col_names.push(name);
                    }
                }
                if !col_names.is_empty() {
                    qb.push(" WHERE ");
                    let mut separated = qb.separated(" OR ");
                    for col in &col_names {
                        if driver == "postgres" {
                            separated.push(format!(
                                "CAST(\"{}\" AS TEXT) ILIKE ",
                                sanitize_identifier(col)
                            ));
                        } else if driver == "mysql" {
                            separated.push(format!(
                                "CAST(`{}` AS CHAR) LIKE ",
                                sanitize_identifier(col)
                            ));
                        } else {
                            separated.push(format!("\"{}\" LIKE ", sanitize_identifier(col)));
                        }
                        separated.push_bind_unseparated(format!("%{}%", search));
                    }
                }
            }
        }
    }

    let row = qb.build().fetch_one(pool).await?;
    let count: i64 = row.try_get(0).unwrap_or(0);
    Ok(count as usize)
}

/// Sanitize table and column names to prevent SQL injections in dynamic queries
fn sanitize_identifier(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64) // Strict length limit for security
        .collect()
}

/// Helper to build table headers HTML
fn build_headers_html(col_names: &[String], primary_keys: &[usize]) -> String {
    col_names.iter().enumerate().fold(
        String::with_capacity(col_names.len() * 128),
        |mut acc, (i, col)| {
            let is_pk = primary_keys.contains(&i);
            let pk_badge = if is_pk {
                "<span class=\"ml-1.5 text-[9px] font-extrabold tracking-widest bg-sky-500/10 text-sky-400 border border-sky-500/20 px-1 py-0.2 rounded font-mono\">PK</span>"
            } else {
                ""
            };
            let _ = write!(
                acc,
                "<th scope=\"col\" class=\"px-6 py-3.5 text-left text-xs font-bold text-slate-400 tracking-wider uppercase border-b border-slate-800/80\">\n                <div class=\"flex items-center\">{} {}</div>\n            </th>",
                escape_html_attr(col), pk_badge
            );
            acc
        },
    )
}

/// Helper to build table rows HTML
fn build_rows_html(records: &[sqlx::any::AnyRow], col_names: &[String]) -> String {
    if records.is_empty() {
        let cols_len = col_names.len().max(1);
        return format!(
            "<tr>\n                <td colspan=\"{}\" class=\"px-6 py-16 text-center text-sm text-slate-500 font-medium bg-slate-900/20\">\n                    No records found inside this table.\n                </td>\n            </tr>",
            cols_len
        );
    }

    records.iter().fold(
        String::with_capacity(records.len() * col_names.len() * 64),
        |mut rows_html, row| {
            rows_html.push_str("<tr class=\"border-b border-slate-800/40 hover:bg-slate-900/30 transition duration-150\">");
            for i in 0..col_names.len() {
                let cell_val = get_any_value_as_string(row, i);
                let is_null = cell_val == "NULL";
                let text_class = if is_null {
                    "text-slate-600 font-mono italic"
                } else {
                    "text-slate-300"
                };
                let _ = write!(
                    rows_html,
                    "<td class=\"px-6 py-4 text-sm truncate max-w-xs {}\">{}</td>",
                    text_class,
                    escape_html_attr(&cell_val)
                );
            }
            rows_html.push_str("</tr>");
            rows_html
        },
    )
}

/// Base visual template wrapper
fn studio_layout(content: String, active_table: Option<&str>, tables: &[String]) -> String {
    let mut sidebar_links = String::new();

    for t in tables {
        let is_active = Some(t.as_str()) == active_table;
        let active_classes = if is_active {
            "bg-gradient-to-r from-sky-500/10 to-indigo-500/10 border-l-4 border-sky-400 text-sky-400 font-semibold"
        } else {
            "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40 border-l-4 border-transparent"
        };

        let path = format!("/tables/{}", urlencoding::encode(t));
        let link_html = html! {
            <a href="#"
               hx-get={path.as_str()}
               hx-target="#studio-content"
               hx-push-url="true"
               class={format!("flex items-center justify-between px-4 py-3 text-sm transition-all duration-200 {}", active_classes).as_str()}>
                <span class="truncate">{t.as_str()}</span>
                <span class="text-xs px-2 py-0.5 rounded-full bg-slate-800 text-slate-500 group-hover:text-slate-400 font-mono">"tbl"</span>
            </a>
        };
        sidebar_links.push_str(&link_html);
    }

    let inner_html = html! {
        <html lang="en" class="h-full bg-slate-950">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>"Rullst Studio | Database Inspector"</title>
            <link rel="icon" type="image/png" href="data:image/png;base64,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" />
            <script src="https://cdn.tailwindcss.com"></script>
            <script src="https://unpkg.com/htmx.org@1.9.10"></script>
            <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
            <style>
                "body { font-family: 'Outfit', sans-serif; }"
                ":-webkit-scrollbar { width: 6px; height: 6px; }"
                ":-webkit-scrollbar-track { background: #0b0f19; }"
                ":-webkit-scrollbar-thumb { background: #1e293b; border-radius: 4px; }"
                ":-webkit-scrollbar-thumb:hover { background: #334155; }"
            </style>
        </head>
        <body class="h-full text-slate-100 flex flex-col antialiased selection:bg-sky-500/30 selection:text-sky-200">
            <header class="flex-shrink-0 bg-slate-900 border-b border-slate-800 px-6 py-4 flex items-center justify-between shadow-lg">
                <div class="flex items-center gap-3">
                    <span class="text-2xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                        "Rullst"
                    </span>
                    <span class="text-xs font-bold tracking-widest px-2 py-0.5 rounded bg-sky-500/10 text-sky-400 border border-sky-400/20 uppercase">
                        "Studio"
                    </span>
                </div>
                <div class="flex items-center gap-3 bg-slate-950 border border-slate-800/80 px-3.5 py-1.5 rounded-full text-xs font-medium text-slate-300 shadow-inner">
                    <span class="relative flex h-2.5 w-2.5">
                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                        <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-emerald-500"></span>
                    </span>
                    "Connected"
                </div>
            </header>

            <div class="flex-grow flex overflow-hidden">
                <aside class="w-72 bg-slate-900/60 border-r border-slate-800/80 flex flex-col flex-shrink-0 overflow-y-auto">
                    <div class="p-4 border-b border-slate-800/50">
                        <h2 class="text-xs font-bold text-slate-500 uppercase tracking-widest mb-1">"Database Schema"</h2>
                        <p class="text-[11px] text-slate-400 font-medium">"Studio DB"</p>
                    </div>
                    <div class="flex-grow py-2">
                        { RawHtml(sidebar_links) }
                    </div>
                    <div class="p-4 border-t border-slate-800/40 text-center text-xs text-slate-500">
                        { format!("Rullst Studio v{}", env!("CARGO_PKG_VERSION")) }
                    </div>
                </aside>

                <main id="studio-content" class="flex-grow flex flex-col overflow-hidden bg-slate-950">
                    { RawHtml(content) }
                </main>
            </div>
        </body>
        </html>
    };

    format!("<!DOCTYPE html>{}", inner_html)
}

/// Dashboard index page
pub async fn handle_dashboard() -> impl IntoResponse {
    let tables = match fetch_tables().await {
        Ok(t) => t,
        Err(e) => return Html(format!("Error loading schema: {}", e)).into_response(),
    };

    let dash_content = html! {
        <div class="flex-grow flex flex-col items-center justify-center p-12 text-center max-w-2xl mx-auto space-y-6">
            <div class="h-16 w-16 rounded-2xl bg-gradient-to-tr from-sky-400 to-indigo-500 flex items-center justify-center shadow-xl shadow-sky-500/10">
                <svg aria-hidden="true" class="h-8 w-8 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
                </svg>
            </div>
            <div class="space-y-2">
                <h1 class="text-3xl font-extrabold tracking-tight text-white">"Welcome to Rullst Studio"</h1>
                <p class="text-slate-400 text-sm leading-relaxed">
                    "Inspect tables, explore structural schemas, and view real-time records inside your dev database effortlessly."
                </p>
            </div>
            <div class="w-full grid grid-cols-2 gap-4 mt-8 text-left">
                <div class="p-4 rounded-xl bg-slate-900 border border-slate-800/80 shadow-md">
                    <span class="text-xs text-sky-400 font-bold uppercase tracking-wider">"Database Type"</span>
                    <h3 class="text-xl font-bold mt-1 text-slate-200 uppercase">{crate::db::safe_driver().unwrap_or("sqlite")}</h3>
                </div>
                <div class="p-4 rounded-xl bg-slate-900 border border-slate-800/80 shadow-md">
                    <span class="text-xs text-indigo-400 font-bold uppercase tracking-wider">"Total Tables"</span>
                    <h3 class="text-xl font-bold mt-1 text-slate-200">{tables.len()}</h3>
                </div>
            </div>
        </div>
    };

    Html(studio_layout(dash_content, None, &tables)).into_response()
}

/// HTMX-aware handler for rendering a paginated, searchable table view inside Rullst Studio.
/// Responds with a full HTML page on direct load, or an HTML fragment for HTMX partial updates.
pub async fn handle_table(
    headers: axum::http::HeaderMap,
    path: Path<String>,
    query: Query<TableQuery>,
) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let table_name = path.0;
    let tables = match fetch_tables().await {
        Ok(t) => t,
        Err(e) => return Html(format!("Error loading schema: {}", e)).into_response(),
    };

    if !tables.contains(&table_name) {
        return Html(format!("Table '{}' not found.", table_name)).into_response();
    }

    let page = query.page.unwrap_or(1);
    let page_size = 15;
    let offset = (page - 1) * page_size;
    let search = query.search.as_deref().unwrap_or("");

    let total_rows = match count_table_rows(&table_name, query.search.as_deref()).await {
        Ok(c) => c,
        Err(e) => return Html(format!("Error counting rows: {}", e)).into_response(),
    };

    let total_pages = (total_rows as f64 / page_size as f64).ceil() as usize;
    let pool = match crate::db::safe_pool() {
        Some(p) => p,
        None => {
            return Html(
                "Database pool not initialized. Please configure database_url.".to_string(),
            )
            .into_response();
        }
    };
    let driver = crate::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(&table_name);

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

    let columns_rows = match QueryBuilder::<Any>::new(columns_query)
        .build()
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => return Html(format!("Error loading schema: {}", e)).into_response(),
    };

    let mut col_names = Vec::new();
    let mut primary_keys = Vec::new();
    for r in &columns_rows {
        if let Ok(name) = r.try_get::<String, _>("name") {
            let is_pk = if let Ok(pk) = r.try_get::<i64, _>("pk") {
                pk > 0
            } else if let Ok(pk) = r.try_get::<i32, _>("pk") {
                pk > 0
            } else {
                false
            };

            if is_pk {
                primary_keys.push(col_names.len());
            }
            col_names.push(name);
        }
    }

    let quoted_table = if driver == "mysql" {
        format!("`{}`", clean_table)
    } else {
        format!("\"{}\"", clean_table)
    };

    // Dynamic records list
    let mut qb: QueryBuilder<Any> = QueryBuilder::new(format!("SELECT * FROM {}", quoted_table));

    if !search.is_empty() && !col_names.is_empty() {
        qb.push(" WHERE ");
        let mut separated = qb.separated(" OR ");
        for col in &col_names {
            if driver == "postgres" {
                separated.push(format!(
                    "CAST(\"{}\" AS TEXT) ILIKE ",
                    sanitize_identifier(col)
                ));
            } else if driver == "mysql" {
                separated.push(format!(
                    "CAST(`{}` AS CHAR) LIKE ",
                    sanitize_identifier(col)
                ));
            } else {
                separated.push(format!("\"{}\" LIKE ", sanitize_identifier(col)));
            }
            separated.push_bind_unseparated(format!("%{}%", search));
        }
    }

    qb.push(format!(" LIMIT {} OFFSET {}", page_size, offset));

    let records = match qb.build().fetch_all(pool).await {
        Ok(recs) => recs,
        Err(e) => return Html(format!("Error loading records: {}", e)).into_response(),
    };

    // Build headers HTML
    let headers_html = build_headers_html(&col_names, &primary_keys);

    // Build rows HTML
    let rows_html = build_rows_html(&records, &col_names);

    let prev_page = if page > 1 { page - 1 } else { 1 };
    let next_page = if page < total_pages {
        page + 1
    } else {
        total_pages
    };

    let prev_hx = format!(
        "/tables/{}?page={}&search={}",
        urlencoding::encode(&table_name),
        prev_page,
        urlencoding::encode(search)
    );
    let next_hx = format!(
        "/tables/{}?page={}&search={}",
        urlencoding::encode(&table_name),
        next_page,
        urlencoding::encode(search)
    );

    let prev_disabled = if page <= 1 {
        "opacity-50 cursor-not-allowed"
    } else {
        "hover:bg-slate-800 hover:text-white"
    };
    let next_disabled = if page >= total_pages {
        "opacity-50 cursor-not-allowed"
    } else {
        "hover:bg-slate-800 hover:text-white"
    };

    let table_main_html = html! {
        <div class="flex-grow flex flex-col overflow-hidden h-full">
            <div class="flex-shrink-0 bg-slate-900/30 border-b border-slate-800/80 px-6 py-4 flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                <div class="space-y-1">
                    <div class="flex items-center gap-2.5">
                        <h1 class="text-xl font-extrabold text-white tracking-tight">{table_name.as_str()}</h1>
                        <span class="text-xs px-2.5 py-0.5 rounded-full bg-slate-850 border border-slate-700/60 text-slate-400 font-mono font-medium shadow-sm">
                            {total_rows} " rows"
                        </span>
                    </div>
                    <p class="text-[11px] text-slate-500 font-medium">"Displaying up to 15 records per page."</p>
                </div>

                <div class="flex items-center gap-3">
                    <div class="relative">
                        <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-500">
                            <svg aria-hidden="true" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                            </svg>
                        </div>
                        <input type="text"
                               name="search"
                               value={search}
                               placeholder="Search records..."
                               hx-get={format!("/tables/{}", urlencoding::encode(&table_name)).as_str()}
                               hx-trigger="keyup[target.value.length == 0 || target.value.length > 2] delay:400ms, search"
                               hx-target="#studio-content"
                               class="w-64 pl-9 pr-4 py-2 bg-slate-950 border border-slate-800 rounded-lg text-sm text-slate-200 placeholder-slate-500 focus:outline-none focus:border-sky-500/80 focus:ring-1 focus:ring-sky-500/50 transition-all duration-200" />
                    </div>
                </div>
            </div>

            <div class="flex-grow overflow-auto p-6">
                <div class="rounded-xl border border-slate-800 bg-slate-900/40 overflow-hidden shadow-xl">
                    <div class="overflow-x-auto">
                        <table class="min-w-full divide-y divide-slate-800 table-fixed">
                            <thead class="bg-slate-900/70">
                                <tr>
                                    { RawHtml(headers_html) }
                                </tr>
                            </thead>
                            <tbody class="divide-y divide-slate-800/30">
                                { RawHtml(rows_html) }
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>

            <div class="flex-shrink-0 bg-slate-900/20 border-t border-slate-800/80 px-6 py-4 flex items-center justify-between">
                <div class="text-xs text-slate-400 font-medium">
                    "Page " <span class="text-slate-200 font-bold">{page}</span> " of " <span class="text-slate-200 font-bold">{total_pages.max(1)}</span>
                </div>
                <div class="flex items-center gap-2">
                    <button hx-get={prev_hx.as_str()}
                            hx-target="#studio-content"
                            disabled={page <= 1}
                            class={format!("px-4 py-2 bg-slate-900 border border-slate-800 text-slate-300 text-xs font-semibold rounded-lg shadow transition duration-150 cursor-pointer {}", prev_disabled).as_str()}>
                        "Previous"
                    </button>
                    <button hx-get={next_hx.as_str()}
                            hx-target="#studio-content"
                            disabled={page >= total_pages}
                            class={format!("px-4 py-2 bg-slate-900 border border-slate-800 text-slate-300 text-xs font-semibold rounded-lg shadow transition duration-150 cursor-pointer {}", next_disabled).as_str()}>
                        "Next"
                    </button>
                </div>
            </div>
        </div>
    };

    if is_htmx {
        Html(table_main_html).into_response()
    } else {
        Html(studio_layout(table_main_html, Some(&table_name), &tables)).into_response()
    }
}

/// Actual clean routes wrapper
pub async fn run_studio(_db_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(handle_dashboard))
        .route("/tables/{table_name}", get(handle_table));

    let host_str = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr_str = format!("{}:5555", host_str);
    let listener = tokio::net::TcpListener::bind(&addr_str).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html_attr() {
        let input = r#"<script>alert("XSS & Hack")</script> 'test'"#;
        let expected =
            "&lt;script&gt;alert(&quot;XSS &amp; Hack&quot;)&lt;/script&gt; &#x27;test&#x27;";
        assert_eq!(escape_html_attr(input), expected);
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("valid_table_123"), "valid_table_123");
        assert_eq!(sanitize_identifier("invalid-table!@#"), "invalidtable");
        assert_eq!(sanitize_identifier("drop table users;--"), "droptableusers");

        // Test length limit 64
        let long_id = "a".repeat(100);
        assert_eq!(sanitize_identifier(&long_id).len(), 64);
    }

    #[test]
    fn test_build_headers_html() {
        let cols = vec!["id".to_string(), "name".to_string()];
        let pks = vec![0];
        let html = build_headers_html(&cols, &pks);

        assert!(html.contains("id"));
        assert!(html.contains("name"));
        assert!(html.contains("PK")); // id is PK

        let cols2 = vec!["created_at".to_string()];
        let html2 = build_headers_html(&cols2, &[]);
        assert!(html2.contains("created_at"));
        assert!(!html2.contains("PK"));
    }

    #[test]
    fn test_studio_layout() {
        let tables = vec!["users".to_string(), "posts".to_string()];
        let content = "<h1>Main Content</h1>".to_string();

        let html = studio_layout(content, Some("users"), &tables);

        assert!(html.contains("Main Content"));
        assert!(html.contains("users"));
        assert!(html.contains("posts"));
        assert!(html.contains("bg-gradient-to-r from-sky-500/10")); // Active class should be applied

        let inactive_html = studio_layout("content".into(), Some("nonexistent"), &tables);
        assert!(inactive_html.contains("users"));
        assert!(inactive_html.contains("hover:text-slate-200")); // Inactive class
    }

    #[tokio::test]
    async fn test_db_operations() {
        let db_path = "sqlite:file:studio_test_db?mode=memory&cache=shared";
        
        let _ = rullst_orm::Orm::init(db_path).await;
        let pool = crate::db::safe_pool().expect("pool should be initialized");

        let _ = sqlx::query("DROP TABLE IF EXISTS test_users").execute(pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS test_posts").execute(pool).await;

        // Create dummy tables
        sqlx::query("CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT);")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE test_posts (id INTEGER PRIMARY KEY, title TEXT);")
            .execute(pool)
            .await
            .unwrap();

        // Insert dummy data
        sqlx::query("INSERT INTO test_users (name) VALUES ('Alice'), ('Bob')")
            .execute(pool)
            .await
            .unwrap();

        let tables = fetch_tables().await.unwrap();
        assert!(tables.contains(&"test_users".to_string()));
        assert!(tables.contains(&"test_posts".to_string()));

        let users_count = count_table_rows("test_users", None).await.unwrap();
        assert_eq!(users_count, 2);

        let search_count = count_table_rows("test_users", Some("Alice")).await.unwrap();
        assert_eq!(search_count, 1);
    }
}
