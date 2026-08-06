use axum::response::Html;

/// Renders the Distributed Tracing Flamegraph Inspector page fragment for Rullst Studio (`/studio/traces`).
pub async fn render_traces_page() -> Html<String> {
    let collector = rullst_core::telemetry_spans::global_span_collector();
    let spans = collector.snapshot();

    let mut rows_html = String::new();
    if spans.is_empty() {
        rows_html.push_str(
            r#"<tr>
                <td colspan="4" class="px-6 py-12 text-center text-sm text-slate-500 font-medium bg-slate-950/40">
                    No trace spans recorded yet. Execute HTTP requests or ORM database queries to generate live traces.
                </td>
            </tr>"#,
        );
    } else {
        for span in spans.iter().rev() {
            let (badge_class, badge_text) = match span.kind.as_str() {
                "sql" => (
                    "bg-blue-500/10 text-blue-400 border-blue-500/20",
                    "SQL QUERY",
                ),
                "ai" => (
                    "bg-purple-500/10 text-purple-400 border-purple-500/20",
                    "AI GENERATION",
                ),
                "job" => (
                    "bg-amber-500/10 text-amber-400 border-amber-500/20",
                    "ASYNC JOB",
                ),
                _ => (
                    "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
                    "HTTP REQUEST",
                ),
            };

            rows_html.push_str(&format!(
                r#"<tr class="hover:bg-slate-900/50 transition">
                    <td class="px-6 py-4"><span class="px-2.5 py-0.5 rounded text-[10px] font-bold border {badge_class}">{badge_text}</span></td>
                    <td class="px-6 py-4 font-mono text-xs text-slate-200 font-semibold">{name}</td>
                    <td class="px-6 py-4 font-mono text-xs font-bold text-sky-400">{duration} µs</td>
                    <td class="px-6 py-4 font-mono text-xs text-slate-500">{ts}s epoch</td>
                </tr>"#,
                badge_class = badge_class,
                badge_text = badge_text,
                name = rullst_core::html::escape_str(&span.name),
                duration = span.duration_us,
                ts = span.timestamp
            ));
        }
    }

    Html(format!(
        r#"<div class="p-8 font-mono space-y-8 max-w-7xl mx-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>🔍 Distributed Tracing & Flamegraph Inspector</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Live Microsecond Telemetry Spans for HTTP Handlers, SQLx ORM Queries & AI Model Calls</p>
                </div>
                <span class="px-3.5 py-1.5 bg-indigo-950 border border-indigo-800/80 rounded-full text-xs font-bold text-indigo-400 flex items-center gap-2">
                    <span class="h-2 w-2 rounded-full bg-indigo-400 animate-pulse"></span>
                    <span>{spans_count} Spans Captured</span>
                </span>
            </header>

            <div class="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow-md space-y-4 p-6">
                <div class="overflow-x-auto rounded-lg border border-slate-800">
                    <table class="w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-slate-950 border-b border-slate-800 text-slate-400 text-xs uppercase tracking-wider font-bold">
                                <th class="px-6 py-3.5">Kind</th>
                                <th class="px-6 py-3.5">Operation / Target</th>
                                <th class="px-6 py-3.5">Duration</th>
                                <th class="px-6 py-3.5">Timestamp</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-slate-800/80">
                            {rows_html}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>"#,
        spans_count = spans.len(),
        rows_html = rows_html
    ))
}
