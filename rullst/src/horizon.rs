use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use crate::queue::Queue;

/// Horizon Dashboard App State
struct HorizonState {
    queue: Queue,
}

/// Create a pre-configured Axum Router for the Horizon dashboard
pub fn router(queue: Queue) -> Router {
    let state = Arc::new(HorizonState { queue });

    Router::new()
        .route("/", get(dashboard_home))
        .route("/jobs-table", get(jobs_table))
        .route("/retry/:id", post(retry_job))
        .route("/purge", post(purge_failed_jobs))
        .with_state(state)
}

// ─── Route Handlers ─────────────────────────────────────────────────────────

async fn dashboard_home(State(state): State<Arc<HorizonState>>) -> impl IntoResponse {
    let jobs = state.queue.list_all_jobs(50).await.unwrap_or_default();
    let pending = state.queue.pending_count().await.unwrap_or(0);
    
    let failed = jobs.iter().filter(|j| j.status == "failed").count();
    let _completed = jobs.iter().filter(|j| j.status == "completed").count(); // completed are deleted, but let's count completed in the session list if any
    let processing = jobs.iter().filter(|j| j.status == "processing").count();
    
    let html_content = render_dashboard_layout(pending, failed, processing, render_table_rows(&jobs));
    Html(html_content)
}

async fn jobs_table(State(state): State<Arc<HorizonState>>) -> impl IntoResponse {
    let jobs = state.queue.list_all_jobs(50).await.unwrap_or_default();
    Html(render_table_rows(&jobs))
}

async fn retry_job(
    State(state): State<Arc<HorizonState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let _ = state.queue.retry_failed_job(&id).await;
    
    // Return HTMX trigger header to refresh the table and metrics
    let headers = [
        ("HX-Trigger", "refresh-jobs"),
    ];
    (headers, Html(r#"<span class="text-xs text-teal-400 font-semibold">Retrying...</span>"#))
}

async fn purge_failed_jobs(State(state): State<Arc<HorizonState>>) -> impl IntoResponse {
    let _ = state.queue.purge_completed_jobs().await;
    
    let headers = [
        ("HX-Trigger", "refresh-jobs"),
    ];
    (headers, Html(r#"<span class="text-xs text-rose-400 font-semibold">Purged Failed Jobs</span>"#))
}

// ─── UI Rendering Helpers ───────────────────────────────────────────────────

fn render_dashboard_layout(pending: u64, failed: usize, processing: usize, table_rows: String) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Rullst Horizon 🪐</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@1.9.12"></script>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;800&family=JetBrains+Mono:wght@400;700&display=swap');
        body {{
            font-family: 'Outfit', sans-serif;
        }}
        .font-mono {{
            font-family: 'JetBrains Mono', monospace;
        }}
        .glass {{
            background: rgba(15, 23, 42, 0.45);
            backdrop-filter: blur(12px);
            -webkit-backdrop-filter: blur(12px);
            border: 1px solid rgba(255, 255, 255, 0.05);
        }}
        .glow-teal {{
            box-shadow: 0 0 25px -5px rgba(20, 184, 166, 0.2);
        }}
        .glow-rose {{
            box-shadow: 0 0 25px -5px rgba(244, 63, 94, 0.2);
        }}
        .animate-pulse-slow {{
            animation: pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: .65; }}
        }}
    </style>
</head>
<body class="min-h-full flex flex-col bg-slate-950 bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(120,119,198,0.15),rgba(255,255,255,0))]">

    <!-- Navbar -->
    <header class="border-b border-slate-900 bg-slate-950/80 backdrop-blur sticky top-0 z-50">
        <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
            <div class="flex items-center gap-3">
                <span class="text-2xl">🪐</span>
                <div>
                    <h1 class="text-xl font-bold bg-gradient-to-r from-teal-400 to-indigo-400 bg-clip-text text-transparent">Rullst Horizon</h1>
                    <p class="text-xs text-slate-500 font-medium">Background Queue Dashboard</p>
                </div>
            </div>
            <div class="flex items-center gap-4">
                <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-teal-400/10 text-teal-400 animate-pulse-slow">
                    <span class="h-1.5 w-1.5 rounded-full bg-teal-400"></span>
                    Live Connected
                </span>
            </div>
        </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
        
        <!-- Metrics Row -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
            
            <!-- Pending Card -->
            <div class="glass glow-teal rounded-2xl p-6 transition-all duration-300 hover:scale-[1.01]">
                <div class="flex justify-between items-start">
                    <div>
                        <p class="text-sm font-semibold text-slate-400 uppercase tracking-wider">Pending Jobs</p>
                        <h3 class="text-4xl font-extrabold text-teal-400 mt-2 font-mono" id="metrics-pending">{pending}</h3>
                    </div>
                    <span class="p-3 bg-teal-500/10 text-teal-400 rounded-xl">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                    </span>
                </div>
                <div class="mt-4 text-xs text-slate-500">
                    Active worker threads polling continuously.
                </div>
            </div>

            <!-- Processing Card -->
            <div class="glass rounded-2xl p-6 transition-all duration-300 hover:scale-[1.01]">
                <div class="flex justify-between items-start">
                    <div>
                        <p class="text-sm font-semibold text-slate-400 uppercase tracking-wider">Active Workers</p>
                        <h3 class="text-4xl font-extrabold text-indigo-400 mt-2 font-mono">{processing}</h3>
                    </div>
                    <span class="p-3 bg-indigo-500/10 text-indigo-400 rounded-xl">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                        </svg>
                    </span>
                </div>
                <div class="mt-4 text-xs text-slate-500">
                    Executing registered closure callbacks.
                </div>
            </div>

            <!-- Failed Card -->
            <div class="glass glow-rose rounded-2xl p-6 transition-all duration-300 hover:scale-[1.01]">
                <div class="flex justify-between items-start">
                    <div>
                        <p class="text-sm font-semibold text-slate-400 uppercase tracking-wider">Failed Jobs</p>
                        <h3 class="text-4xl font-extrabold text-rose-400 mt-2 font-mono" id="metrics-failed">{failed}</h3>
                    </div>
                    <span class="p-3 bg-rose-500/10 text-rose-400 rounded-xl">
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                        </svg>
                    </span>
                </div>
                <div class="mt-4 flex gap-3">
                    <button hx-post="/horizon/purge" 
                            hx-target="#jobs-table-body"
                            class="text-xs text-rose-400 hover:text-rose-300 font-semibold transition-colors duration-200">
                        Purge Failed
                    </button>
                </div>
            </div>

        </div>

        <!-- Jobs Listing Section -->
        <div class="glass rounded-2xl border border-slate-900 overflow-hidden">
            <div class="px-6 py-5 border-b border-slate-900 flex justify-between items-center bg-slate-950/45">
                <div>
                    <h3 class="text-lg font-bold text-slate-200">Recent Queue Jobs</h3>
                    <p class="text-xs text-slate-500 mt-0.5">Real-time listing of background execution pipelines</p>
                </div>
                <button hx-get="/horizon/jobs-table"
                        hx-target="#jobs-table-body"
                        hx-trigger="click, refresh-jobs from:body"
                        class="px-4 py-2 rounded-lg bg-slate-900 border border-slate-800 text-xs font-semibold hover:bg-slate-850 hover:text-teal-400 transition-all duration-200">
                    Refresh List
                </button>
            </div>
            
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-slate-900 text-sm">
                    <thead class="bg-slate-950/20 text-slate-400 font-semibold uppercase tracking-wider text-xs">
                        <tr>
                            <th class="px-6 py-4 text-left">Job ID / Type</th>
                            <th class="px-6 py-4 text-left">Payload Parameters</th>
                            <th class="px-6 py-4 text-left">Status</th>
                            <th class="px-6 py-4 text-left">Attempts</th>
                            <th class="px-6 py-4 text-left">Date / Logs</th>
                            <th class="px-6 py-4 text-right">Actions</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-900" id="jobs-table-body" hx-get="/horizon/jobs-table" hx-trigger="every 5s">
                        {table_rows}
                    </tbody>
                </table>
            </div>
        </div>

    </main>

    <!-- Footer -->
    <footer class="border-t border-slate-900 bg-slate-950/40 text-center py-6 mt-12 text-xs text-slate-650">
        Rullst Horizon v0.9.0 &bull; Built in Rust for maximum factory productivity.
    </footer>

</body>
</html>"##,
        pending = pending,
        failed = failed,
        processing = processing,
        table_rows = table_rows
    )
}

