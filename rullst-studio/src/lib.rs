extern crate rullst_core as rullst;
use axum::{Router, response::Html, routing::get};
use rullst_core::Queue;
use std::sync::Arc;
use utoipa::openapi::OpenApi;

pub mod api_playground;
pub mod data_browser;
pub use data_browser::run_studio;
pub mod env_viewer;
pub mod er_diagram;
pub mod feature_flags;
pub mod jobs_monitor;
pub mod logger;

pub struct Studio {
    openapi: Option<OpenApi>,
    queue: Option<Queue>,
}

impl Default for Studio {
    fn default() -> Self {
        Self::new()
    }
}

impl Studio {
    pub fn new() -> Self {
        Self {
            openapi: None,
            queue: None,
        }
    }

    pub fn with_openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    pub fn with_horizon(mut self, queue: Queue) -> Self {
        self.queue = Some(queue);
        self
    }

    pub fn into_router(self) -> Router {
        let logger_state = Arc::new(logger::LoggerState::new());
        let mut router = Router::new()
            .route("/", get(studio_dashboard))
            .nest("/data", data_browser::router())
            .nest("/requests", logger::router(logger_state.clone()))
            .layer(axum::middleware::from_fn_with_state(
                logger_state,
                logger::logger_middleware,
            ))
            .nest("/env", env_viewer::router())
            .nest("/features", feature_flags::router())
            .nest("/er", er_diagram::router());

        if let Some(openapi) = self.openapi {
            router = router.nest("/api", api_playground::router(openapi));
        }

        if let Some(queue) = self.queue {
            router = router.nest("/jobs", jobs_monitor::router(queue));
        }

        router
    }
}

async fn studio_dashboard() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <h1 class="text-4xl font-bold mb-8 text-emerald-400">Rullst Studio 🛠️</h1>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <a href="/studio/requests" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">Real-time Logger</h2>
            <p class="text-slate-400 mt-2 text-sm">Monitor incoming HTTP requests via live SSE stream.</p>
        </a>
        <a href="/studio/data" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">Data Browser</h2>
            <p class="text-slate-400 mt-2 text-sm">Visually inspect and manage database tables.</p>
        </a>
        <a href="/studio/jobs" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">Jobs Monitor</h2>
            <p class="text-slate-400 mt-2 text-sm">Dashboard for background queues and workers.</p>
        </a>
        <a href="/studio/api" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">API Playground</h2>
            <p class="text-slate-400 mt-2 text-sm">Interactive Swagger UI for your REST endpoints.</p>
        </a>
        <a href="/studio/env" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">Environment Viewer</h2>
            <p class="text-slate-400 mt-2 text-sm">Securely inspect active environment variables.</p>
        </a>
        <a href="/studio/features" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">Feature Flags</h2>
            <p class="text-slate-400 mt-2 text-sm">Manage dynamic database-backed feature flags.</p>
        </a>
        <a href="/studio/er" class="p-6 bg-slate-900 border border-slate-800 rounded-xl hover:border-emerald-500 transition-colors">
            <h2 class="text-xl font-bold text-slate-200">ER Diagram</h2>
            <p class="text-slate-400 mt-2 text-sm">Interactive visualization of your database schema.</p>
        </a>
    </div>
</body>
</html>"#.to_string())
}
