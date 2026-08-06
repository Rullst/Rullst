//! Rullst Radar Kernel-Level Telemetry & Spans Visualizer (`/studio/radar`)

use axum::{
    Json, Router,
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_core::radar::RadarSnapshot;

/// Renders the unified glassmorphic Rullst Radar & Telemetry Dashboard HTML fragment.
pub fn render_radar_page() -> String {
    let snapshot = RadarSnapshot::collect();
    let collector = rullst_core::telemetry_spans::global_span_collector();
    let spans = collector.snapshot();

    let mut span_rows_html = String::new();
    if spans.is_empty() {
        span_rows_html.push_str(
            r#"<div class="p-6 text-center text-sm text-slate-500 font-medium bg-slate-950/40 rounded-xl border border-slate-800/60">
                No active telemetry spans recorded yet. Send HTTP requests or execute ORM queries to stream live microsecond spans.
            </div>"#,
        );
    } else {
        span_rows_html.push_str(r#"<div class="space-y-2 font-mono text-xs">"#);
        for span in spans.iter().rev().take(15) {
            let (badge_color, badge_text) = match span.kind.as_str() {
                "sql" => ("bg-blue-500/10 text-blue-400 border-blue-500/20", "SQL QUERY"),
                "ai" => ("bg-purple-500/10 text-purple-400 border-purple-500/20", "AI GENERATION"),
                "job" => ("bg-amber-500/10 text-amber-400 border-amber-500/20", "ASYNC JOB"),
                "security" => ("bg-rose-500/10 text-rose-400 border-rose-500/20", "SECURITY WAF"),
                _ => ("bg-emerald-500/10 text-emerald-400 border-emerald-500/20", "HTTP REQUEST"),
            };

            span_rows_html.push_str(&format!(
                r#"<div class="p-3 bg-slate-950 border border-slate-800/80 rounded-xl flex items-center justify-between hover:border-slate-700 transition">
                    <div class="flex items-center gap-3">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold border {badge_color}">{badge_text}</span>
                        <span class="text-slate-200 font-semibold">{name}</span>
                    </div>
                    <div class="flex items-center gap-4">
                        <span class="text-emerald-400 font-bold">{duration} µs</span>
                        <span class="text-slate-500 text-[11px]">{ts}s epoch</span>
                    </div>
                </div>"#,
                badge_color = badge_color,
                badge_text = badge_text,
                name = rullst_core::html::escape_str(&span.name),
                duration = span.duration_us,
                ts = span.timestamp
            ));
        }
        span_rows_html.push_str("</div>");
    }

    format!(
        r#"<div class="p-8 font-mono space-y-8 max-w-7xl mx-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>📡 Telemetry & Rullst Radar</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Real-time Tokio Runtime Kernel Telemetry, Memory RSS & Microsecond Async Spans</p>
                </div>
                <div class="flex items-center gap-3">
                    <a href="/metrics" target="_blank" class="px-3.5 py-1.5 bg-sky-950 border border-sky-800/80 hover:border-sky-500 rounded-full text-xs font-bold text-sky-400 transition flex items-center gap-1.5 shadow-inner">
                        <span>📊 Prometheus /metrics</span>
                        <span class="text-slate-500">↗</span>
                    </a>
                    <span class="px-3.5 py-1.5 bg-slate-900 border border-slate-800 rounded-full text-xs font-semibold text-slate-300 flex items-center gap-2">
                        <span class="h-2 w-2 rounded-full bg-emerald-400 animate-pulse"></span>
                        <span>Zero-Overhead Probe</span>
                    </span>
                </div>
            </header>

            <!-- Real-time Kernel KPI Cards -->
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Tokio Tick Latency</div>
                    <div class="text-2xl font-bold text-emerald-400 mt-1">{latency} µs</div>
                    <div class="text-xs text-slate-400 mt-2">Zero-cost async event loop</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Active Async Tasks</div>
                    <div class="text-2xl font-bold text-sky-400 mt-1">{tasks}</div>
                    <div class="text-xs text-slate-400 mt-2">Running tokio tasks</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Memory RSS RAM</div>
                    <div class="text-2xl font-bold text-indigo-400 mt-1">{rss:.1} MB</div>
                    <div class="text-xs text-slate-400 mt-2">Process RAM memory</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Process CPU</div>
                    <div class="text-2xl font-bold text-amber-400 mt-1">{cpu:.1}%</div>
                    <div class="text-xs text-slate-400 mt-2">Active CPU core load</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">System Uptime</div>
                    <div class="text-2xl font-bold text-cyan-400 mt-1">{uptime}s</div>
                    <div class="text-xs text-slate-400 mt-2">Total process runtime</div>
                </div>
            </div>

            <!-- Real Telemetry Spans Section -->
            <div class="bg-slate-900/90 border border-slate-800 rounded-xl p-6 shadow-md space-y-4">
                <div class="flex items-center justify-between pb-3 border-b border-slate-800/80">
                    <div>
                        <h2 class="text-lg font-bold text-slate-200 flex items-center gap-2">
                            <span>⚡ Live Async Telemetry Spans</span>
                        </h2>
                        <p class="text-xs text-slate-400 mt-0.5">Captured microsecond spans across HTTP handlers, SQL queries, AI generations & security filters.</p>
                    </div>
                    <span class="text-xs font-semibold text-slate-400 bg-slate-950 px-2.5 py-1 rounded-lg border border-slate-800">
                        {spans_count} Recorded Spans
                    </span>
                </div>
                {span_rows_html}
            </div>
        </div>"#,
        latency = snapshot.tokio_latency_micros,
        tasks = snapshot.active_tokio_tasks,
        rss = snapshot.memory_rss_mb,
        cpu = snapshot.cpu_usage_percent,
        uptime = snapshot.uptime_seconds,
        spans_count = spans.len(),
        span_rows_html = span_rows_html
    )
}

/// Endpoint handler for JSON Radar telemetry API (`GET /api/radar`).
pub async fn api_radar_handler() -> impl IntoResponse {
    Json(RadarSnapshot::collect())
}

/// Returns an Axum `Router` mounting the Rullst Radar Telemetry endpoints.
pub fn router() -> Router {
    Router::new()
        .route(
            "/studio/radar",
            get(|| async { Html(render_radar_page()) }),
        )
        .route(
            "/studio/tools/radar",
            get(|| async { Html(render_radar_page()) }),
        )
        .route("/api/radar", get(api_radar_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_radar_visualizer_endpoint() {
        let app = router();

        let req = Request::builder()
            .uri("/studio/radar")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let api_req = Request::builder()
            .uri("/api/radar")
            .body(Body::empty())
            .unwrap();
        let api_resp = app.oneshot(api_req).await.unwrap();
        assert_eq!(api_resp.status(), StatusCode::OK);
    }
}
