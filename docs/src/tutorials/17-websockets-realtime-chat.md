# Tutorial 17: Real-Time WebSockets & Presence 💬

Build real-time channels, pub-sub broadcasting, and presence tracking using `rullst::realtime`.

---

## 🛠️ Step 1: Subscribe to Real-Time Channels

```rust
use rullst::realtime::{RealtimeChannel, Message};

pub async fn handle_chat_room(channel: RealtimeChannel) {
    let mut rx = channel.subscribe("room:general").await;

    while let Ok(msg) = rx.recv().await {
        println!("Received message in room:general: {:?}", msg);
    }
}
```

---

## 💻 Step 2: Broadcast Messages to Subscribers

```rust
use rullst::realtime::RealtimeEngine;

pub async fn send_chat_message(engine: RealtimeEngine, user: String, content: String) {
    engine.broadcast("room:general", serde_json::json!({
        "sender": user,
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })).await;
}
```

---

## 💡 Key Takeaways
- `rullst::realtime` channels support broadcast, direct client messaging, and room presence states.
- High concurrency powered by Tokio broadcast channels and WebSockets.
