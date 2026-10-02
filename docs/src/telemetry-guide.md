# Rullst Telemetry, Spans & Process Observability 📡

Rullst v12 exposes three related but separate observability surfaces:

- `RadarSnapshot` samples supported process and Tokio runtime data;
- `radar_metrics_router()` exposes those samples in Prometheus text format;
- `SpanCollector` is a bounded, process-local buffer for spans that application
  or framework code records explicitly.

The optional `telemetry` Cargo feature also installs an OpenTelemetry tracing
layer. None of these components proves a performance target or replaces a
durable production observability backend.

The unpublished v13 candidate adds an explicit, minimized cross-process profile
and repairs legacy OTLP transport initialization. See
[distributed operation tracing](distributed-tracing.md) for parent trust,
approved labels, owned shutdown, actual collector tests and admission status.

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

The following legacy initializer example targets the unpublished v13 repair.
Prefer the owned profile linked above when you need explicit metadata policy
and flush/shutdown. Enable the feature and use reviewed candidate source:

```toml
[dependencies]
rullst-core = { version = "13.0.0-alpha.1", features = ["telemetry"] }
```

```env
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://127.0.0.1:4318/v1/traces
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
The generic `OTEL_EXPORTER_OTLP_ENDPOINT` instead denotes a base URL to which
the initializer appends `/v1/traces`; do not put a gRPC endpoint in either value.

`RedactPersonalDataLayer` detects a small list of sensitive *field names* and
emits a warning. It cannot rewrite a tracing event already observed by another
layer, so secrets must be removed or redacted at the call site.

## Development dashboard endpoint

`cargo rullst dash` shows live request, latency, error, ORM and queue figures
from `GET /_rullst/dev-telemetry` (v13). `Server` mounts this endpoint next to
the development reload routes, so it exists only when all of these hold:

- a debug build (`debug_assertions`);
- the Development environment (section 4.1 of the specification);
- a valid `RULLST_DEV_GENERATION`, which the `cargo rullst dev`/`dash`
  supervisor sets for the process it starts.

Release builds, Staging, Production and an application started with
`cargo run` never mount it. The endpoint answers only a direct loopback peer
that addresses the server with a loopback `Host` (`localhost`, `127.0.0.0/8` or
`[::1]`) and, when present, a loopback `Origin`, over HTTP/1.1 or newer and
without a forwarding header (`Forwarded`, `X-Forwarded-For`,
`X-Forwarded-Host`, `X-Forwarded-Proto`, `X-Forwarded-Server`, `X-Real-IP`,
`Via`, `CF-Connecting-IP` or `True-Client-IP`); any other request receives an
empty `404`. This rejects other machines, clients resolved through trusted
proxies, DNS-rebinding pages and the common same-host reverse proxies and
tunnels (Apache `mod_proxy`, Caddy, Traefik, ngrok, cloudflared and nginx's
default HTTP/1.0 upstream), which connect from loopback and may rewrite `Host`
to a loopback address. A same-host proxy that speaks HTTP/1.1, rewrites `Host`
and adds none of these headers is indistinguishable from a local client, so do
not publish a development server through one. This is a local development
boundary, not authentication. Responses carry `Cache-Control: no-store` and
`X-Content-Type-Options: nosniff`. Like the reload poll, the dashboard's poll
bypasses the rate limiter and Traffic Shield and is neither access-logged nor
counted.

The `rullst.dev-telemetry.v1` document contains:

- `http`: request, 4xx and 5xx counters since the process started and the
  newest 64 requests (sequence number, method, path without query string,
  status, duration in microseconds). They are recorded by the outermost layer,
  so a handler panic that the development error console answers with `500` and
  the responses of the security baseline (for example a CSRF `403`), lifecycle
  admission, the rate limiter (`429`) and Traffic Shield are counted too.
  Like the access log, it skips the development polls and the files served
  from the framework's `/static` directory;
- `database`: ORM operation counters and the newest 16 operations that took at
  least 100 ms, with their static model, table and operation labels, or
  `unavailable` with `subscriber_not_installed` or `orm_spans_filtered`;
- `queue`: the pending count of the queue passed to `Server::with_dev_queue`,
  `not_configured`, or `unavailable` with `timeout` (250 ms) or `driver_error`.

```json
{
  "schema": "rullst.dev-telemetry.v1",
  "generation": "0123456789abcdef0123456789abcdef",
  "uptime_ms": 5120,
  "http": {
    "requests_total": 2,
    "client_errors_total": 0,
    "server_errors_total": 1,
    "recent": [
      {"seq": 1, "method": "GET", "path": "/", "status": 200, "duration_us": 912},
      {"seq": 2, "method": "POST", "path": "/orders", "status": 500, "duration_us": 48210}
    ]
  },
  "database": {
    "state": "observed",
    "queries_total": 3,
    "slow_queries_total": 1,
    "slow_threshold_ms": 100,
    "recent_slow": [
      {"seq": 1, "operation": "select_many", "model": "Order", "table": "orders", "duration_us": 152004}
    ]
  },
  "queue": {"state": "observed", "pending": 4}
}
```

Request and response bodies, headers, cookies, query strings, SQL text,
bindings and error messages are never recorded. Paths appear as the access log
already prints them, so a path segment that carries a secret is visible here
too. Recording starts only after the endpoint is mounted; until then each
request costs one atomic load. Methods are cut to 16 bytes, paths to 256 bytes
and labels to 64 bytes, and control characters are replaced.

ORM figures come from the existing secret-free `rullst.orm.query` spans. In
debug builds `telemetry::init_telemetry` (which `Server::run` calls) adds a
passive layer that times the outermost such span of each ORM operation, from
creation until it closes; nested operations such as eager loads belong to their
outer operation. The span of a `chunk`/`chunk_by_id` traversal also covers the
application's handler, so it is neither counted nor treated as enclosing: each
page it fetches and each operation the handler runs is counted on its own. An
application that installs its own global subscriber first, or a `RUST_LOG` that
disables `rullst_orm` INFO spans, receives the `unavailable` state instead of
misleading zeros. Statements executed directly
through SQLx are not observed.

Report a queue's pending count, read with a 250 ms limit on each poll:

```rust,no_run
use rullst_core::{Queue, Server, routes, routing::get};
use std::sync::Arc;

# async fn run(queue: Queue) -> Result<(), Box<dyn std::error::Error>> {
let queue = Arc::new(queue); // also shared with the application's workers
Server::new(routes![get("/" => || async { "OK" })])
    .with_dev_queue(queue.clone())
    .run(3000)
    .await?;
# Ok(())
# }
```

The setting has no effect outside a supervised debug Development process.

## Studio boundaries

- Radar cards poll the local `/api/radar` endpoint and display `Unavailable`
  instead of fabricated values.
- Local span pages reflect only the in-memory collector in the Studio process.
- Studio binds to loopback by default and has no built-in shared-environment
  password mode. Do not expose it publicly without an authenticated boundary.
- Measure collector/export overhead against the real application workload and
  release build; Rullst publishes no universal latency or memory number.
