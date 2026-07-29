use axum::Router;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{
    Html,
    sse::{Event, Sse},
};
use axum::routing::get;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub latency_ms: u128,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct LoggerState {
    pub tx: broadcast::Sender<RequestLog>,
}

impl Default for LoggerState {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggerState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }
}

pub async fn logger_middleware(
    state: axum::extract::State<Arc<LoggerState>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let start = Instant::now();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let res = next.run(req).await;

    let latency_ms = start.elapsed().as_millis();
    let status = res.status().as_u16();

    let log = RequestLog {
        method,
        uri,
        status,
        latency_ms,
        timestamp,
    };

    let _ = state.tx.send(log);

    res
}

pub fn router(state: Arc<LoggerState>) -> Router {
    Router::new()
        .route("/", get(logger_dashboard))
        .route("/stream", get(logger_stream))
        .with_state(state)
}

#[cfg_attr(mutants, mutants::skip)]
async fn logger_dashboard() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Studio Logger</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@1.9.12"></script>
    <script src="https://unpkg.com/htmx.org@1.9.12/dist/ext/sse.js"></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <h1 class="text-3xl font-bold mb-4 text-emerald-400">Request Logger</h1>
    <div hx-ext="sse" sse-connect="/studio/requests/stream" sse-swap="message" hx-swap="afterbegin" class="flex-1 bg-slate-900 border border-slate-800 rounded-lg p-4 overflow-y-auto space-y-2">
        <!-- New logs will appear here -->
    </div>
</body>
</html>"#.to_string())
}

async fn logger_stream(
    axum::extract::State(state): axum::extract::State<Arc<LoggerState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| {
        let log = res.ok()?;
        let color = match log.status {
            200..=299 => "text-green-400",
            300..=399 => "text-yellow-400",
            400..=499 => "text-orange-400",
            500..=599 => "text-red-400",
            _ => "text-slate-400",
        };
        let html = format!(
            "<div class=\"p-2 rounded bg-slate-800 flex justify-between\">\
             <span><span class=\"font-bold {}\">{}</span> <span class=\"text-slate-300\">{}</span></span>\
             <span class=\"text-slate-500\">{}ms</span>\
             </div>",
            color, log.method, log.uri, log.latency_ms
        );
        Some(Ok(Event::default().data(html)))
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
