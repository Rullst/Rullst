use crate::ws::WebSocket;
use async_trait::async_trait;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use serde_json::Value;

pub mod recovery;

/// Rullst Live Component (Server-Driven UI)
/// Inspired by Phoenix LiveView and Laravel Livewire, allowing you to write
/// interactive components entirely in Rust, updated in real-time via WebSockets.
#[async_trait]
pub trait LiveComponent: Send + Sync + Default + 'static {
    /// Called on the first render (both on the initial HTTP load and when the WebSocket connection opens).
    async fn mount(&mut self) {}

    /// Processes JSON events originating from the frontend via WebSocket.
    /// HTMX will by default send a JSON payload containing headers and submitted values (hx-vals, forms).
    async fn handle_event(&mut self, _payload: Value) {}

    /// Renders the current state of the component as an HTML String.
    /// REQUIRED: The root of the rendered string MUST have a unique `id` attribute
    /// so that HTMX knows exactly which DOM node to update.
    fn render(&self) -> String;
}

/// Largest WebSocket message or frame accepted by [`live_ws_handler`].
///
/// HTMX event payloads are small JSON documents. Without an explicit bound the
/// socket inherited tungstenite's 64 MiB message default, and every message is
/// parsed into a `serde_json::Value`.
const MAX_LIVE_MESSAGE_BYTES: usize = 64 * 1024;

/// Generic Axum handler for the WebSocket route of a Rullst Live component.
/// It will instantiate the component, call `mount`, and enter the event-listening loop.
///
/// Incoming frames and messages are limited to 64 KiB. A larger message ends
/// the session before it is buffered in full or parsed; `serde_json` also keeps
/// its default nesting limit of 128. Origin checks, authentication, connection
/// caps and idle timeouts remain route-level application policy. The
/// `live::recovery` API applies stricter bounds and admission control.
pub async fn live_ws_handler<C: LiveComponent>(ws: WebSocketUpgrade) -> impl IntoResponse {
    let ws = ws
        .max_message_size(MAX_LIVE_MESSAGE_BYTES)
        .max_frame_size(MAX_LIVE_MESSAGE_BYTES);
    ws.on_upgrade(|socket| async move {
        let mut rullst_ws = WebSocket::new(socket);
        let mut component = C::default();

        // Mount the initial state in the WebSocket session
        component.mount().await;

        // Continuous loop receiving events from the frontend (HTMX ws-ext)
        while let Some(Ok(msg)) = rullst_ws.recv().await {
            // HTMX sends messages in JSON format with headers and input values
            if let Ok(payload) = serde_json::from_str::<Value>(&msg) {
                // Forward the event to the component lifecycle
                component.handle_event(payload).await;

                // Re-render the HTML after the possible state mutation
                let html = component.render();

                // Push the new HTML via WebSocket. HTMX will hot-swap it automatically using the root ID.
                if let Err(e) = rullst_ws.send_html(html).await {
                    eprintln!("Rullst Live WS Error: {}", e);
                    break; // Client disconnected or network failure
                }
            }
        }
    })
}

/// Utility to facilitate mounting a Live component in a normal HTTP page.
pub struct Live;

impl Live {
    /// Generates the wrapper `<div>` tag that activates the `hx-ext="ws"` HTMX extension.
    /// It pre-renders (`mount` + `render`) on the first load. Search-engine
    /// behavior still depends on the rendered document and deployment.
    pub async fn mount<C: LiveComponent>(ws_path: &str) -> String {
        let mut comp = C::default();
        comp.mount().await;
        let html = comp.render();

        // HTML escape ws_path to prevent path/attribute injection
        let safe_path = ws_path
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        // Wrap the component in an invisible div that instructs HTMX to open the WebSocket
        format!(
            "<div hx-ext=\"ws\" ws-connect=\"{}\">\n{}\n</div>",
            safe_path, html
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct DummyComponent;

    #[async_trait]
    impl LiveComponent for DummyComponent {
        fn render(&self) -> String {
            "<h1>Live Demo</h1>".to_string()
        }
    }

    #[derive(Default)]
    struct CountingComponent {
        events: usize,
    }

    #[async_trait]
    impl LiveComponent for CountingComponent {
        async fn handle_event(&mut self, _payload: Value) {
            self.events += 1;
        }

        fn render(&self) -> String {
            format!("<p id=\"events\">{}</p>", self.events)
        }
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    async fn legacy_handler_closes_oversized_messages_without_parsing_them() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;

        let app = axum::Router::new().route(
            "/ws",
            axum::routing::get(live_ws_handler::<CountingComponent>),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .unwrap();
        let wait = std::time::Duration::from_secs(5);

        let bounded = format!("[{}0]", "0,".repeat(MAX_LIVE_MESSAGE_BYTES / 2 - 2));
        assert!(bounded.len() <= MAX_LIVE_MESSAGE_BYTES);
        socket.send(Message::Text(bounded.into())).await.unwrap();
        let reply = tokio::time::timeout(wait, socket.next()).await.unwrap();
        assert!(
            matches!(&reply, Some(Ok(Message::Text(html))) if html.as_str() == "<p id=\"events\">1</p>")
        );

        let oversized = format!("[{}0]", "0,".repeat(MAX_LIVE_MESSAGE_BYTES / 2));
        assert!(oversized.len() > MAX_LIVE_MESSAGE_BYTES);
        let _ = socket.send(Message::Text(oversized.into())).await;
        let reply = tokio::time::timeout(wait, socket.next()).await.unwrap();
        assert!(
            !matches!(reply, Some(Ok(Message::Text(_)))),
            "an oversized message must close the session instead of being rendered"
        );
        server.abort();
    }

    #[tokio::test]
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    async fn legacy_handler_keeps_the_session_across_control_frames() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;

        let app = axum::Router::new().route(
            "/ws",
            axum::routing::get(live_ws_handler::<CountingComponent>),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await });
        let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .unwrap();

        socket
            .send(Message::Ping(b"keepalive".to_vec().into()))
            .await
            .unwrap();
        socket
            .send(Message::Pong(b"unsolicited".to_vec().into()))
            .await
            .unwrap();
        socket.send(Message::Text("{}".into())).await.unwrap();

        let rendered = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                match socket.next().await {
                    Some(Ok(Message::Pong(_))) => continue,
                    other => break other,
                }
            }
        })
        .await
        .unwrap();
        assert!(
            matches!(&rendered, Some(Ok(Message::Text(html))) if html.as_str() == "<p id=\"events\">1</p>"),
            "control frames must not end the session: {rendered:?}"
        );
        server.abort();
    }

    #[tokio::test]
    async fn test_live_mount() {
        let html = Live::mount::<DummyComponent>("/ws/demo?a=1&b=2").await;
        assert!(html.contains("hx-ext=\"ws\""));
        assert!(html.contains("ws-connect=\"/ws/demo?a=1&amp;b=2\""));
        assert!(html.contains("<h1>Live Demo</h1>"));
        assert_ne!(html, "xyzzy");
        assert_ne!(html, "");
    }
}
