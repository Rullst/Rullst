# Tutorial 23: Process Telemetry & Prometheus Exporter 📡

Monitor process uptime, RSS memory, CPU, Tokio scheduler-yield latency, and active tasks using Rullst Radar (`rullst::radar`) and Prometheus (`GET /metrics`).

---

## 🛠️ Step 1: Mount Prometheus Exporter

In `src/main.rs`:

```rust,no_run
use axum::Router;
use rullst_core::radar::radar_metrics_router;
use rullst::Server;

#[tokio::main]
async fn main() -> Result<(), rullst_core::server::ServerError> {
    let app = Router::new()
        .merge(radar_metrics_router()); // Exposes GET /metrics

    Server::new(app.into()).run(3000).await
}
```

---

## 📊 Step 2: Prometheus Metrics Scrape Output

Query `GET /metrics`:

```text
# HELP rullst_uptime_seconds Process uptime in seconds.
# TYPE rullst_uptime_seconds counter
rullst_uptime_seconds 120

# HELP rullst_memory_rss_bytes Process RSS memory consumption in bytes.
# TYPE rullst_memory_rss_bytes gauge
rullst_memory_rss_bytes 24536678

# HELP rullst_cpu_usage_percent Process CPU utilization percentage.
# TYPE rullst_cpu_usage_percent gauge
rullst_cpu_usage_percent 1.25

# HELP rullst_tokio_active_tasks Total active Tokio tasks count.
# TYPE rullst_tokio_active_tasks gauge
rullst_tokio_active_tasks 4

# HELP rullst_tokio_latency_microseconds Observed Tokio scheduler yield latency in microseconds.
# TYPE rullst_tokio_latency_microseconds gauge
rullst_tokio_latency_microseconds 42
```

The CPU gauge needs an earlier sample in the same process, so the first
collection omits it.

Visual dashboard available in Studio: `http://localhost:5555/studio/radar`.

`/metrics` is not authenticated by `radar_metrics_router()`. Restrict it with a
private network, service-mesh policy, or reviewed authentication middleware;
process and runtime measurements can disclose operational information.

---

## 💡 Key Takeaways
- The response is a point-in-time local snapshot and allocates its text body.
- Linux and Windows expose supported process RSS/CPU probes. Tokio task data is
  available only inside a Tokio runtime; unsupported probes are omitted.
- The scheduler-yield observation is not a universal request-latency target.
