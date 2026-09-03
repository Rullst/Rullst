use axum::response::Html;
use std::fmt::Write;

/// Renders local spans with an empty distributed store.
pub async fn render_traces_page() -> Html<String> {
    render_traces_page_with_store(&crate::distributed_traces::DistributedTraceStore::default())
        .await
}

/// Renders local spans and authenticated distributed records supplied through
/// an explicitly shared bounded store.
pub async fn render_traces_page_with_store(
    distributed_store: &crate::distributed_traces::DistributedTraceStore,
) -> Html<String> {
    let collector = rullst_core::telemetry_spans::global_span_collector();
    let spans = collector.snapshot();
    let distributed = distributed_store.snapshot();
    let findings = distributed_store.query_findings();

    let mut rows_html = String::new();
    if spans.is_empty() {
        rows_html.push_str(
            r#"<tr>
                <td colspan="4" class="px-6 py-12 text-center text-sm text-slate-500 font-medium bg-slate-950/40">
                    No local spans have been recorded yet. Application or framework code must record TraceSpan values explicitly.
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

    let distributed_html = render_distributed_rows(distributed.as_deref());
    let findings_html = render_query_findings(findings.as_deref());
    let distributed_count = distributed.as_ref().map_or(0, |records| records.len());

    Html(format!(
        r#"<div class="p-8 font-mono space-y-8 max-w-7xl mx-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>🔍 Trace Inspector</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Local observations plus bounded HMAC-authenticated, attribute-free distributed spans</p>
                </div>
                <span class="px-3.5 py-1.5 bg-indigo-950 border border-indigo-800/80 rounded-full text-xs font-bold text-indigo-400 flex items-center gap-2">
                    <span class="h-2 w-2 rounded-full bg-indigo-400 animate-pulse"></span>
                    <span>{spans_count} Spans Captured</span>
                </span>
            </header>

            <div class="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow-md space-y-4 p-6">
                <h2 class="text-sm font-bold text-slate-200">Local process spans</h2>
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

            <div class="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow-md space-y-4 p-6">
                <div class="flex items-center justify-between gap-4">
                    <div>
                        <h2 class="text-sm font-bold text-slate-200">Authenticated distributed spans</h2>
                        <p class="text-xs text-slate-500 mt-1">Push-only v1 records; no SQL text, bindings, attributes, headers, or bodies are accepted.</p>
                    </div>
                    <span class="text-xs font-bold text-indigo-400">{distributed_count} retained</span>
                </div>
                <div class="overflow-x-auto rounded-lg border border-slate-800">
                    <table class="w-full text-left border-collapse">
                        <thead><tr class="bg-slate-950 border-b border-slate-800 text-slate-400 text-xs uppercase tracking-wider font-bold">
                            <th class="px-4 py-3">Source</th><th class="px-4 py-3">Trace</th><th class="px-4 py-3">Kind</th><th class="px-4 py-3">Operation</th><th class="px-4 py-3">Duration</th><th class="px-4 py-3">Status</th>
                        </tr></thead>
                        <tbody class="divide-y divide-slate-800/80">{distributed_html}</tbody>
                    </table>
                </div>
            </div>

            <div class="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow-md space-y-4 p-6">
                <div>
                    <h2 class="text-sm font-bold text-slate-200">SQL profiler findings</h2>
                    <p class="text-xs text-slate-500 mt-1">Heuristics flag ≥3 equal labels in one trace or spans ≥100 ms; findings are evidence to inspect, not proof of N+1.</p>
                </div>
                <div class="space-y-2">{findings_html}</div>
            </div>
        </div>"#,
        spans_count = spans.len(),
        rows_html = rows_html,
        distributed_count = distributed_count,
        distributed_html = distributed_html,
        findings_html = findings_html,
    ))
}

fn render_distributed_rows(
    records: Result<
        &[crate::distributed_traces::StoredDistributedTraceSpan],
        &crate::distributed_traces::TraceIngestionError,
    >,
) -> String {
    let Ok(records) = records else {
        return table_message(6, "Distributed trace store unavailable");
    };
    if records.is_empty() {
        return table_message(6, "No authenticated distributed spans have been ingested");
    }
    let mut html = String::new();
    for record in records.iter().rev() {
        let status_class = match record.span.status {
            crate::distributed_traces::DistributedTraceStatus::Ok => "text-emerald-400",
            crate::distributed_traces::DistributedTraceStatus::Error => "text-rose-400",
            crate::distributed_traces::DistributedTraceStatus::Unset => "text-slate-400",
        };
        let _ = write!(
            html,
            r#"<tr class="hover:bg-slate-900/50"><td class="px-4 py-3 text-xs text-sky-400">{}</td><td class="px-4 py-3 font-mono text-[10px] text-slate-500">{}</td><td class="px-4 py-3 text-xs text-slate-400">{:?}</td><td class="px-4 py-3 text-xs text-slate-200">{}</td><td class="px-4 py-3 text-xs text-indigo-400">{} µs</td><td class="px-4 py-3 text-xs font-bold {}">{:?}</td></tr>"#,
            rullst_core::html::escape_str(&record.source),
            rullst_core::html::escape_str(&record.span.trace_id),
            record.span.kind,
            rullst_core::html::escape_str(&record.span.operation),
            record.span.duration_us,
            status_class,
            record.span.status,
        );
    }
    html
}

fn render_query_findings(
    findings: Result<
        &[crate::distributed_traces::QueryFinding],
        &crate::distributed_traces::TraceIngestionError,
    >,
) -> String {
    let Ok(findings) = findings else {
        return "<p class=\"text-xs text-amber-400\">Query profiler unavailable</p>".to_string();
    };
    if findings.is_empty() {
        return "<p class=\"text-xs text-slate-500\">No SQL heuristic findings in retained distributed spans.</p>".to_string();
    }
    let mut html = String::new();
    for finding in findings {
        let label = match finding.kind {
            crate::distributed_traces::QueryFindingKind::RepeatedOperation => "Possible N+1",
            crate::distributed_traces::QueryFindingKind::SlowOperation => "Slow operation",
        };
        let _ = write!(
            html,
            r#"<div class="p-3 rounded-lg border border-amber-800/50 bg-amber-950/20 text-xs"><span class="font-bold text-amber-400">{label}</span><span class="text-slate-300"> · {} · {} · {} occurrence(s) · max {} µs</span></div>"#,
            rullst_core::html::escape_str(&finding.source),
            rullst_core::html::escape_str(&finding.operation),
            finding.occurrences,
            finding.maximum_duration_us,
        );
    }
    html
}

fn table_message(columns: usize, message: &str) -> String {
    format!(
        "<tr><td colspan=\"{columns}\" class=\"px-6 py-10 text-center text-xs text-slate-500\">{}</td></tr>",
        rullst_core::html::escape_str(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_render_traces_page() {
        let collector = rullst_core::telemetry_spans::global_span_collector();

        // Record spans of various kinds
        collector.record(rullst_core::telemetry_spans::TraceSpan {
            name: "SELECT * FROM users WHERE id = 1".to_string(),
            kind: "sql".to_string(),
            duration_us: 120,
            timestamp: 1700000000,
        });
        collector.record(rullst_core::telemetry_spans::TraceSpan {
            name: "gemini-2.5-flash generate".to_string(),
            kind: "ai".to_string(),
            duration_us: 850,
            timestamp: 1700000000,
        });
        collector.record(rullst_core::telemetry_spans::TraceSpan {
            name: "send_welcome_email".to_string(),
            kind: "job".to_string(),
            duration_us: 450,
            timestamp: 1700000000,
        });
        collector.record(rullst_core::telemetry_spans::TraceSpan {
            name: "GET /api/v1/health".to_string(),
            kind: "http".to_string(),
            duration_us: 95,
            timestamp: 1700000000,
        });

        let html = render_traces_page().await.0;
        assert!(html.contains("Trace Inspector"));
        assert!(html.contains("SQL QUERY"));
        assert!(html.contains("AI GENERATION"));
        assert!(html.contains("ASYNC JOB"));
        assert!(html.contains("HTTP REQUEST"));
        assert!(html.contains("SELECT * FROM users"));
    }

    #[tokio::test]
    async fn distributed_markup_is_escaped_and_query_findings_are_explicitly_heuristic() {
        let store = crate::distributed_traces::DistributedTraceStore::new(8)
            .expect("distributed trace store");
        let now = 1_800_000_000;
        let spans = (1..=3)
            .map(|suffix| crate::distributed_traces::DistributedTraceSpanV1 {
                trace_id: "0123456789abcdef0123456789abcdef".to_string(),
                span_id: format!("0123456789abcde{suffix}"),
                parent_span_id: None,
                operation: "users.<script>".to_string(),
                kind: crate::distributed_traces::DistributedTraceKind::Sql,
                started_at_unix_us: now * 1_000_000,
                duration_us: 5,
                status: crate::distributed_traces::DistributedTraceStatus::Ok,
            })
            .collect();
        store
            .insert_batch("api-1", now, spans)
            .expect("trace fixture");

        let html = render_traces_page_with_store(&store).await.0;
        assert!(html.contains("Possible N+1"));
        assert!(html.contains("findings are evidence to inspect, not proof of N+1"));
        assert!(html.contains("users.&lt;script&gt;"));
        assert!(!html.contains("users.<script>"));
    }
}
