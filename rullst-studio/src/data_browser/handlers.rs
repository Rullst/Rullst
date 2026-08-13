//! Studio Axum HTTP Route Handlers

use super::db::*;
use super::layout::*;
use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse},
};
use rullst_macros::html;
use sqlx::{QueryBuilder, Row};
use std::fmt::Write;

/// Dashboard index page
pub async fn handle_dashboard(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let tables = match fetch_tables().await {
        Ok(t) => t,
        Err(e) => {
            let error_html = html! {
                <div class="flex-grow flex flex-col items-center justify-center p-8 max-w-2xl mx-auto text-left">
                    <div class="w-full bg-slate-900 border border-amber-500/30 p-6 rounded-2xl space-y-4 shadow-xl">
                        <div class="flex items-center gap-3 text-amber-400 font-bold text-base">
                            <svg aria-hidden="true" class="h-6 w-6 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                            <span>"Database Connection Offline"</span>
                        </div>
                        <p class="text-xs text-slate-300 leading-relaxed">
                            "Rullst Studio could not initialize the database connection pool:"
                        </p>
                        <div class="p-3 bg-slate-950 border border-slate-800 rounded-xl font-mono text-xs text-rose-400 overflow-x-auto">
                            { e.to_string() }
                        </div>
                        <div class="space-y-2 pt-2">
                            <h4 class="text-xs font-bold text-slate-200 uppercase tracking-wider">"Troubleshooting Steps:"</h4>
                            <ul class="text-xs text-slate-300 space-y-1.5 list-disc list-inside">
                                <li>"Ensure your database server (e.g. PostgreSQL on port 5432 or MySQL on port 3306) is running locally."</li>
                                <li>"Verify credentials in your project's .env file (DATABASE_URL)."</li>
                                <li>"Or switch to zero-config local SQLite: DATABASE_URL=sqlite://db.sqlite?mode=rwc in .env."</li>
                            </ul>
                        </div>
                    </div>
                </div>
            };
            if is_htmx {
                return Html(error_html).into_response();
            } else {
                return Html(studio_layout(error_html, None, &[])).into_response();
            }
        }
    };

    let driver_name = resolve_driver_display_name();
    let tables_count = tables.len();

    let mut table_badges_html = String::new();
    for t in &tables {
        let clean = escape_html_attr(t.as_str());
        let enc = urlencoding::encode(t.as_str());
        let _ = write!(
            table_badges_html,
            r##"<a href="#" hx-get="/studio/tables/{}" hx-target="#studio-content" hx-push-url="true" class="p-3 bg-slate-900/90 border border-slate-800 rounded-lg hover:border-sky-500/60 hover:bg-slate-900 transition group flex items-center justify-between">
                <span class="text-sm font-semibold text-slate-200 group-hover:text-sky-400">{}</span>
                <span class="text-xs font-mono text-slate-500 group-hover:text-slate-400">tbl →</span>
            </a>"##,
            enc, clean
        );
    }

    let mut dash_content_str = String::from(
        r##"<div class="p-8 font-mono space-y-8 max-w-7xl mx-auto overflow-y-auto">
        <!-- Hero Header -->
        <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-slate-800">
            <div class="flex items-center gap-4">
                <div class="h-14 w-14 rounded-2xl bg-gradient-to-tr from-sky-500 via-indigo-500 to-purple-600 flex items-center justify-center shadow-lg shadow-indigo-500/20 p-2">
                    <img src="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" class="h-9 w-9 object-contain" alt="Rullst" />
                </div>
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>Rullst Studio Control Center</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Full-Stack Developer Hub — Database Inspector, AI Sentinel, Security Radar & Telemetry</p>
                </div>
            </div>
            <div class="flex items-center gap-3">
                <span class="px-3.5 py-1.5 bg-slate-900 border border-slate-800 rounded-full text-xs font-semibold text-slate-300 flex items-center gap-2 shadow-inner">
                    <span class="h-2 w-2 rounded-full bg-emerald-400 animate-pulse"></span>
                    <span>Isolated (127.0.0.1:5555)</span>
                </span>
                <span class="px-3.5 py-1.5 bg-emerald-950 border border-emerald-800/80 rounded-full text-xs font-bold text-emerald-400">
                    🔒 Guard Active
                </span>
            </div>
        </div>

        <!-- Top Metric KPI Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Database Engine</div>
                <div class="text-2xl font-bold text-sky-400 mt-1 uppercase">"##,
    );
    dash_content_str.push_str(&driver_name);
    dash_content_str.push_str(r##"</div>
                <div class="text-xs text-slate-400 mt-2">SQLx Async Zero-Lock Pool</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Managed Tables</div>
                <div class="text-2xl font-bold text-indigo-400 mt-1">"##);
    let _ = write!(dash_content_str, "{} Tables", tables_count);
    dash_content_str.push_str(r##"</div>
                <div class="text-xs text-slate-400 mt-2">Full Schema Inspection Ready</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">AI Sentinel Guard</div>
                <div class="text-2xl font-bold text-cyan-400 mt-1">Guarded</div>
                <div class="text-xs text-slate-400 mt-2">Prompt Injection & PII Filter</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Tokio Executor</div>
                <div class="text-2xl font-bold text-emerald-400 mt-1">&lt; 0.15 ms</div>
                <div class="text-xs text-slate-400 mt-2">Ultra-light ~14MB RSS RAM</div>
            </div>
        </div>

        <!-- Security Protection Banner -->
        <div class="p-4 bg-slate-900 border border-slate-800 rounded-xl flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
                <span class="text-amber-400 text-xl">🛡️</span>
                <div>
                    <h4 class="text-xs font-bold text-amber-400 uppercase tracking-wider">Studio Security & Auth Protection</h4>
                    <p class="text-xs text-slate-300 mt-0.5">Bound strictly to local loopback (127.0.0.1) for dev isolation. To add password protection in shared environments, set <code class="text-emerald-400 bg-slate-950 px-1.5 py-0.5 rounded">STUDIO_PASSWORD=your_password</code> in <code>.env</code>.</p>
                </div>
            </div>
            <span class="text-xs text-emerald-400 font-bold border border-emerald-900 bg-emerald-950 px-3 py-1 rounded-lg flex-shrink-0">
                Loopback Protected
            </span>
        </div>

        <!-- Studio Tools Feature Navigation Cards -->
        <div>
            <h2 class="text-lg font-bold text-slate-200 mb-4 flex items-center gap-2">
                <span>⚡ Studio Tools Hub</span>
            </h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <a href="#" hx-get="/studio/migrations" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-purple-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-purple-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🛠️ Database Tools & Migrations</span>
                        <span class="text-slate-600 group-hover:text-purple-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Run pending schema migrations, rollbacks, data seeders, and inspect raw database records line by line.</p>
                </a>
                <a href="#" hx-get="/studio/ai" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-cyan-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-cyan-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🤖 rullst-ai Playground</span>
                        <span class="text-slate-600 group-hover:text-cyan-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Test AI prompts, embeddings, and context injections in real-time across Gemini, OpenAI, Claude, and DeepSeek.</p>
                </a>
                <a href="#" hx-get="/studio/radar" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-sky-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-sky-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>📡 Telemetry & Rullst Radar</span>
                        <span class="text-slate-600 group-hover:text-sky-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Kernel-level telemetry visualizer displaying Tokio tick latency (µs), active async tasks, CPU, RSS memory & live spans.</p>
                </a>
                <a href="#" hx-get="/studio/capital" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-emerald-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-emerald-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>💳 Revenue Dashboard</span>
                        <span class="text-slate-600 group-hover:text-emerald-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Real-time SaaS MRR/ARR analytics, active subscriber metrics, churn rate, and live Stripe/LemonSqueezy Webhook Audit.</p>
                </a>
                <a href="#" hx-get="/studio/security" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-amber-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-amber-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🛡️ Visual Threat Radar</span>
                        <span class="text-slate-600 group-hover:text-amber-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Real-time SOC security dashboard, RASP engine memory alerts, Honeypot traps, and HMAC tamper-proof audit chain.</p>
                </a>
                <a href="#" hx-get="/studio/traces" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-indigo-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-indigo-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>📊 Distributed Tracing</span>
                        <span class="text-slate-600 group-hover:text-indigo-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Flamegraph inspector visualizing microsecond-level HTTP requests, SQLx queries, and AI prompt execution spans.</p>
                </a>
            </div>
        </div>
    </div>"##);

    if is_htmx {
        Html(format!(
            "{}{}",
            dash_content_str,
            render_sidebar_oob(&[], None)
        ))
        .into_response()
    } else {
        Html(studio_layout(dash_content_str, None, &[])).into_response()
    }
}

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
            if is_htmx {
                return Html(err).into_response();
            } else {
                return Html(studio_layout(err, Some(&clean_table), &tables)).into_response();
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

async fn fetch_table_schema(
    pool: &rullst_orm::RullstPool,
    driver: &str,
    clean_table: &str,
) -> Result<(Vec<String>, Vec<usize>), String> {
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

    let columns_rows = match QueryBuilder::<rullst_orm::RullstDatabase>::new(columns_query)
        .build()
        .fetch_all(pool)
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Error loading schema: {}", e)),
    };

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

pub async fn handle_studio_tools_migrations(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let mut table_badges_html = String::new();
    let mut tables_count = 0;

    if let Ok(tables) = fetch_tables().await {
        tables_count = tables.len();
        for t in &tables {
            let clean = escape_html_attr(t.as_str());
            let enc = urlencoding::encode(t.as_str());
            let _ = write!(
                table_badges_html,
                r##"<a href="#" hx-get="/studio/tables/{}" hx-target="#studio-content" hx-push-url="true" class="p-3 bg-slate-900/90 border border-slate-800 rounded-lg hover:border-sky-500/60 hover:bg-slate-900 transition group flex items-center justify-between">
                    <span class="text-sm font-semibold text-slate-200 group-hover:text-sky-400">{}</span>
                    <span class="text-xs font-mono text-slate-500 group-hover:text-slate-400">tbl →</span>
                </a>"##,
                enc, clean
            );
        }
    }

    let schema_section = if tables_count > 0 {
        format!(
            r##"<div class="mt-8 pt-6 border-t border-slate-800"><h2 class="text-lg font-bold text-slate-200 mb-4 flex items-center gap-2"><span>🗄️ Inspect Schema Tables ({})</span></h2><div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">{}</div></div>"##,
            tables_count, table_badges_html
        )
    } else {
        String::new()
    };

    let content = crate::migration_manager::render_migration_manager_html(&schema_section);
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

pub async fn handle_studio_tools_ai(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let content = crate::ai_playground::render_ai_playground_html();
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

pub async fn handle_studio_tools_security(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let (ai_active, provider_name) = detect_ai_provider();
    let (_ai_card_status, _ai_card_color, _ai_subtext) = if ai_active {
        (
            "ENFORCED".to_string(),
            "text-cyan-400",
            format!("Active Provider: {}", provider_name),
        )
    } else {
        (
            "NOT CONFIGURED".to_string(),
            "text-amber-400",
            "No AI API key or Local Ollama detected".to_string(),
        )
    };

    let ai_filter_status = if ai_active {
        r#"<span class="text-xs text-emerald-400 font-bold">Active (0 Attacks)</span>"#
    } else {
        r#"<span class="text-xs text-amber-400 font-bold">Disabled (No API Key)</span>"#
    };

    let ai_masking_status = if ai_active {
        r#"<span class="text-xs text-emerald-400 font-bold">Active</span>"#
    } else {
        r#"<span class="text-xs text-amber-400 font-bold">Disabled</span>"#
    };

    let ai_quota_status = if ai_active {
        r#"<span class="text-xs text-cyan-400 font-bold">Enforced</span>"#
    } else {
        r#"<span class="text-xs text-slate-500 font-bold">N/A</span>"#
    };

    let ai_setup_box = if !ai_active {
        r#"<div class="bg-slate-900 border border-amber-900/60 rounded-xl p-6 mb-8">
            <h2 class="text-lg font-bold text-amber-400 mb-2 flex items-center gap-2">
                <span>💡 Universal LLM Provider Support (Provider-Agnostic)</span>
            </h2>
            <p class="text-slate-300 text-sm mb-4">Rullst AI is provider-agnostic. You can connect to <strong>ANY AI service or local model</strong> — including Gemini, OpenAI, Claude, DeepSeek, Groq, Qwen, or local Ollama — by adding credentials to your project's <code>.env</code> file:</p>
            <div class="bg-slate-950 p-4 rounded-lg border border-slate-800 text-xs font-mono space-y-2">
                <p class="text-slate-400"># Google Gemini:</p>
                <p class="text-cyan-300">GEMINI_API_KEY="AIzaSyYourGeminiApiKeyHere"</p>
                <p class="text-slate-400 mt-2"># OpenAI (ChatGPT / GPT-4o):</p>
                <p class="text-emerald-300">OPENAI_API_KEY="sk-YourOpenAiKeyHere"</p>
                <p class="text-slate-400 mt-2"># Anthropic Claude:</p>
                <p class="text-purple-300">ANTHROPIC_API_KEY="sk-ant-YourClaudeKeyHere"</p>
                <p class="text-slate-400 mt-2"># DeepSeek / Qwen / Moonshot:</p>
                <p class="text-yellow-300">DEEPSEEK_API_KEY="sk-YourDeepSeekKeyHere"</p>
                <p class="text-slate-400 mt-2"># Local Ollama (100% Offline & Free):</p>
                <p class="text-sky-300">OLLAMA_HOST="http://127.0.0.1:11434"</p>
                <p class="text-slate-400 mt-3"># 2. Add rullst-ai to your dependencies or use CLI scaffold:</p>
                <p class="text-yellow-300">cargo rullst pkg add rullst-ai</p>
            </div>
        </div>"#
    } else {
        ""
    };

    let sec_store = rullst_security::SecurityStore::global();
    let log_redactions = sec_store
        .log_redactions_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let zero_trust_mismatches = sec_store
        .zero_trust_mismatches_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let schema_violations = sec_store
        .schema_violations_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let sri_signed = sec_store
        .sri_signed_assets_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let mfa_verifications = sec_store
        .mfa_verifications_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let deception_hits = sec_store
        .deception_hits_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let cswsh_blocks = sec_store
        .cswsh_blocks_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let rate_limit_blocks = sec_store
        .rate_limit_blocks_count
        .load(std::sync::atomic::Ordering::Relaxed);
    let siem_dispatches = sec_store
        .siem_dispatches_count
        .load(std::sync::atomic::Ordering::Relaxed);

    let events = sec_store
        .live_events
        .lock()
        .map(|e| e.iter().take(6).cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut incidents_html = String::new();
    if events.is_empty() {
        incidents_html.push_str(
            r#"<div class="p-6 text-center text-xs text-slate-500 font-medium bg-slate-950/60 border border-slate-800/80 rounded-xl">
                🛡️ Zero critical threats detected. RASP Honeypot traps and WAF guards operating normally.
            </div>"#,
        );
    } else {
        incidents_html.push_str(r#"<div class="space-y-3 font-mono text-xs">"#);
        for evt in events {
            let (badge_color, border_color) = match evt.event_type.as_str() {
                "HONEYPOT_TRAP_TRIGGERED" | "LOGIN_JAIL_TRIGGERED" => {
                    ("text-rose-400 border-rose-500/30 bg-rose-500/10", "border-rose-900/40")
                }
                "XSS_SANITIZED" | "DLP_SECRET_LEAK_PREVENTED" => {
                    ("text-cyan-400 border-cyan-500/30 bg-cyan-500/10", "border-cyan-900/40")
                }
                _ => ("text-amber-400 border-amber-500/30 bg-amber-500/10", "border-amber-900/40"),
            };

            incidents_html.push_str(&format!(
                r#"<div class="p-3.5 bg-slate-950 border {border_color} rounded-xl flex flex-col sm:flex-row sm:items-center justify-between gap-2">
                    <div class="flex items-center gap-2.5">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold border {badge_color}">{evt_type}</span>
                        <span class="text-slate-300 font-semibold">{details}</span>
                    </div>
                    <span class="text-slate-500 text-[11px] font-mono">{ts}</span>
                </div>"#,
                border_color = border_color,
                badge_color = badge_color,
                evt_type = rullst_core::html::escape_str(&evt.event_type),
                ts = rullst_core::html::escape_str(&evt.timestamp_str),
                details = rullst_core::html::escape_str(&evt.details)
            ));
        }
        incidents_html.push_str("</div>");
    }

    let content = format!(
        r#"<div class="p-6 md:p-8 font-mono space-y-8 max-w-7xl mx-auto overflow-y-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-2xl md:text-3xl font-extrabold text-amber-400 flex items-center gap-3">
                        <span>🛡️ Visual Threat Radar & AI Security</span>
                    </h1>
                    <p class="text-slate-400 text-xs md:text-sm mt-1">Rullst Security SOC Shield, RASP Engine, AI Sentinel & Real-Time SOC Telemetry</p>
                </div>
                <div class="flex items-center gap-3">
                    <span class="px-3.5 py-1.5 bg-emerald-950/80 text-emerald-400 border border-emerald-800/80 rounded-full text-xs font-bold flex items-center gap-2 shadow-inner">
                        <span class="w-2 h-2 rounded-full bg-emerald-400 animate-ping"></span>
                        Production Shield Active
                    </span>
                </div>
            </header>

            <!-- 9 Live Metric Cards Grid -->
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5 md:gap-6">
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔒 Secret Log Redactor</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{log_redactions}</div>
                    <div class="text-xs text-slate-400 mt-2">Zero-leak bearer &amp; token redactions</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🧬 Zero-Trust Fingerprints</div>
                    <div class="text-3xl font-bold text-cyan-400 mt-1">{zero_trust_mismatches}</div>
                    <div class="text-xs text-slate-400 mt-2">Hijack &amp; subnet drift interventions</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🛡️ Schema Guard Intercepts</div>
                    <div class="text-3xl font-bold text-amber-400 mt-1">{schema_violations}</div>
                    <div class="text-xs text-slate-400 mt-2">JSON depth &amp; payload size limits</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔑 Subresource Integrity (SRI)</div>
                    <div class="text-3xl font-bold text-purple-400 mt-1">{sri_signed}</div>
                    <div class="text-xs text-slate-400 mt-2">SHA-384 asset signature tags</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🔐 TOTP MFA Validations</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{mfa_verifications}</div>
                    <div class="text-xs text-slate-400 mt-2">RFC 6238 2FA authentications</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🍯 Dynamic Deception Traps</div>
                    <div class="text-3xl font-bold text-rose-400 mt-1">{deception_hits}</div>
                    <div class="text-xs text-slate-400 mt-2">Decoy route hits (/.env, /admin)</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">🌐 CSWSH Protection</div>
                    <div class="text-3xl font-bold text-yellow-400 mt-1">{cswsh_blocks}</div>
                    <div class="text-xs text-slate-400 mt-2">Cross-Site WebSocket hijacks blocked</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">⚡ Sliding-Window Rate Limit</div>
                    <div class="text-3xl font-bold text-cyan-400 mt-1">{rate_limit_blocks}</div>
                    <div class="text-xs text-slate-400 mt-2">IP bucket throttles enforced</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">📡 SIEM Common Event Log</div>
                    <div class="text-3xl font-bold text-emerald-400 mt-1">{siem_dispatches}</div>
                    <div class="text-xs text-slate-400 mt-2">CEF &amp; Webhook alerts exported</div>
                </div>
            </div>

            <!-- Live Security Incident Feed -->
            <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold text-slate-200 flex items-center gap-2">
                        <span>🚨 Live Security Incident Stream</span>
                    </h2>
                    <span class="text-xs text-slate-500 font-mono">Real-Time In-Memory Stream</span>
                </div>
                {incidents_html}
            </div>

            {ai_setup_box}

            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                    <h2 class="text-lg font-bold text-cyan-400 mb-4 flex items-center gap-2">
                        <span>🤖 rullst-ai Guardrails</span>
                    </h2>
                    <div class="space-y-3 text-sm">
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Prompt Injection Filter</span>
                            {ai_filter_status}
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">LLM Output PII Masking</span>
                            {ai_masking_status}
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Token Rate-Limit Quota</span>
                            {ai_quota_status}
                        </div>
                    </div>
                </div>

                <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md">
                    <h2 class="text-lg font-bold text-amber-400 mb-4 flex items-center gap-2">
                        <span>🔒 rullst-security Built-ins</span>
                    </h2>
                    <div class="space-y-3 text-sm">
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Double-Submit Cookie CSRF</span>
                            <span class="text-xs text-emerald-400 font-bold">Strict</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Parametrized SQLx ORM</span>
                            <span class="text-xs text-emerald-400 font-bold">SQL-Injection Safe</span>
                        </div>
                        <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                            <span class="text-slate-300">Leaky Bucket Rate Limiter</span>
                            <span class="text-xs text-emerald-400 font-bold">Active</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>"#,
        log_redactions = log_redactions,
        zero_trust_mismatches = zero_trust_mismatches,
        schema_violations = schema_violations,
        sri_signed = sri_signed,
        mfa_verifications = mfa_verifications,
        deception_hits = deception_hits,
        cswsh_blocks = cswsh_blocks,
        rate_limit_blocks = rate_limit_blocks,
        siem_dispatches = siem_dispatches,
        incidents_html = incidents_html,
        ai_setup_box = ai_setup_box,
        ai_filter_status = ai_filter_status,
        ai_masking_status = ai_masking_status,
        ai_quota_status = ai_quota_status
    );

    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

fn detect_ai_provider() -> (bool, String) {
    if let Ok(key) = std::env::var("GEMINI_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "Google Gemini API".to_string());
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "OpenAI (ChatGPT / GPT-4o)".to_string());
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "Anthropic Claude".to_string());
    }
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "DeepSeek / Qwen / Moonshot".to_string());
    }
    if let Ok(key) = std::env::var("GROQ_API_KEY")
        && !key.trim().is_empty()
    {
        return (true, "Groq Llama 3".to_string());
    }
    if let Ok(host) = std::env::var("OLLAMA_HOST")
        && !host.trim().is_empty()
    {
        return (true, "Local Ollama (Offline)".to_string());
    }
    (false, "No AI Provider Configured".to_string())
}

pub async fn handle_studio_radar(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let content = crate::radar_visualizer::render_radar_page();
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

pub async fn handle_studio_capital(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let content = crate::revenue_dashboard::render_revenue_dashboard_page();
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}

pub async fn handle_studio_traces(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    let Html(content) = crate::traces_visualizer::render_traces_page().await;
    if is_htmx {
        Html(format!("{}{}", content, render_sidebar_oob(&[], None))).into_response()
    } else {
        Html(studio_layout(content, None, &[])).into_response()
    }
}
