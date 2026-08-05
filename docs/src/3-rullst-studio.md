# Rullst Studio: Real-Time Monitoring & Control Room

**Rullst Studio** is your local development control room. It comes built-in with all Rullst application blueprints.

While you develop your application on port `3000`, Rullst Studio automatically boots up on port `5555` (`http://localhost:5555`).

## ✨ Features & Visual Tooling Suite

The Studio acts as a zero-overhead local observability dashboard. It connects to your Rullst application via WebSockets in the background to capture runtime telemetry without affecting your app's performance.

With Rullst Studio, you can:
- **📡 Rullst Radar (`/studio/radar`):** Kernel-level telemetry visualizer displaying Tokio runtime tick latency (in µs), active async tasks, process CPU, RSS memory consumption, and a direct link to the Prometheus `/metrics` exporter.
- **💳 Revenue Dashboard (`/studio/capital`):** Real-time SaaS MRR/ARR analytics, active subscriber count, churn rate calculator, and live Stripe / LemonSqueezy Webhook Audit Inspector.
- **🛡️ Visual Threat Radar / SOC (`/studio/security`):** Real-time threat vectors, banned IP reputation scores, blocked honeypot hits (`rullst-honey`), and RASP incident reports.
- **📊 Distributed Tracing Visualizer (`/studio/tools/traces`):** Jaeger/Zipkin-style flamegraph inspector visualizing microsecond-level HTTP, SQL, and AI prompt spans.
- **Monitor Traffic & SQL Auditing:** Real-time HTTP request streams and `rullst-orm` SQL query time inspections for hunting N+1 query bottlenecks.
- **Debug Async Jobs & Queues:** Visualize worker queues and retry failing jobs.

## How to Access

1. Run your Rullst project:
   ```bash
   cargo rullst dev
   ```
2. Open your browser at `http://localhost:5555`

> **Note:** Rullst Studio is designed exclusively for local development environments (`cargo rullst dev`). In production (compiled via `cargo build --release`), Studio's features are completely stripped away via conditional compilation (`cfg(debug_assertions)`), guaranteeing **Zero Overhead** for production servers.
