//! Studio HTML Layout & Navigation Components

use super::db::resolve_driver_display_name;
use rullst_core::html::RawHtml;
use rullst_macros::html;

pub fn render_sidebar_oob(tables: &[String], active_table: Option<&str>) -> String {
    if tables.is_empty() {
        return r#"<aside id="studio-sidebar" hx-swap-oob="outerHTML" class="hidden"></aside>"#
            .to_string();
    }

    let mut sidebar_links = String::new();
    for t in tables {
        let is_active = Some(t.as_str()) == active_table;
        let active_classes = if is_active {
            "bg-gradient-to-r from-sky-500/10 to-indigo-500/10 border-l-4 border-sky-400 text-sky-400 font-semibold"
        } else {
            "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40 border-l-4 border-transparent"
        };

        let path = format!("/studio/tables/{}", urlencoding::encode(t));
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

    format!(
        r##"<aside id="studio-sidebar" hx-swap-oob="outerHTML" class="w-72 bg-slate-900/60 border-r border-slate-800/80 flex flex-col flex-shrink-0 overflow-y-auto">
            <div class="p-4 border-b border-slate-800/50 flex items-center justify-between">
                <div>
                    <h2 class="text-xs font-bold text-slate-400 uppercase tracking-widest mb-0.5">Database Schema</h2>
                    <p class="text-[11px] text-slate-500 font-medium">Studio Tables ({})</p>
                </div>
                <span class="text-xs px-2 py-0.5 rounded bg-sky-950 text-sky-400 border border-sky-800 font-mono">{}</span>
            </div>
            <div class="flex-grow py-2">
                {}
            </div>
            <div class="p-4 border-t border-slate-800/40 text-center text-xs text-slate-500">
                Rullst Studio v{}
            </div>
        </aside>"##,
        tables.len(),
        resolve_driver_display_name(),
        sidebar_links,
        env!("CARGO_PKG_VERSION")
    )
}

/// Base visual template wrapper for all Studio pages
pub fn studio_layout(content: String, active_table: Option<&str>, tables: &[String]) -> String {
    let sidebar_markup = if !tables.is_empty() {
        render_sidebar_oob(tables, active_table).replace(r#"hx-swap-oob="outerHTML" "#, "")
    } else {
        r#"<aside id="studio-sidebar" class="hidden"></aside>"#.to_string()
    };

    let inner_html = html! {
        <html lang="en" class="h-full bg-slate-950">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>"Rullst Studio Control Center"</title>
            <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
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
            <header class="flex-shrink-0 bg-slate-900 border-b border-slate-800 px-6 py-3 flex flex-wrap items-center justify-between shadow-lg gap-4">
                <div class="flex items-center gap-3">
                    <a href="#" hx-get="/studio" hx-target="#studio-content" hx-push-url="true" class="flex items-center gap-2 group">
                        <span class="text-2xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                            "Rullst"
                        </span>
                        <span class="text-xs font-bold tracking-widest px-2 py-0.5 rounded bg-sky-500/10 text-sky-400 border border-sky-400/20 uppercase">
                            "Studio"
                        </span>
                    </a>
                </div>

                <nav class="flex items-center gap-1.5 bg-slate-950/80 p-1.5 rounded-xl border border-slate-800/80 overflow-x-auto text-xs font-semibold">
                    <a href="#" hx-get="/studio" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🏠 Control Center"</span>
                    </a>
                    <a href="#" hx-get="/studio/migrations" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🛠️ Database Tools"</span>
                    </a>
                    <a href="#" hx-get="/studio/ai" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🤖 AI Integration"</span>
                    </a>
                    <a href="#" hx-get="/studio/radar" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"📡 Radar & Telemetry"</span>
                    </a>
                    <a href="#" hx-get="/studio/capital" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"💳 Capital"</span>
                    </a>
                    <a href="#" hx-get="/studio/security" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🛡️ Threat Radar"</span>
                    </a>
                    <a href="#" hx-get="/studio/traces" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🔍 Traces"</span>
                    </a>
                </nav>

                <div class="flex items-center gap-2 bg-slate-950 border border-slate-800/80 px-3 py-1 rounded-full text-xs font-medium text-slate-300">
                    <span class="relative flex h-2 w-2">
                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                        <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                    </span>
                    "Local page"
                </div>
            </header>

            <div class="flex-grow flex overflow-hidden">
                { RawHtml(sidebar_markup) }

                <main id="studio-content" class="flex-grow flex flex-col overflow-y-auto bg-slate-950">
                    { RawHtml(content) }
                </main>
            </div>
        </body>
        </html>
    };

    format!("<!DOCTYPE html>{}", inner_html)
}
