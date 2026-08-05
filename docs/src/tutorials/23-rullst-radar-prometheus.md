# Tutorial 23: Kernel Telemetry & Prometheus Exporter 📡

Monitor process RSS memory, Tokio runtime tick latency, and active tasks using Rullst Radar (`rullst::radar`) and Prometheus (`GET /metrics`).

---

## 🛠️ Step 1: Mount Prometheus Exporter

In `src/main.rs`:

```rust
use axum::Router;
use rullst_core::radar::radar_metrics_router;
use rullst::Server;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .merge(radar_metrics_router()); // Exposes GET /metrics

    Server::new().merge(app).run().await;
}
```

---

## 📊 Step 2: Prometheus Metrics Scrape Output

Query `GET /metrics`:

```text
# HELP rullst_memory_rss_bytes Process RSS memory consumption
# TYPE rullst_memory_rss_bytes gauge
rullst_memory_rss_bytes 24510464

# HELP rullst_tokio_latency_microseconds Tokio runtime tick latency in microseconds
# TYPE rullst_tokio_latency_microseconds gauge
rullst_tokio_latency_microseconds 42
```

Visual dashboard available in Studio: `http://localhost:5555/studio/radar`.

---

## 💡 Key Takeaways
- Zero allocation text-format Prometheus exporter.
- Real-time kernel metrics read directly from `/proc/self/statm`.
