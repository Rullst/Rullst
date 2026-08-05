# Tutorial 16: LiveView Server-Driven UI (`rullst::live`) ⚡

Build stateful, reactive user interfaces in pure Rust that synchronize over WebSockets without writing JavaScript.

---

## 🛠️ Step 1: Scaffold a LiveComponent

```bash
cargo rullst make:live CounterComponent
```

This creates `src/live/counter_component.rs`.

---

## 💻 Step 2: Implement the Component Lifecycle

```rust
use async_trait::async_trait;
use rullst::live::LiveComponent;
use serde_json::Value;

#[derive(Default)]
pub struct CounterComponent {
    pub count: i32,
}

#[async_trait]
impl LiveComponent for CounterComponent {
    async fn mount(&mut self) {
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
        format!(
            r#"<div id="counter-component" class="p-6 bg-slate-800 text-white rounded-xl">
    <h2 class="text-xl font-bold">Counter: {}</h2>
    <button ws-send name="action" value="increment" class="px-4 py-2 bg-emerald-600 rounded">+1</button>
</div>"#,
            self.count
        )
    }
}
```

---

## 💻 Step 3: Mount Component in a Controller

```rust
use rullst::live::Live;
use crate::live::counter_component::CounterComponent;

pub async fn page_handler() -> String {
    Live::mount::<CounterComponent>("/ws/counter").await
}
```

---

## 💡 Key Takeaways
- Event payloads travel over WebSocket connections; `render()` produces updated HTML fragments.
- HTMX handles Out-Of-Band (OOB) swaps using the root container ID.
