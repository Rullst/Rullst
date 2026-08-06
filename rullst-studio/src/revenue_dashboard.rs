//! Rullst Capital Revenue Dashboard & Webhook Event Inspector (`/studio/capital`)

use axum::{
    Json, Router,
    response::{Html, IntoResponse},
    routing::get,
};
use rullst_capital::RevenueDashboardManager;
use std::sync::Arc;

static REVENUE_MANAGER: std::sync::OnceLock<Arc<RevenueDashboardManager>> =
    std::sync::OnceLock::new();

pub fn get_revenue_manager() -> Arc<RevenueDashboardManager> {
    REVENUE_MANAGER
        .get_or_init(|| Arc::new(RevenueDashboardManager::new()))
        .clone()
}

/// Renders the glassmorphic Rullst Capital Revenue Dashboard HTML interface fragment.
pub fn render_revenue_dashboard_page() -> String {
    let mgr = get_revenue_manager();
    let metrics = mgr.get_metrics();
    let events = mgr.get_recent_events(20);

    let mrr_fmt = format!("${:.2}", metrics.mrr_cents as f64 / 100.0);
    let arr_fmt = format!("${:.2}", metrics.arr_cents as f64 / 100.0);
    let net_fmt = format!("${:.2}", metrics.net_revenue_cents as f64 / 100.0);

    let mut event_rows = String::new();
    if events.is_empty() {
        event_rows.push_str(
            r#"<tr>
                <td colspan="4" class="px-6 py-12 text-center text-sm text-slate-500 font-medium bg-slate-950/40">
                    No payment webhook events recorded yet. Webhooks from Stripe or LemonSqueezy will automatically appear here in real-time.
                </td>
            </tr>"#,
        );
    } else {
        for evt in &events {
            let (badge_class, badge_text) = if evt.status == "processed" {
                (
                    "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
                    "PROCESSED",
                )
            } else {
                ("bg-rose-500/10 text-rose-400 border-rose-500/20", "FAILED")
            };

            event_rows.push_str(&format!(
                r#"<tr class="hover:bg-slate-900/50 transition">
                    <td class="px-6 py-4 font-mono text-xs text-slate-300 font-semibold">{id}</td>
                    <td class="px-6 py-4 text-xs font-bold text-slate-400 uppercase">{provider}</td>
                    <td class="px-6 py-4 font-mono text-xs text-sky-400">{event_type}</td>
                    <td class="px-6 py-4"><span class="px-2.5 py-0.5 rounded text-[10px] font-bold border {badge_class}">{badge_text}</span></td>
                </tr>"#,
                id = rullst_core::html::escape_str(&evt.id),
                provider = rullst_core::html::escape_str(&evt.provider),
                event_type = rullst_core::html::escape_str(&evt.event_type),
                badge_class = badge_class,
                badge_text = badge_text
            ));
        }
    }

    format!(
        r#"<div class="p-8 font-mono space-y-8 max-w-7xl mx-auto">
            <header class="pb-6 border-b border-slate-800 flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>💳 Rullst Capital Dashboard</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Real-time SaaS MRR/ARR Revenue Analytics & Live Payment Webhook Inspector</p>
                </div>
                <span class="px-3.5 py-1.5 bg-emerald-950 border border-emerald-800/80 rounded-full text-xs font-bold text-emerald-400 flex items-center gap-2">
                    <span class="h-2 w-2 rounded-full bg-emerald-400 animate-pulse"></span>
                    <span>Stripe & LemonSqueezy Ready</span>
                </span>
            </header>

            <!-- Revenue KPI Metric Grid -->
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Monthly Recurring (MRR)</div>
                    <div class="text-2xl font-bold text-sky-400 mt-1">{mrr}</div>
                    <div class="text-xs text-slate-400 mt-2">Active recurring revenue</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Annual Recurring (ARR)</div>
                    <div class="text-2xl font-bold text-emerald-400 mt-1">{arr}</div>
                    <div class="text-xs text-slate-400 mt-2">Annualized run-rate</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Net Revenue</div>
                    <div class="text-2xl font-bold text-indigo-400 mt-1">{net}</div>
                    <div class="text-xs text-slate-400 mt-2">Net after provider fees</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Active Subscribers</div>
                    <div class="text-2xl font-bold text-amber-400 mt-1">{subs}</div>
                    <div class="text-xs text-slate-400 mt-2">Total paying customers</div>
                </div>
                <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                    <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Churn Rate</div>
                    <div class="text-2xl font-bold text-purple-400 mt-1">{churn:.1}%</div>
                    <div class="text-xs text-slate-400 mt-2">Estimated monthly churn</div>
                </div>
            </div>

            <!-- Webhook Audit Log Table -->
            <div class="bg-slate-900/90 border border-slate-800 rounded-xl overflow-hidden shadow-md space-y-4 p-6">
                <div class="flex items-center justify-between pb-3 border-b border-slate-800/80">
                    <h2 class="text-lg font-bold text-slate-200 flex items-center gap-2">
                        <span>📡 Live Webhook Audit Log Inspector</span>
                    </h2>
                    <span class="text-xs font-semibold text-slate-400 bg-slate-950 px-2.5 py-1 rounded-lg border border-slate-800">
                        {events_count} Webhook Events Recorded
                    </span>
                </div>
                <div class="overflow-x-auto rounded-lg border border-slate-800">
                    <table class="w-full text-left border-collapse">
                        <thead>
                            <tr class="bg-slate-950 border-b border-slate-800 text-slate-400 text-xs uppercase tracking-wider font-bold">
                                <th class="px-6 py-3.5">Event ID</th>
                                <th class="px-6 py-3.5">Provider</th>
                                <th class="px-6 py-3.5">Event Type</th>
                                <th class="px-6 py-3.5">Status</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-slate-800/80">
                            {rows}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>"#,
        mrr = mrr_fmt,
        arr = arr_fmt,
        net = net_fmt,
        subs = metrics.active_subscriptions,
        churn = metrics.churn_rate_percent,
        events_count = events.len(),
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
            "/studio/capital",
            get(|| async { Html(render_revenue_dashboard_page()) }),
        )
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
            .uri("/studio/capital")
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