fn render_table_rows(jobs: &[crate::queue::QueuedJobDetail]) -> String {
    if jobs.is_empty() {
        return r#"<tr>
            <td colspan="6" class="px-6 py-12 text-center text-slate-500 italic">
                No recent jobs found on this queue. Go dispatch some work! 🚀
            </td>
        </tr>"#.to_string();
    }

    let mut rows = String::new();
    for job in jobs {
        let badge_class = match job.status.as_str() {
            "pending" => "bg-yellow-400/10 text-yellow-400 border border-yellow-500/20",
            "processing" => "bg-indigo-400/10 text-indigo-400 border border-indigo-500/20 animate-pulse",
            "failed" => "bg-rose-400/10 text-rose-400 border border-rose-500/20",
            _ => "bg-emerald-400/10 text-emerald-400 border border-emerald-500/20",
        };

        let action_button = if job.status == "failed" {
            format!(
                r#"<button hx-post="/horizon/retry/{}" 
                           hx-swap="outerHTML"
                           class="px-3 py-1.5 rounded-lg bg-teal-500/10 hover:bg-teal-500/20 text-teal-400 text-xs font-semibold transition-all duration-200">
                    Retry Job
                </button>"#,
                job.id
            )
        } else {
            r#"<span class="text-xs text-slate-600 font-medium">No actions</span>"#.to_string()
        };

        let error_log = if let Some(ref err) = job.error {
            format!(
                r#"<div class="text-[11px] text-rose-400 font-mono mt-1 max-w-md overflow-x-auto bg-rose-950/20 p-2 rounded border border-rose-500/10">
                    {}
                </div>"#,
                err
            )
        } else {
            "".to_string()
        };

        rows.push_str(&format!(
            r#"<tr class="hover:bg-slate-900/30 transition-all duration-150">
                <td class="px-6 py-4 whitespace-nowrap">
                    <span class="font-mono text-xs text-slate-500 font-bold">{}</span>
                    <div class="text-sm font-semibold text-slate-200 mt-0.5 capitalize">{}</div>
                </td>
                <td class="px-6 py-4">
                    <span class="font-mono text-xs text-teal-300">{}</span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize {}">
                        {}
                    </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap font-mono text-xs text-slate-300">
                    {}
                </td>
                <td class="px-6 py-4">
                    <span class="text-xs text-slate-500">{}</span>
                    {}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-right">
                    {}
                </td>
            </tr>"#,
            &job.id[0..8],
            job.name,
            job.payload,
            badge_class,
            job.status,
            job.attempts,
            job.created_at,
            error_log,
            action_button
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::ServiceExt;
    use axum::body::Body;

    #[tokio::test]
    async fn test_horizon_dashboard_router_compiles() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let app = router(queue);

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_horizon_jobs_table() {
        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let app = router(queue);

        let req = Request::builder().uri("/jobs-table").body(Body::empty()).unwrap();
        let response = app.oneshot(req).await.unwrap();

        assert_eq!(response.status(), 200);
    }
}
