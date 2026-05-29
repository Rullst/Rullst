use async_trait::async_trait;
use axum::{Router, routing::get};
use futures_util::{SinkExt, StreamExt};
use rullst::live::{Live, LiveComponent, live_ws_handler};
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

// A simple mock component for testing
#[derive(Default, Clone)]
struct CounterComponent {
    count: i32,
    mounted: bool,
}

#[async_trait]
impl LiveComponent for CounterComponent {
    async fn mount(&mut self) {
        self.mounted = true;
        self.count = 10;
    }

    async fn handle_event(&mut self, payload: Value) {
        if let Some(action) = payload.get("action").and_then(|v| v.as_str()) {
            match action {
                "increment" => self.count += 1,
                "decrement" => self.count -= 1,
                _ => {}
            }
        }
    }

    fn render(&self) -> String {
        format!("<div id=\"counter\">Count: {}</div>", self.count)
    }
}

#[tokio::test]
async fn test_live_mount_html() {
    let html = Live::mount::<CounterComponent>("/live/counter").await;

    assert!(html.contains("hx-ext=\"ws\""));
    assert!(html.contains("ws-connect=\"/live/counter\""));
    assert!(html.contains("<div id=\"counter\">Count: 10</div>"));
}

#[tokio::test]
async fn test_live_mount_safe_path() {
    let html = Live::mount::<CounterComponent>("/live?param=\"hack\"&other='<>'").await;

    assert!(
        html.contains("ws-connect=\"/live?param=&quot;hack&quot;&amp;other=&#x27;&lt;&gt;&#x27;\"")
    );
}

#[tokio::test]
async fn test_live_ws_handler() {
    // 1. Build an Axum router with the WS handler
    let app = Router::new().route("/ws", get(live_ws_handler::<CounterComponent>));

    // 2. Start a real server on a random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 3. Connect via WebSocket client
    let ws_url = format!("ws://{}/ws", addr);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("Failed to connect to WS");

    // 4. Send an increment event
    let event = serde_json::json!({
        "action": "increment"
    });

    ws_stream
        .send(Message::Text(serde_json::to_string(&event).unwrap().into()))
        .await
        .expect("Failed to send message");

    // 5. Receive the updated HTML
    if let Some(Ok(Message::Text(html_response))) = ws_stream.next().await {
        let html_string = html_response.to_string();
        // It started at 10 (mount), we incremented it to 11
        assert_eq!(html_string, "<div id=\"counter\">Count: 11</div>");
    } else {
        panic!("Did not receive text message from server");
    }

    // 6. Send a decrement event
    let event2 = serde_json::json!({
        "action": "decrement"
    });

    ws_stream
        .send(Message::Text(
            serde_json::to_string(&event2).unwrap().into(),
        ))
        .await
        .expect("Failed to send message");

    // 7. Receive the updated HTML
    if let Some(Ok(Message::Text(html_response))) = ws_stream.next().await {
        let html_string = html_response.to_string();
        // It was 11, we decremented it to 10
        assert_eq!(html_string, "<div id=\"counter\">Count: 10</div>");
    } else {
        panic!("Did not receive text message from server");
    }

    // 8. Gracefully close
    ws_stream.close(None).await.unwrap();
}

#[tokio::test]
async fn test_live_module_exists() {
    assert!(true);
}

