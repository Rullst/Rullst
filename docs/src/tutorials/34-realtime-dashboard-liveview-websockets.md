# Tutorial 34: Live Analytics Dashboard (`rullst::live` & WebSockets) 📈

Build a per-connection analytics view with `rullst::live`. Feed it values from
your own authoritative metrics source; the example below uses explicit state
only to demonstrate the component lifecycle.

---

## 🛠️ Step 1: Create Analytics LiveComponent

This fragment expects the `crate::live::analytics_dashboard` module created in
Step 1 to be registered by the generated application:

```rust
use async_trait::async_trait;
use rullst::live::LiveComponent;
use serde_json::Value;

#[derive(Default)]
pub struct AnalyticsDashboard {
    pub revenue_mrr: f64,
    pub active_users: usize,
}

#[async_trait]
impl LiveComponent for AnalyticsDashboard {
    async fn mount(&mut self) {
        // Replace these initial values with an application-owned metrics query.
        self.revenue_mrr = 0.0;
        self.active_users = 0;
    }

    async fn handle_event(&mut self, payload: Value) {
        if let Some(event) = payload.get("event").and_then(|v| v.as_str()) {
            if event == "refresh" {
                self.revenue_mrr += 100.00;
                self.active_users += 1;
            }
        }
    }

    fn render(&self) -> String {
        format!(
            r#"<div id="analytics-dashboard" class="p-8 bg-slate-900 text-white rounded-2xl shadow-2xl border border-slate-800">
    <h2 class="text-2xl font-bold mb-6">Live Analytics Stream</h2>
    <div class="grid grid-cols-2 gap-6 mb-6">
        <div class="p-4 bg-slate-800 rounded-xl">
            <p class="text-slate-400 text-sm">MRR</p>
            <p class="text-3xl font-mono text-emerald-400">${:.2}</p>
        </div>
        <div class="p-4 bg-slate-800 rounded-xl">
            <p class="text-slate-400 text-sm">Active Users</p>
            <p class="text-3xl font-mono text-cyan-400">{}</p>
        </div>
    </div>
    <button ws-send name="event" value="refresh" class="px-6 py-3 bg-indigo-600 hover:bg-indigo-500 font-semibold rounded-xl">
        ⚡ Refresh Live Stream
    </button>
</div>"#,
            self.revenue_mrr, self.active_users
        )
    }
}
```

---

## 💻 Step 2: Render in View Page

```rust,ignore
use rullst::live::Live;
use crate::live::analytics_dashboard::AnalyticsDashboard;

pub async fn analytics_page() -> String {
    Live::mount::<AnalyticsDashboard>("/ws/analytics").await
}
```

Register `/ws/analytics` with
`axum::routing::get(rullst::live::live_ws_handler::<AnalyticsDashboard>)` and
load a pinned HTMX WebSocket extension in the page.

---

## 💡 Key Takeaways
- Rullst owns the server-side component lifecycle; a browser transport is still
  required.
- The current implementation re-renders an HTML fragment after each valid JSON
  event. It does not provide distributed state, replay, authorization or a
  metrics source automatically.
