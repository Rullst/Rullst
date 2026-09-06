//! Studio HTML Layout & Navigation Components

use rullst_core::html::RawHtml;
use rullst_macros::html;

/// Base visual template wrapper for all Studio pages
pub fn studio_layout(content: String, _active_table: Option<&str>, _tables: &[String]) -> String {
    let inner_html = html! {
        <html lang="en" class="min-h-full bg-slate-950">
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
        <body class="min-h-screen text-slate-100 flex flex-col antialiased selection:bg-sky-500/30 selection:text-sky-200">
            <header class="sticky top-0 z-50 flex-shrink-0 bg-slate-900 border-b border-slate-800 px-4 sm:px-6 py-3 flex flex-wrap items-center justify-between shadow-lg gap-3 lg:gap-4">
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

                <nav class="order-3 w-full min-w-0 lg:order-none lg:w-auto flex items-center gap-1.5 bg-slate-950/80 p-1.5 rounded-xl border border-slate-800/80 overflow-x-auto text-xs font-semibold">
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
                    <a href="#" hx-get="/studio/cache" hx-target="#studio-content" hx-push-url="true" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                        <span>"🧊 Cache"</span>
                    </a>
                </nav>

                <div class="flex items-center gap-2 bg-slate-950 border border-slate-800/80 px-3 py-1 rounded-full text-xs font-medium text-slate-300 whitespace-nowrap">
                    <span class="relative flex h-2 w-2">
                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                        <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                    </span>
                    "Local page"
                </div>
            </header>

            <div class="flex-grow flex min-w-0">
                <main id="studio-content" class="w-full min-w-0 flex-grow flex flex-col bg-slate-950">
                    { RawHtml(content) }
                </main>
            </div>
        </body>
        </html>
    };

    format!("<!DOCTYPE html>{}", inner_html)
}
