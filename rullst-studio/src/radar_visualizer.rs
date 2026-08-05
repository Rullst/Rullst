//! Rullst Radar Kernel-Level Telemetry Visualizer (`/studio/tools/radar`)

use axum::{
    Json, Router,
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_core::radar::RadarSnapshot;

/// Renders the glassmorphic Rullst Radar Telemetry Dashboard HTML interface.
pub fn render_radar_page() -> String {
    let snapshot = RadarSnapshot::collect();

    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rullst Radar — Kernel Telemetry & Tokio Runtime Inspector</title>
    <style>
        body {{
            margin: 0;
            padding: 2rem;
            background: #0f172a;
            color: #f8fafc;
            font-family: system-ui, -apple-system, sans-serif;
        }}
        .grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }}
        .card {{
            background: rgba(30, 41, 59, 0.7);
            border: 1px solid #334155;
            border-radius: 0.75rem;
            padding: 1.5rem;
            backdrop-filter: blur(12px);
        }}
        .card-label {{
            font-size: 0.875rem;
            color: #94a3b8;
            margin-bottom: 0.5rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .card-value {{
            font-size: 2rem;
            font-weight: 700;
            color: #38bdf8;
        }}
    </style>
</head>
<body>
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:2rem;">
        <div>
            <h1 style="margin:0;color:#38bdf8;">📡 Rullst Radar Telemetry</h1>
            <p style="margin:0.25rem 0 0 0;color:#94a3b8;">Kernel-Level Metrics, Tokio Runtime Latency & Memory RSS Monitoring</p>
        </div>
        <div>
            <a href="/metrics" target="_blank" style="background:#0284c7;color:white;padding:0.5rem 1rem;border-radius:0.5rem;text-decoration:none;font-weight:600;">Prometheus /metrics</a>
        </div>
    </div>

    <div class="grid">
        <div class="card">
            <div class="card-label">Tokio Tick Latency</div>
            <div class="card-value" style="color:#4ade80;">{latency} µs</div>
        </div>
        <div class="card">
            <div class="card-label">Active Async Tasks</div>
            <div class="card-value">{tasks}</div>
        </div>
        <div class="card">
            <div class="card-label">Memory RSS Ocupation</div>
            <div class="card-value" style="color:#a78bfa;">{rss:.1} MB</div>
        </div>
        <div class="card">
            <div class="card-label">Process CPU Utilization</div>
            <div class="card-value" style="color:#facc15;">{cpu:.1}%</div>
        </div>
        <div class="card">
            <div class="card-label">System Uptime</div>
            <div class="card-value" style="color:#38bdf8;">{uptime}s</div>
        </div>
    </div>
</body>
</html>"###,
        latency = snapshot.tokio_latency_micros,
        tasks = snapshot.active_tokio_tasks,
        rss = snapshot.memory_rss_mb,
        cpu = snapshot.cpu_usage_percent,
        uptime = snapshot.uptime_seconds
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
            .uri("/studio/tools/radar")
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
