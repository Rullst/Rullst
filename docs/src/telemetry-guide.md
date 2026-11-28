# Rullst Telemetry, Spans & Process Observability 📡

Rullst exposes three related but separate observability surfaces:

- `RadarSnapshot` samples supported process and Tokio runtime data;
- `radar_metrics_router()` exposes those samples in Prometheus text format;
- `SpanCollector` is a bounded, process-local buffer for spans that application
  or framework code records explicitly.

The optional `telemetry` Cargo feature also installs an OpenTelemetry tracing
layer. None of these components proves a performance target or replaces a
durable production observability backend.

Rullst 13 (since 13.0.0-alpha.1) adds an explicit, minimized cross-process
profile and repairs legacy OTLP transport initialization. See
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

global_span_collector().record(TraceSpan::new(
    "catalog.refresh",
    "job",
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
    timestamp,
));
```

Studio's `/studio/traces` and `/studio/radar` pages display the records that are
actually present in this process. The buffer is not distributed, persistent,
or a parent/child tracing model.

## OpenTelemetry export

The legacy initializer below uses the OTLP transport repaired in v13. Prefer
the owned profile linked above when you need explicit metadata policy and
flush/shutdown. Enable the feature with the exact prerelease version (Cargo
never selects a prerelease on its own):

```toml
[dependencies]
rullst-core = { version = "=13.0.0-rc.1", features = ["telemetry"] }
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

Without `RUST_LOG` (or with one that does not parse), the subscriber that
`Server::run` and `init_telemetry` install uses `info,sqlx=warn`: SQLx's own
connection and statement events, such as `acquired connection`, appear only
from WARN up. To see them again, set `RUST_LOG=info,sqlx=info` (or
`sqlx=debug`); any `RUST_LOG` replaces the default as a whole.

