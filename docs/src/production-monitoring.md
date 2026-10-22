# Production monitoring endpoint

Rullst Studio is a development tool: it runs only in debug builds, on
`127.0.0.1:5555`, and reads the local process. To watch an application in
staging or production, 13.0.0-alpha.3 adds a read-only monitoring endpoint to
the application itself:

```text
GET /_rullst/monitor/v1
```

It answers a JSON document, versioned as `rullst.monitor.v1`, with request
rates and latency per route pattern, status classes, queue depth and worker
activity, possible N+1 queries and the process radar. It never contains
database rows, request data or configuration. This is phase 1 of the
[Studio production monitor plan](https://github.com/Rullst/Rullst/blob/main/ROADMAP.md#planned-for-1300-alpha3):
phase 2 adds a "remote projects" mode to Studio on your machine that reads this
endpoint for several projects. Until then, any HTTPS client (curl, a script, an
uptime tool) can read it.

The endpoint is **off by default**. The threat model is
[`TM-STUDIO-2`](threat-models.md#tm-studio-2--production-monitoring-endpoint).

## Enable it

Two things are needed: the application mounts the monitor, and the operator
gives it a credential.

```rust,no_run
use rullst::Server;
use rullst::monitor::Monitor;
use rullst::routes;
use rullst::routing::get;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Server::new(routes![get("/" => || async { "OK" })])
        .with_monitor(Monitor::new().with_application(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .run(3000)
        .await?;
    Ok(())
}
```

Then generate a token of at least 32 random bytes and set it where the server
reads its configuration, the process environment or `.env`:

```bash
openssl rand -base64 32   # or: openssl rand -hex 32
```

```env
RULLST_MONITOR_TOKENS=<the generated value>
```

When the server starts it validates every
token: fewer than 43 base64/base64url or 64 hexadecimal characters, other
characters, fewer than ten distinct characters or a placeholder such as
`changeme` stop start-up with a configuration error that names the token's
position, never its value. Without any token and without an application
authorizer, `with_monitor` mounts nothing: `/_rullst/monitor/v1` answers `404`
and requests are handled exactly as without the monitor. So the code can stay
in place and the operator decides per deployment.

`Monitor::with_token` adds a token from your own secret store instead of the
environment. Generated starters do not call `with_monitor` and their
`.env.example` does not list `RULLST_MONITOR_TOKENS`, because the variable
does nothing until the application mounts the monitor.

Read it with the token in the `Authorization` header, never in the URL:

```bash
curl -fsS -H "Authorization: Bearer $RULLST_MONITOR_TOKEN" \
  https://shop.example/_rullst/monitor/v1
```

### Use the application's own sign-in

An administrator who is signed in to the application can read the endpoint
with their session instead of a token, the way Nexus accepts
`NexusAuthPolicy::application`. Implement `MonitorAuthorizer`:

```rust,no_run
use rullst::http::request::Parts;
use rullst::monitor::{Monitor, MonitorAccess, MonitorAuthorizer, MonitorAuthorizerError};

/// Admits requests an outer session layer marked as coming from operators.
struct Operators;

impl MonitorAuthorizer for Operators {
    async fn authorize(&self, request: &Parts) -> Result<MonitorAccess, MonitorAuthorizerError> {
        Ok(match request.headers.get("x-verified-operator") {
            Some(_) => MonitorAccess::Allow,
            None => MonitorAccess::SignIn,
        })
    }
}

let monitor = Monitor::new().with_authorizer(Operators);
```

The authorizer is asked for every request without a valid token. `Allow`
serves the document, `SignIn` answers `401`, `Deny` (a signed-in user without
the permission) answers `403`, and an error answers `503` and logs the
reason. Browsers cannot read the endpoint from another page; see below.

### Report queues

```rust,no_run
# use rullst::monitor::Monitor;
# use std::sync::Arc;
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let queue = Arc::new(rullst::Queue::sqlite("sqlite://jobs.db").await?);
let monitor = Monitor::new().with_queue("default", queue.clone());
# Ok(())
# }
```

Up to 16 queues, named with 1 to 64 letters, digits, `.`, `_` or `-`.

## Put it behind your proxy or VPN

In staging and production the endpoint refuses plain HTTP with `426 Upgrade
Required`, before it looks at credentials. A request is accepted when:

- a reverse proxy listed in `Server::trusted_proxies` (or `[security]
  trusted_proxies`) with `trust_forwarded_proto` reported `https`;
- the application terminates TLS itself and inserts
  `MonitorVerifiedTls::from_trusted_tls_termination()` into the request's
  extensions from its TLS listener; or
- `Monitor::allow_loopback_plaintext(true)` is set and the request comes
  directly from a loopback address, names a loopback `Host` and carries no
  forwarding header: an SSH tunnel (`ssh -L 9000:127.0.0.1:3000 server`) or a
  sidecar on the same host.

A forwarded header from any other peer, or an absolute `https://` request
target, is not evidence. If a token ever crossed a network in plain text,
rotate it.

Exposure is your choice. The safest setups keep the path off the public
internet: allow it only from your VPN (WireGuard, Tailscale) or office ranges
in the reverse proxy, for example with nginx:

```nginx
location /_rullst/monitor/ {
    allow 100.64.0.0/10;   # Tailscale
    deny all;
    proxy_pass http://127.0.0.1:3000;
}
```

## Rotate tokens

`RULLST_MONITOR_TOKENS` holds up to eight comma-separated tokens, and any of
them is accepted. To rotate without downtime:

1. add the new token: `RULLST_MONITOR_TOKENS=<new>,<old>`, and restart or
   redeploy (tokens are read at start-up);
2. switch your monitoring clients to the new token;
3. remove the old one and restart again.

Give each client (each developer machine, each script) its own token, so you
can revoke one by removing it.

## Responses

| Status | When |
| --- | --- |
| `200` | Document served (`Content-Type: application/json`). |
| `401` | No credential, or an invalid one: always the same empty answer, with `WWW-Authenticate: Bearer realm="rullst-monitor"`. |
| `403` | The request carries an `Origin` header or a `Sec-Fetch-Site` of `cross-site`/`same-site` (a browser page), or the authorizer denied it. |
| `404` | The monitor is not mounted. |
| `405` | Any method other than `GET`/`HEAD`. |
| `426` | Staging or production without TLS evidence. |
| `429` | More than 30 requests in a burst (one more each second) from one client, with `Retry-After: 1`; or, after 10 invalid presented tokens, every request from that client until one more attempt is allowed every 90 seconds (`Retry-After: 90`), without evaluating credentials. |
| `503` | The application authorizer failed. |

Every answer carries `Cache-Control: no-store` and
`X-Content-Type-Options: nosniff`; staging and production also add the usual
security headers. The endpoint adds no CORS headers, and because it refuses
any request with an `Origin`, an application CORS allowlist never releases it
to a browser page. Clients are identified per IPv4 address or IPv6 /64, as
resolved through trusted proxies: configure them, or every client behind your
proxy shares one budget and one lockout. These limits are per process. The
server's own rate limiter and Traffic Shield apply to the endpoint like any
route, and the endpoint's own requests are not counted in its statistics.

## What it reports

```json
{
  "schema": "rullst.monitor.v1",
  "generated_at": 1792648800,
  "application": { "name": "shop", "version": "1.4.0" },
  "framework": { "name": "rullst", "version": "13.0.0-alpha.3" },
  "uptime_seconds": 86400,
  "process": {
    "memory_rss_mb": 48.2,
    "cpu_usage_percent": 3.5,
    "active_tokio_tasks": 42,
    "tokio_latency_micros": 18
  },
  "http": {
    "window_seconds": 300,
    "route_limit": 256,
    "routes_overflowed": false,
    "requests_total": 120345,
    "status_total": { "1xx": 0, "2xx": 118000, "3xx": 1200, "4xx": 1100, "5xx": 45 },
    "window": {
      "covered_seconds": 271,
      "requests": 1626,
      "requests_per_second": 6.0,
      "status": { "1xx": 0, "2xx": 1600, "3xx": 10, "4xx": 15, "5xx": 1 },
      "latency_ms": { "p50": 2.5, "p90": 10.0, "p95": 25.0, "p99": 100.0, "max": 412.3 }
    },
    "routes": [
      {
        "method": "GET",
        "route": "/orders/{id}",
        "requests_total": 5400,
        "status_total": { "1xx": 0, "2xx": 5300, "3xx": 0, "4xx": 98, "5xx": 2 },
        "window": { "covered_seconds": 271, "requests": 80, "requests_per_second": 0.295,
                    "status": { "1xx": 0, "2xx": 79, "3xx": 0, "4xx": 1, "5xx": 0 },
                    "latency_ms": { "p50": 5.0, "p90": 10.0, "p95": 10.0, "p99": 25.0, "max": 19.8 } }
      }
    ]
  },
  "database": {
    "state": "observed",
    "queries_total": 9876,
    "n_plus_one": {
      "threshold": 3,
      "findings_total": 12,
      "findings": [
        { "method": "GET", "route": "/orders/{id}", "fingerprint": "LineItem.find (line_items)",
          "requests": 12, "max_occurrences": 25, "last_seen": 1792648790 }
      ]
    }
  },
  "queues": [
    { "name": "default", "pending": { "state": "observed", "count": 3 },
      "in_flight": 1, "completed_total": 5000, "failed_total": 4,
      "retried_total": 2, "stalled_recovered_total": 0 }
  ]
}
```

- **Routes** are keyed by the matched route pattern (`/orders/{id}`) and a
  normalized method (the standard methods, else `OTHER`); requests that matched
  no route are counted as `(unmatched)`. At most 256 method and pattern pairs
  are kept; later ones share `{"method": "*", "route": "(other)"}` and
  `routes_overflowed` becomes `true`.
- **Totals** count since the process started. The **window** covers the last
  five minutes in one-minute slots; `covered_seconds` says how much of it has
  elapsed (less right after start-up), and `requests_per_second` is
  `requests / covered_seconds`, rounded to thousandths.
- **Latency percentiles** come from a fixed histogram (bucket bounds from
  0.1 ms to 10 s): each value is the upper bound of its bucket, capped at the
  window's maximum, and `latency_ms` is `null` without requests in the window.
  Durations run from the outermost framework layer to the response headers.
- **Database** counts the outermost ORM operations started by requests and
  lists repeated operations as the ORM's static `Model.operation (table)`
  fingerprint, never SQL text or values (at most 64 entries, the least
  recently seen replaced first). It needs the framework's tracing subscriber
  (`Server` installs it) and a `RUST_LOG` that keeps `rullst_orm` at `info`;
  otherwise it reports `{"state": "unavailable", "reason":
  "subscriber_not_installed"}` or `"orm_spans_filtered"`.
- **Queues**: `pending` is read from the queue store (all workers, with a
  250 ms limit per queue; `timeout` or `driver_error` otherwise, never the
  driver's message). The other counters cover this process's workers since
  start: running handlers, completed, failed (error, panic, timeout, no
  handler), retried (requeued at shutdown, handed back for lack of a handler,
  or `Queue::retry_failed_job`) and stalled leases recovered.

What it never contains: raw paths, query strings, headers, cookies, client
addresses, user identifiers, request or response bodies, SQL text or bound
values, error messages, environment variables or configuration values. Route
patterns and queue names are chosen by your code, so keep personal data out of
them.

## Limits

- Each process reports itself. Behind a load balancer, each replica has its
  own counters, budgets and lockouts, and a poll reaches one of them; poll
  replicas individually if you need all of them.
- Counters restart with the process.
- The development hot-reload server does not mount the monitor.
- A panic that aborts the connection is not counted.
- The schema is versioned: fields are only added in a new version, served
  next to `v1` at a new path.

## Overhead

Collection runs on every request once the monitor is mounted: it reads the
matched route, takes a read lock on the route table, updates that route's
counters under its own lock, and gives the request a scope in which ORM
operations are counted. Measured on an 8-thread Linux machine in a release
build (`cargo test --release -p rullst-core --lib monitor::overhead_tests --
--ignored --nocapture`): a trivial Axum route took 0.72 µs per request without
the recorder and 1.32 µs with it, so about **0.6 µs per request**; the
statistics update alone took about 96 ns, and 70 to 80 ns per call with 4 to 8
threads recording at once. Memory is bounded: about 1 KiB per route pattern
(273 KiB for 257), 64 N+1 entries, and the endpoint's per-client limiters,
which keep at most 100,000 clients each like Core's `RateLimiter`.
