# Tutorial 16: LiveView Server-Driven UI (`rullst::live`) ⚡

Build a per-connection Rust component that receives JSON events and sends
rendered HTML over WebSockets. The browser still needs the HTMX WebSocket
extension (or a compatible client transport).

This tutorial documents the legacy per-connection API. For the v13 prerelease
candidate with explicit authorization, revision conflicts, bounded reconnection
and recovery after process restart, use [Recoverable Live UI](../live-recovery.md).
Its browser module and application trait are separate; `make:live` does not
automatically supply that contract.

---

## 🛠️ Step 1: Scaffold a LiveComponent

```bash
cargo rullst make:live CounterComponent
```

This creates `src/live/counter_component.rs` and declares it in
`src/live/mod.rs`. Declare `mod live;` in the crate root yourself; the command
does not edit `src/main.rs` or `src/lib.rs`.

---

## 💻 Step 2: Implement the Component Lifecycle

Edit the generated `src/live/counter_component.rs`, for example:

```rust
use rullst::async_trait;
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

This route fragment belongs in the same application module where
`CounterComponent` is in scope:

```rust,ignore
use rullst::html::Markup;
use rullst::live::Live;
use crate::live::counter_component::CounterComponent;

// `Live::mount` returns `Markup`: return it as the page, or interpolate it
// into an `html!` layout as `{component}`.
pub async fn page_handler() -> Markup {
    Live::mount::<CounterComponent>("/ws/counter").await
}
```

Register the page and the matching WebSocket route, for example in the
`routes![...]` list of `router()`:

```rust,ignore
use rullst::live::live_ws_handler;

let app = rullst::routes![
    get("/counter" => page_handler),
    get("/ws/counter" => live_ws_handler::<CounterComponent>),
];
```

The page must also load HTMX and its WebSocket extension (see the takeaways).
The command's own hint uses `/ws/counter_component`; any path works as long as
`Live::mount` and the route name the same one.

---

## 💡 Key Takeaways
- Event payloads travel over WebSocket connections; `render()` produces updated HTML fragments.
- The current component state lives in one socket task. Authentication,
  authorization, reconnect/replay, backpressure and multi-process state remain
  application concerns.
- `live_ws_handler` accepts WebSocket frames and messages of at most 64 KiB; a
  larger message closes the session before it is parsed. Origin checks,
  connection caps and idle timeouts are still route-level policy.
- Include and pin the HTMX WebSocket extension; Rullst does not inject that
  browser dependency automatically.
