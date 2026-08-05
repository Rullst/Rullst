# Tutorial 34: Live Analytics Dashboard (`rullst::live` & WebSockets) 📈

Build a live financial analytics dashboard combining `rullst::live` server components, WebSockets presence, and HTMX OOB HTML swaps.

---

## 🛠️ Step 1: Create Analytics LiveComponent

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
        self.revenue_mrr = 12450.00;
        self.active_users = 342;
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

```rust
use rullst::live::Live;
use crate::live::analytics_dashboard::AnalyticsDashboard;

pub async fn analytics_page() -> String {
    Live::mount::<AnalyticsDashboard>("/ws/analytics").await
}
```

---

## 💡 Key Takeaways
- Zero JavaScript required to maintain stateful real-time WebSockets connections.
- Out-Of-Band (OOB) swaps allow multiple sections of the dashboard to update simultaneously.