Since 13.1.0 that subscriber creates the ORM's `rullst.orm.*` spans only
while something in the process records them: once the
[development dashboard endpoint](#development-dashboard-endpoint) or the
[production monitor](production-monitoring.md) is mounted, and from the start
in the `telemetry` build, whose OpenTelemetry layer exports them. Until then an
ORM call creates no span, so it does not pay for creating it, formatting its
fields and entering it again on every poll of SQLx's streams. The difference
shows in the log: an event written during such an ORM call, for example SQLx's
warning about a slow connection acquisition or an application event inside an
`Orm::transaction` closure, has no `rullst.orm.query{...}:` or
`rullst.orm.transaction{...}:` prefix. `RUST_LOG` selects the same events as
before, a `RUST_LOG` with a span directive (such as
`info,[rullst.orm.query]=debug`) keeps every ORM span, and an application that
installs its own subscriber is not affected.

### Health probes in the access log

`Server` prints one `[HTTP] GET /orders -> 200 (1.20 ms)` line per request. In
staging and production it skips `/health` and `/ready` requests that answer
below 400, so orchestrator probes every few seconds do not fill the log; a
failing probe (for example `/ready -> 503` while draining) is still printed.
Set `RULLST_LOG_HEALTH_PROBES=true` in the process environment or `.env` to
print every probe again, or `false` to skip successful probes in development
too.

### Turning the access log down or off

`RULLST_ACCESS_LOG`, in the process environment or `.env`, chooses which
requests get a line (since 13.0.0-alpha.6):

| Value | Lines |
| :--- | :--- |
| `on` (the default; also `true`, `1`, `yes`) | every request |
| `errors` | responses with a status of 400 or above |
| `1/N`, `N` from 1 to 1000000 (for example `1/100`) | every response of 400 or above, and one in `N` of the others |
| `off` (also `false`, `0`, `no`) | none: `Server` does not install the access log |

The health-probe and path rules above apply to the lines that remain. Any
other value stops startup with `ServerError::Configuration`.

Lines are written by a background thread, not by the request: each request
hands its line to a bounded queue (8192 lines, 2 MiB) and never waits for
stdout, and the thread writes what is queued in batches, at most a couple of
milliseconds later. When stdout cannot keep up (a slow pipe or log driver)
and the queue is full, the line is dropped instead of slowing the server, and
the next batch reports how many were lost:

```text
[HTTP] access log: 1250 lines dropped because stdout did not keep up
```

On a graceful shutdown `Server` waits up to one second for the queued lines.
Until 13.0.0-alpha.6 each request wrote its line to stdout itself, holding the
stdout lock, which cost a large share of the throughput of a generated
application under load.

A line is queued when its response is ready, so the log is in the order in
which responses finished, not the order in which requests arrived: a slow
request's line comes after the lines of faster requests that started later,
and two requests that finish together may be written in either order. The
elapsed time on each line says how long its request took; a line carries no
arrival time or request id. To follow one request, or to reconstruct arrival
order, log a request id of your own from a middleware, or use the request spans
of the [OpenTelemetry export](#opentelemetry-export), rather than the position
of a line in the log.

### Refused requests in the access log

Some of `Server`'s own layers answer a request before the application's
handler runs: the CSRF check and the authentication of
`Server::with_machine_endpoints` routes, the WAF (in staging and production;
in development it only logs a warning and lets the request through), the rate
limiter of `Server::rate_limit`, Traffic Shield and lifecycle admission while
the application is starting or draining. Since 13.0.0-rc.1 their responses get
an access-log line too, with the status and the layer that refused the
request, in every environment:

```text
[HTTP] POST /login -> 403 (0.31 ms) refused by csrf
[HTTP] GET /search -> 429 (0.12 ms) refused by rate-limit
```

The layer is one of `csrf`, `machine-endpoint`, `waf`, `rate-limit`,
`traffic-shield` and `lifecycle`. The line follows the same rules as the
others (the path or route, `RULLST_ACCESS_LOG`, the health-probe rule) and
writes neither the request's body, its headers nor the token it lacked.
Until 13.0.0-rc.1 such requests reached no access-log line, because the
access log runs inside those layers. A refusal by `rate_limit_middleware` or
`csrf_middleware` that an application adds to its own router gets one line,
from the access log it passes. A CORS preflight that the CORS layer answers,
and a static file under `/static/`, are still not written.

### Paths in the access log

A request path can carry a secret, such as the token of a booking or sign-in
link (`/booking/3f9a…`). Since 13.0.0-alpha.5, in staging and production the
access log writes the route that matched instead, as it was declared:

```text
[HTTP] GET /booking/{token} -> 200 (3.10 ms)
```

Development and test keep the request path, which is what you want to see
while working. A request that matched no route (a `404`, or a path under a
nested service such as a `ServeDir`) writes its path cut to 128 bytes, ending
in `…` when cut, with every byte outside visible ASCII written as `%XX`. The
query string is never written. To choose one form in every environment, set
it in `Rullst.toml`:

```toml
[app]
access_log_paths = "route" # or "full", or "auto" (the default)
```

or on the server, which takes precedence:

```rust,ignore
use rullst::config::AccessLogPaths;

Server::new(router).access_log_paths(AccessLogPaths::Route);
```

`RedactPersonalDataLayer` detects a small list of sensitive *field names* and
emits a warning. It cannot rewrite a tracing event already observed by another
layer, so secrets must be removed or redacted at the call site.

## Development dashboard endpoint

`cargo rullst dash` shows live request, latency, error, ORM and queue figures
from `GET /_rullst/dev-telemetry` (v13). `Server` mounts this endpoint next to
the development reload routes, so it exists only when all of these hold:

- a debug build (`debug_assertions`);
- the Development environment (`RULLST_ENV`, legacy `APP_ENV` or `[app].env`;
  see [Tutorial 09](tutorials/09-environment-config.md));
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
  `unavailable` with `subscriber_not_installed` or `orm_spans_filtered`. An
  observed database also reports possible N+1 queries (v13):
  `repeated_threshold` (3), `repeated_queries_total` and the newest 16
  `recent_repeated` findings, each one ORM operation fingerprint that a single
  request ran at least that many times, with the request's method and matched
  route template (or its path when no route matched);
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
    ],
    "repeated_threshold": 3,
    "repeated_queries_total": 1,
    "recent_repeated": [
      {"seq": 1, "method": "GET", "route": "/orders/{id}", "fingerprint": "LineItem.find (line_items)", "occurrences": 12}
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

ORM figures come from the existing secret-free `rullst.orm.query` spans.
`telemetry::init_telemetry` (which `Server::run` calls) adds a passive layer
that times the outermost such span of each ORM operation, from creation until
it closes. Since 13.0.0-alpha.3 the layer is installed in release builds too,
for the [production monitoring endpoint](production-monitoring.md); outside a
recording scope it only compares the span's name; nested operations such as eager loads belong to their
outer operation. The span of a `chunk`/`chunk_by_id` traversal also covers the
application's handler, so it is neither counted nor treated as enclosing: each
page it fetches and each operation the handler runs is counted on its own. An
application that installs its own global subscriber first, or a `RUST_LOG` that
disables `rullst_orm` INFO spans, receives the `unavailable` state instead of
misleading zeros. Statements executed directly
through SQLx are not observed.

N+1 findings use the shared rule in `rullst_core::query_patterns`, which
Studio's trace view also applies: the same fingerprint at least
`N_PLUS_ONE_THRESHOLD` (3) times within one request. The recording layer runs
each request in a task-local scope; an outermost ORM operation created on that
task adds its fingerprint (`Model.operation (table)`, built from the static
span labels, so no SQL text or literal value is involved). Operations without
a model or table label, such as `Orm::raw` statements, are not fingerprinted
because different statements share one label, and operations started on
another task (for example inside `tokio::spawn`) are not attributed to the
request. Both cases can only hide a repetition, never invent one. A finding is
a heuristic: repetition can be intentional.

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
