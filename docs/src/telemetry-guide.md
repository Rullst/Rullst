# Rullst Telemetry, Spans & Process Observability 📡

Rullst v12 exposes three related but separate observability surfaces:

- `RadarSnapshot` samples supported process and Tokio runtime data;
- `radar_metrics_router()` exposes those samples in Prometheus text format;
- `SpanCollector` is a bounded, process-local buffer for spans that application
  or framework code records explicitly.

The optional `telemetry` Cargo feature also installs an OpenTelemetry tracing
layer. None of these components proves a performance target or replaces a
durable production observability backend.

## Process and Tokio observations

`RadarSnapshot::collect_async()` measures one scheduler yield and samples the
probes available on the current platform:

```rust
use rullst_core::radar::RadarSnapshot;

# async fn inspect_process() {
let snapshot = RadarSnapshot::collect_async().await;
println!("uptime: {}s", snapshot.uptime_seconds);
println!("rss: {:?} MB", snapshot.memory_rss_mb);
println!("cpu: {:?}%", snapshot.cpu_usage_percent);
println!("tokio tasks: {:?}", snapshot.active_tokio_tasks);
println!("yield observation: {:?} us", snapshot.tokio_latency_micros);
# }
```

The option-valued fields are deliberately `None` when a real probe is not
available. Linux and Windows provide the current RSS/CPU implementations;
active-task data requires a Tokio runtime. A yield observation is not a complete
event-loop latency distribution.

## Prometheus endpoint

Mount the metrics router explicitly:

```rust
use axum::Router;
use rullst_core::radar::radar_metrics_router;

let app = Router::new().merge(radar_metrics_router()); // GET /metrics
```

Only available metrics are emitted. The exporter formats a point-in-time local
snapshot; authentication, network exposure, scraping, retention, dashboards,
alerts, and multi-instance aggregation belong to the deployment.

## Bounded local span collector

The global collector holds at most 500 `TraceSpan` records in memory. Recording
is explicit; merely constructing a server does not instrument every HTTP, SQL,
AI, mail, or security operation.

```rust
use rullst_core::telemetry_spans::{TraceSpan, global_span_collector};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

let started = Instant::now();
// Run the operation being observed.

let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_secs())
    .unwrap_or_default();

global_span_collector().record(TraceSpan {
    name: "catalog.refresh".to_string(),
    kind: "job".to_string(),
    duration_us: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
    timestamp,
});
```

Studio's `/studio/traces` and `/studio/radar` pages display the records that are
actually present in this process. The buffer is not distributed, persistent,
or a parent/child tracing model.

## OpenTelemetry export

Enable the feature and point it at an OTLP/HTTP collector:

```toml
[dependencies]
rullst-core = { version = "12.0.0-rc.1", features = ["telemetry"] }
```

```env
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
RUST_LOG=info
```

Then initialize the tracing subscriber once at startup:

```rust
# fn initialize() -> Result<(), Box<dyn std::error::Error>> {
rullst_core::telemetry::init_telemetry()?;
# Ok(())
# }
```

`Server::run` also attempts this initialization, but an application that needs
to fail closed on telemetry configuration should initialize it explicitly and
handle the returned error before starting the server. The current exporter uses
OTLP over HTTP and the service resource name `rullst-app`.

`RedactPersonalDataLayer` detects a small list of sensitive *field names* and
emits a warning. It cannot rewrite a tracing event already observed by another
layer, so secrets must be removed or redacted at the call site.

## Studio boundaries

- Radar cards poll the local `/api/radar` endpoint and display `Unavailable`
  instead of fabricated values.
- Local span pages reflect only the in-memory collector in the Studio process.
- Studio binds to loopback by default and has no built-in shared-environment
  password mode. Do not expose it publicly without an authenticated boundary.
- Measure collector/export overhead against the real application workload and
  release build; Rullst publishes no universal latency or memory number.
