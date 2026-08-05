//! Rullst Capital Revenue Dashboard & Webhook Event Inspector (`/studio/tools/revenue`)

use axum::{
    Json, Router,
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_capital::RevenueDashboardManager;
use std::sync::Arc;

static REVENUE_MANAGER: std::sync::OnceLock<Arc<RevenueDashboardManager>> =
    std::sync::OnceLock::new();

fn get_revenue_manager() -> Arc<RevenueDashboardManager> {
    REVENUE_MANAGER
        .get_or_init(|| Arc::new(RevenueDashboardManager::new()))
        .clone()
}

/// Renders the glassmorphic Rullst Capital Revenue Dashboard HTML interface.
pub fn render_revenue_dashboard_page() -> String {
    let mgr = get_revenue_manager();
    let metrics = mgr.get_metrics();
    let events = mgr.get_recent_events(20);

    let mrr_fmt = format!("${:.2}", metrics.mrr_cents as f64 / 100.0);
    let arr_fmt = format!("${:.2}", metrics.arr_cents as f64 / 100.0);
    let net_fmt = format!("${:.2}", metrics.net_revenue_cents as f64 / 100.0);

    let mut event_rows = String::new();
    for evt in events {
        let badge_class = if evt.status == "processed" {
            "background: rgba(34,197,94,0.2); color: #4ade80;"
        } else {
            "background: rgba(239,68,68,0.2); color: #fca5a5;"
        };

        event_rows.push_str(&format!(
            r###"<tr>
                <td style="padding:0.75rem;border-bottom:1px solid #334155;"><code>{}</code></td>
                <td style="padding:0.75rem;border-bottom:1px solid #334155;text-transform:uppercase;font-weight:600;">{}</td>
                <td style="padding:0.75rem;border-bottom:1px solid #334155;"><code>{}</code></td>
                <td style="padding:0.75rem;border-bottom:1px solid #334155;"><span style="padding:0.25rem 0.5rem;border-radius:0.25rem;font-size:0.8rem;{}">{}</span></td>
            </tr>"###,
            evt.id, evt.provider, evt.event_type, badge_class, evt.status
        ));
    }

    format!(
        r###"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rullst Capital — Revenue Dashboard & Webhook Inspector</title>
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
        table {{
            width: 100%;
            border-collapse: collapse;
            background: rgba(30, 41, 59, 0.7);
            border-radius: 0.75rem;
            overflow: hidden;
        }}
        th {{
            background: #1e293b;
            padding: 1rem;
            text-align: left;
            font-size: 0.85rem;
            color: #94a3b8;
            text-transform: uppercase;
        }}
    </style>
</head>
<body>
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:2rem;">
        <div>
            <h1 style="margin:0;color:#38bdf8;">💳 Rullst Capital Dashboard</h1>
            <p style="margin:0.25rem 0 0 0;color:#94a3b8;">Real-time SaaS MRR/ARR analytics & Stripe/LemonSqueezy Webhook Inspector</p>
        </div>
    </div>

    <div class="grid">
        <div class="card">
            <div class="card-label">Monthly Recurring (MRR)</div>
            <div class="card-value">{mrr}</div>
        </div>
        <div class="card">
            <div class="card-label">Annual Recurring (ARR)</div>
            <div class="card-value" style="color:#4ade80;">{arr}</div>
        </div>
        <div class="card">
            <div class="card-label">Net Revenue</div>
            <div class="card-value" style="color:#a78bfa;">{net}</div>
        </div>
        <div class="card">
            <div class="card-label">Active Subscribers</div>
            <div class="card-value" style="color:#facc15;">{subs}</div>
        </div>
        <div class="card">
            <div class="card-label">Churn Rate</div>
            <div class="card-value" style="color:#f87171;">{churn:.1}%</div>
        </div>
    </div>

    <div class="card" style="padding:1rem;">
        <h2 style="margin:0 0 1rem 0;font-size:1.2rem;color:#f8fafc;">📡 Live Webhook Audit Log Inspector</h2>
        <table>
            <thead>
                <tr>
                    <th>Event ID</th>
                    <th>Provider</th>
                    <th>Event Type</th>
                    <th>Status</th>
                </tr>
            </thead>
            <tbody>
                {rows}
            </tbody>
        </table>
    </div>
</body>
</html>"###,
        mrr = mrr_fmt,
        arr = arr_fmt,
        net = net_fmt,
        subs = metrics.active_subscriptions,
        churn = metrics.churn_rate_percent,
        rows = event_rows
    )
}

/// Router endpoint handler for API revenue metrics JSON.
pub async fn api_revenue_handler() -> impl IntoResponse {
    let mgr = get_revenue_manager();
    Json(mgr.get_metrics())
}

/// Returns an Axum `Router` mounting the Rullst Capital Revenue Dashboard endpoints.
pub fn router() -> Router {
    Router::new()
        .route(
            "/studio/tools/revenue",
            get(|| async { Html(render_revenue_dashboard_page()) }),
        )
        .route("/api/revenue", get(api_revenue_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_revenue_dashboard_endpoint() {
        let app = router();

        let req = Request::builder()
            .uri("/studio/tools/revenue")
            .body(Body::empty())
            .unwrap();
        let page_resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(page_resp.status(), StatusCode::OK);

        let api_req = Request::builder()
            .uri("/api/revenue")
            .body(Body::empty())
            .unwrap();
        let api_resp = app.oneshot(api_req).await.unwrap();
        assert_eq!(api_resp.status(), StatusCode::OK);
    }
}
