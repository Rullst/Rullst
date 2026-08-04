use axum::response::Html;

/// Renders the Distributed Tracing Flamegraph Inspector page for Rullst Studio (/studio/tools/traces).
pub async fn render_traces_page() -> Html<String> {
    let collector = rullst_core::telemetry_spans::global_span_collector();
    let spans = collector.snapshot();

    let mut rows_html = String::new();
    if spans.is_empty() {
        rows_html.push_str(
            "<tr><td colspan=\"4\" style=\"padding:16px; text-align:center; color:#94a3b8;\">\
             No trace spans recorded yet. Send HTTP requests or run ORM queries to populate trace data.\
             </td></tr>",
        );
    } else {
        for span in spans.iter().rev() {
            let badge_color = match span.kind.as_str() {
                "sql" => "#3b82f6",
                "ai" => "#a855f7",
                "job" => "#f59e0b",
                _ => "#10b981",
            };
            rows_html.push_str(&format!(
                "<tr style=\"border-bottom:1px solid #334155;\">\
                 <td style=\"padding:10px;\"><span style=\"background:{badge_color}; color:#fff; font-size:10px; font-weight:bold; padding:2px 6px; border-radius:4px;\">{}</span></td>\
                 <td style=\"padding:10px; font-weight:600; color:#f8fafc;\">{}</td>\
                 <td style=\"padding:10px; font-family:monospace; color:#38bdf8;\">{} µs</td>\
                 <td style=\"padding:10px; color:#94a3b8; font-size:12px;\">{}s epoch</td>\
                 </tr>",
                span.kind.to_uppercase(),
                span.name,
                span.duration_us,
                span.timestamp
            ));
        }
    }

    Html(format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8"/>
    <title>Rullst Studio — Distributed Tracing Visualizer</title>
    <style>
        body {{ background: #0f172a; color: #f8fafc; font-family: system-ui, sans-serif; margin: 0; padding: 24px; }}
        .card {{ background: #1e293b; border-radius: 12px; border: 1px solid #334155; padding: 24px; max-width: 1000px; margin: 0 auto; }}
        h1 {{ margin-top: 0; color: #38bdf8; display: flex; align-items: center; gap: 10px; font-size: 22px; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 16px; font-size: 14px; }}
        th {{ text-align: left; padding: 10px; border-bottom: 2px solid #334155; color: #94a3b8; font-size: 12px; text-transform: uppercase; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>🔍 Distributed Tracing Flamegraph Inspector</h1>
        <p style="color:#94a3b8; font-size:14px; margin-top: 4px;">Live microsecond telemetry spans for HTTP, SQLx ORM, and AI prompts</p>
        
        <table>
            <thead>
                <tr>
                    <th>Kind</th>
                    <th>Operation / Target</th>
                    <th>Duration</th>
                    <th>Timestamp</th>
                </tr>
            </thead>
            <tbody>
                {rows_html}
            </tbody>
        </table>
    </div>
</body>
</html>
"#
    ))
}
