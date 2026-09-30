//! Migrations manager handler.

use super::super::db::*;
use super::super::layout::*;
use axum::response::{Html, IntoResponse};
use std::fmt::Write;

pub async fn handle_studio_tools_migrations(headers: axum::http::HeaderMap) -> impl IntoResponse {
    let is_htmx = headers.contains_key("hx-request");
    // Like the dashboard, report an unavailable database instead of an
    // apparently connected, empty schema.
    let tables = match fetch_tables().await {
        Ok(tables) => tables,
        Err(error) => {
            let notice = format!(
                r##"<div class="mt-8 pt-6 border-t border-slate-800"><p class="text-sm text-amber-300">Database unavailable: {}</p></div>"##,
                rullst_core::html::escape_str(&error.to_string())
            );
            let content = crate::migration_manager::render_migration_manager_html(&notice);
            return if is_htmx {
                Html(content).into_response()
            } else {
                Html(studio_layout(content, None, &[])).into_response()
            };
        }
    };
    let mut table_badges_html = String::new();
    let tables_count = tables.len();

    for t in &tables {
        let clean = escape_html_attr(t.as_str());
        let enc = urlencoding::encode(t.as_str());
        let _ = write!(
            table_badges_html,
            r##"<a href="/studio/tables/{}" class="p-3 bg-slate-900/90 border border-slate-800 rounded-lg hover:border-sky-500/60 hover:bg-slate-900 transition group flex items-center justify-between">
                    <span class="text-sm font-semibold text-slate-200 group-hover:text-sky-400">{}</span>
                    <span class="text-xs font-mono text-slate-500 group-hover:text-slate-400">tbl →</span>
                </a>"##,
            enc, clean
        );
    }

    let schema_section = if tables_count > 0 {
        format!(
            r##"<div class="mt-8 pt-6 border-t border-slate-800"><h2 class="text-lg font-bold text-slate-200 mb-4 flex items-center gap-2"><span>🗄️ Inspect Schema Tables ({})</span></h2><div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">{}</div></div>"##,
            tables_count, table_badges_html
        )
    } else {
        r##"<div class="mt-8 pt-6 border-t border-slate-800"><p class="text-sm text-slate-500">No application tables are currently visible.</p></div>"##.to_string()
    };

    let content = crate::migration_manager::render_migration_manager_html(&schema_section);
    if is_htmx {
        Html(content).into_response()
    } else {
        Html(studio_layout(content, None, &tables)).into_response()
    }
}
