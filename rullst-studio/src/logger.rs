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
#[non_exhaustive]
pub struct RequestLog {
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub latency_ms: u128,
    pub timestamp: String,
}

#[derive(Clone)]
#[non_exhaustive]
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

async fn logger_dashboard() -> Html<String> {
    Html(r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Studio Logger</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link href="/studio/assets/studio.css" rel="stylesheet">
    <script src="/studio/assets/logger.js" defer></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <h1 class="text-3xl font-bold mb-4 text-emerald-400">Request Logger</h1>
    <div id="studio-request-stream" class="flex-1 bg-slate-900 border border-slate-800 rounded-lg p-4 overflow-y-auto space-y-2">
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

    #[tokio::test]
    async fn the_dashboard_loads_its_script_and_the_stream_colors_each_status_class() {
        let state = Arc::new(LoggerState::new());
        let page = router(state.clone())
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let page = axum::body::to_bytes(page.into_body(), 64 * 1024)
            .await
            .unwrap();
        let page = String::from_utf8(page.to_vec()).unwrap();
        assert!(page.contains(
            "<h1 class=\"text-3xl font-bold mb-4 text-emerald-400\">Request Logger</h1>"
        ));
        assert!(page.contains("<script src=\"/studio/assets/logger.js\" defer></script>"));

        let stream = router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/stream")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let mut events = stream.into_body().into_data_stream();
        for (status, color) in [
            (204, "text-green-400"),
            (302, "text-yellow-400"),
            (404, "text-orange-400"),
            (503, "text-red-400"),
            (101, "text-slate-400"),
        ] {
            state
                .tx
                .send(RequestLog {
                    method: "GET".to_string(),
                    uri: format!("/status/{status}"),
                    status,
                    latency_ms: 1,
                    timestamp: "2026-11-17T00:00:00Z".to_string(),
                })
                .expect("a subscribed stream");
            let frame = tokio::time::timeout(std::time::Duration::from_secs(10), events.next())
                .await
                .expect("an event before the deadline")
                .expect("an open stream")
                .expect("a data frame");
            let frame = String::from_utf8(frame.to_vec()).unwrap();
            assert!(
                frame.contains(&format!("<span class=\"font-bold {color}\">GET</span>")),
                "{status}: {frame}"
            );
        }
    }
}
