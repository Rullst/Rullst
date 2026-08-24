# Rullst Studio: local development control room

Rullst Studio is a developer-facing Axum dashboard. It can run as a standalone
server bound to `127.0.0.1` (port `5555` by default) or its router can be mounted
explicitly by an application.

Studio is not an authentication boundary. Keep it on a loopback or otherwise
trusted interface, and do not expose it publicly without application-level
authentication, authorization, TLS, and network policy.

## Running Studio

The CLI can launch the local server:

```bash
cargo rullst studio
```

The library entry point is also available:

```rust,no_run
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    rullst_studio::run_studio(5555_u16).await
}
```

`Studio::new().into_router()` builds a router for explicit composition. Optional
OpenAPI and queue views are enabled with `with_openapi` and `with_horizon`.

## Current views

- `/studio`: data browser and dashboard shell.
- `/studio/radar`: runtime probes and recorded spans. Unsupported probes display
  `Unavailable`; Studio does not synthesize a healthy value.
- `/studio/security`: counters and events emitted by the in-process security
  store. Audit-chain integrity displays `Unavailable` until a verifier is
  connected.
- `/studio/capital`: the in-process revenue view; it is not an accounting ledger.
- `/studio/traces`: recorded application spans.
- `/studio/migrations`, `/studio/ai`, `/studio/env`, `/studio/features`, and
  `/studio/er`: development tools for their corresponding subsystems.

Some panels poll HTTP JSON endpoints and the request logger uses SSE. The current
crate does not promise a separate WebSocket telemetry transport or zero runtime
overhead.

## Telemetry contract

Studio reads runtime state exposed by `RadarSnapshot`, `SpanCollector`, the
security store, queues, and configured database connections. A counter means only
that the corresponding instrumentation path emitted it; it is not proof that all
traffic passed through that control. Missing sources must remain visibly
unavailable.

## Production boundary

Studio is an optional crate and is not automatically removed merely because a
binary is compiled with `--release`. Exclude it from production features or do
not mount/start it. If an operator intentionally deploys Studio, protect it like
any other privileged administrative interface.
