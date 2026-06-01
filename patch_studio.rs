use crate::html::RawHtml;
use axum::{
    Router,
    extract::{Path, Query},
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_macros::html;
use rust_eloquent::Eloquent;
use serde::Deserialize;
use sqlx::Row;
use sqlx::{QueryBuilder, Any};
use std::net::SocketAddr;

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
    let pool = Eloquent::pool();
    let rows = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await?;

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
    let pool = Eloquent::pool();
    let clean_table = sanitize_identifier(table);

    let mut qb: QueryBuilder<Any> = QueryBuilder::new(format!("SELECT COUNT(*) FROM \"{}\"", clean_table));

    if let Some(search) = search_query {
        if !search.is_empty() {
            let schema_query = format!("PRAGMA table_info(\"{}\")", clean_table);
            if let Ok(columns_rows) = sqlx::query(&schema_query).fetch_all(pool).await {
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
                        separated.push(format!("\"{}\" LIKE ", sanitize_identifier(col)));
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
        .collect()
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

        let path = format!("/tables/{}", t);
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
                        <p class="text-[11px] text-slate-400 font-medium">"SQLite Database"</p>
                    </div>
                    <div class="flex-grow py-2">
                        { RawHtml(sidebar_links) }
                    </div>
                    <div class="p-4 border-t border-slate-800/40 text-center text-xs text-slate-500">
                        "Rullst Studio v0.9.2"
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
async fn handle_dashboard() -> impl IntoResponse {
    let tables = match fetch_tables().await {
        Ok(t) => t,
        Err(e) => return Html(format!("Error loading schema: {}", e)).into_response(),
    };

    let dash_content = html! {
        <div class="flex-grow flex flex-col items-center justify-center p-12 text-center max-w-2xl mx-auto space-y-6">
            <div class="h-16 w-16 rounded-2xl bg-gradient-to-tr from-sky-400 to-indigo-500 flex items-center justify-center shadow-xl shadow-sky-500/10">
                <svg class="h-8 w-8 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
                </svg>
            </div>
            <div class="space-y-2">
                <h1 class="text-3xl font-extrabold tracking-tight text-white">"Welcome to Rullst Studio"</h1>
                <p class="text-slate-400 text-sm leading-relaxed">
                    "Inspect tables, explore structural schemas, and view real-time records inside your SQLite dev database effortlessly."
                </p>
            </div>
            <div class="w-full grid grid-cols-2 gap-4 mt-8 text-left">
                <div class="p-4 rounded-xl bg-slate-900 border border-slate-800/80 shadow-md">
                    <span class="text-xs text-sky-400 font-bold uppercase tracking-wider">"Database Size"</span>
                    <h3 class="text-xl font-bold mt-1 text-slate-200">"Local SQLite"</h3>
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

/// Actual clean routes wrapper
pub async fn run_studio(_db_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/", get(handle_dashboard))
        .route("/tables/:table_name", get(|
            headers: axum::http::HeaderMap,
            path: Path<String>,
            query: Query<TableQuery>
        | async move {
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

            let pool = Eloquent::pool();
            let clean_table = sanitize_identifier(&table_name);

            // Retrieve columns schema
            let schema_query = format!("PRAGMA table_info(\"{}\")", clean_table);
            let columns_rows = match sqlx::query(&schema_query).fetch_all(pool).await {
                Ok(rows) => rows,
                Err(e) => return Html(format!("Error retrieving column schema: {}", e)).into_response(),
            };

            let mut col_names = Vec::new();
            let mut primary_keys = Vec::new();
            for r in &columns_rows {
                if let Ok(name) = r.try_get::<String, _>("name") {
                    col_names.push(name.clone());
                    if let Ok(pk) = r.try_get::<i32, _>("pk") {
                        if pk > 0 {
                            primary_keys.push(name);
                        }
                    }
                }
            }

            // Dynamic records list
            let mut qb: QueryBuilder<Any> = QueryBuilder::new(format!("SELECT * FROM \"{}\"", clean_table));

            if !search.is_empty() && !col_names.is_empty() {
                qb.push(" WHERE ");
                let mut separated = qb.separated(" OR ");
                for col in &col_names {
                    separated.push(format!("\"{}\" LIKE ", sanitize_identifier(col)));
                    separated.push_bind_unseparated(format!("%{}%", search));
                }
            }

            qb.push(" LIMIT ");
            qb.push_bind(page_size as i64);
            qb.push(" OFFSET ");
            qb.push_bind(offset as i64);

            let records = match qb.build().fetch_all(pool).await {
                Ok(recs) => recs,
                Err(e) => return Html(format!("Error loading records: {}", e)).into_response(),
            };

            // Build headers HTML
            let mut headers_html = String::new();
            for col in &col_names {
                let is_pk = primary_keys.contains(col);
                let pk_badge = if is_pk {
                    "<span class=\"ml-1.5 text-[9px] font-extrabold tracking-widest bg-sky-500/10 text-sky-400 border border-sky-500/20 px-1 py-0.2 rounded font-mono\">PK</span>"
                } else {
                    ""
                };
                headers_html.push_str(&format!(
                    "<th scope=\"col\" class=\"px-6 py-3.5 text-left text-xs font-bold text-slate-400 tracking-wider uppercase border-b border-slate-800/80\">
                        <div class=\"flex items-center\">{} {}</div>
                    </th>",
                    escape_html_attr(col), pk_badge
                ));
            }

            // Build rows HTML
            let mut rows_html = String::new();
            if records.is_empty() {
                let cols_len = col_names.len().max(1);
                rows_html.push_str(&format!(
                    "<tr>
                        <td colspan=\"{}\" class=\"px-6 py-16 text-center text-sm text-slate-500 font-medium bg-slate-900/20\">
                            No records found inside this table.
                        </td>
                    </tr>",
                    cols_len
                ));
            } else {
                for row in records {
                    rows_html.push_str("<tr class=\"border-b border-slate-800/40 hover:bg-slate-900/30 transition duration-150\">");
                    for i in 0..col_names.len() {
                        let cell_val = get_any_value_as_string(&row, i);
                        let is_null = cell_val == "NULL";
                        let text_class = if is_null {
                            "text-slate-600 font-mono italic"
                        } else {
                            "text-slate-300"
                        };
                        rows_html.push_str(&format!(
                            "<td class=\"px-6 py-4 text-sm truncate max-w-xs {}\">{}</td>",
                            text_class, escape_html_attr(&cell_val)
                        ));
                    }
                    rows_html.push_str("</tr>");
                }
            }

            let prev_page = if page > 1 { page - 1 } else { 1 };
            let next_page = if page < total_pages { page + 1 } else { total_pages };

            let prev_hx = format!("/tables/{}?page={}&search={}", table_name, prev_page, escape_html_attr(search));
            let next_hx = format!("/tables/{}?page={}&search={}", table_name, next_page, escape_html_attr(search));

            let prev_disabled = if page <= 1 { "opacity-50 cursor-not-allowed" } else { "hover:bg-slate-800 hover:text-white" };
            let next_disabled = if page >= total_pages { "opacity-50 cursor-not-allowed" } else { "hover:bg-slate-800 hover:text-white" };

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
                                    <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                                    </svg>
                                </div>
                                <input type="text"
                                       name="search"
                                       value={search}
                                       placeholder="Search records..."
                                       hx-get={format!("/tables/{}", table_name).as_str()}
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
        }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 5555));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
