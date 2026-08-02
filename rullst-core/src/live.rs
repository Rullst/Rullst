use crate::ws::WebSocket;
use async_trait::async_trait;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use serde_json::Value;

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

/// Generic Axum handler for the WebSocket route of a Rullst Live component.
/// It will instantiate the component, call `mount`, and enter the event-listening loop.
pub async fn live_ws_handler<C: LiveComponent>(ws: WebSocketUpgrade) -> impl IntoResponse {
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
    /// It pre-renders (`mount` + `render`) to guarantee SEO-optimised SSR on the first load.
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
