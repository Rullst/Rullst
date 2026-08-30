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
        let html = render_log_entry(&log, color);
        Some(Ok(Event::default().data(html)))
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

fn render_log_entry(log: &RequestLog, color: &str) -> String {
    format!(
        "<div class=\"p-2 rounded bg-slate-800 flex justify-between\">\
         <span><span class=\"font-bold {}\">{}</span> <span class=\"text-slate-300\">{}</span></span>\
         <span class=\"text-slate-500\">{}ms</span>\
         </div>",
        color,
        rullst_core::html::escape_str(&log.method),
        rullst_core::html::escape_str(&log.uri),
        log.latency_ms
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_logger_dashboard_and_state() {
        let state = Arc::new(LoggerState::new());
        let app = router(state.clone());

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let default_state = LoggerState::default();
        let _ = default_state.tx.send(RequestLog {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            status: 200,
            latency_ms: 12,
            timestamp: "2026-08-22T00:00:00Z".to_string(),
        });
    }

    #[test]
    fn request_log_markup_escapes_untrusted_method_and_uri() {
        let html = render_log_entry(
            &RequestLog {
                method: "<script>".to_string(),
                uri: "/?<img src=x onerror=alert(1)>".to_string(),
                status: 200,
                latency_ms: 5,
                timestamp: "2026-08-29T00:00:00Z".to_string(),
            },
            "text-green-400",
        );

        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
    }
}
