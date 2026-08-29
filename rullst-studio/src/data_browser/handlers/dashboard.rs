//! Dashboard index page handler for Rullst Studio.

use super::super::db::*;
use super::super::layout::*;
use axum::response::{Html, IntoResponse};
use rullst_macros::html;
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
                <div class="h-14 w-14 rounded-2xl bg-gradient-to-tr from-sky-500 via-indigo-500 to-purple-600 flex items-center justify-center shadow-lg shadow-indigo-500/20 p-2" aria-hidden="true">R</div>
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
                    <span>Supported mode: loopback</span>
                </span>
                <span class="px-3.5 py-1.5 bg-emerald-950 border border-emerald-800/80 rounded-full text-xs font-bold text-emerald-400">
                    Verify direct peer
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
                <div class="text-xs text-slate-400 mt-2">SQLx async connection pool</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Managed Tables</div>
                <div class="text-2xl font-bold text-indigo-400 mt-1">"##);
    let _ = write!(dash_content_str, "{} Tables", tables_count);
    dash_content_str.push_str(r##"</div>
                <div class="text-xs text-slate-400 mt-2">Tables visible to the configured connection</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">AI Integration</div>
                <div class="text-2xl font-bold text-cyan-400 mt-1">Not connected</div>
                <div class="text-xs text-slate-400 mt-2">Application-owned client required</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Runtime Telemetry</div>
                <div class="text-2xl font-bold text-emerald-400 mt-1">Radar</div>
                <div class="text-xs text-slate-400 mt-2">Open the Radar for live supported probes</div>
            </div>
        </div>

        <!-- Security Protection Banner -->
        <div class="p-4 bg-slate-900 border border-slate-800 rounded-xl flex items-center justify-between gap-4">
            <div class="flex items-center gap-3">
                <span class="text-amber-400 text-xl">🛡️</span>
                <div>
                    <h4 class="text-xs font-bold text-amber-400 uppercase tracking-wider">Studio Security & Auth Protection</h4>
                    <p class="text-xs text-slate-300 mt-0.5">The standalone Studio server is bound strictly to local loopback (127.0.0.1) for development. It has no built-in shared-environment password mode; do not expose it publicly. A shared deployment must add an authenticated reverse-proxy or application boundary explicitly.</p>
                </div>
            </div>
            <span class="text-xs text-emerald-400 font-bold border border-emerald-900 bg-emerald-950 px-3 py-1 rounded-lg flex-shrink-0">
                Supported launcher only
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
                    <p class="text-slate-400 text-sm">Inspect the configured schema and view explicit CLI guidance. Application migrations are not inferred by Studio.</p>
                </a>
                <a href="#" hx-get="/studio/ai" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-cyan-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-cyan-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🤖 rullst-ai Integration</span>
                        <span class="text-slate-600 group-hover:text-cyan-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Review the explicit AI integration boundary. No client or provider call is inferred from environment variables.</p>
                </a>
                <a href="#" hx-get="/studio/radar" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-sky-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-sky-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>📡 Telemetry & Rullst Radar</span>
                        <span class="text-slate-600 group-hover:text-sky-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Process and Tokio telemetry displaying supported task, CPU, RSS, uptime, scheduler-yield and local span observations.</p>
                </a>
                <a href="#" hx-get="/studio/capital" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-emerald-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-emerald-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>💳 Revenue Dashboard</span>
                        <span class="text-slate-600 group-hover:text-emerald-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Inspect application-supplied revenue metrics and webhook records held by the local dashboard manager.</p>
                </a>
                <a href="#" hx-get="/studio/security" hx-target="#studio-content" hx-push-url="true" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-amber-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-amber-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🛡️ Visual Threat Radar</span>
                        <span class="text-slate-600 group-hover:text-amber-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Local security telemetry for RASP observations, honeypot events, and HMAC-linked audit records. Operational response remains application-owned.</p>
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
