# Tracing operations across processes

The unpublished v13 candidate extends Core/facade's optional `telemetry` feature
with `telemetry::distributed`. It propagates an explicitly trusted parent and
exports a minimized operation trace through the maintained OpenTelemetry SDK.
The existing `SpanCollector` stays process-local; Studio's authenticated trace
ingestion remains its separate protocol and is not an OTLP collector.

Local acceptance includes independent Messaging producer/consumer processes and
the standard OpenTelemetry Collector 0.161.0 over verified TLS. Full hosted
workspace, platform, package and release admission remain pending.

## Explicit configuration and lifecycle

```toml
[dependencies]
rullst = { version = "13.0.0-alpha.1", default-features = false, features = ["telemetry"] }
tracing = "0.1.44"
tracing-subscriber = "0.3"
```

The candidate version is not a published installation instruction. Evaluate
reviewed source/extracted packages until release admission completes.

```rust,no_run
# #[cfg(feature = "telemetry")]
fn configure_traces() -> Result<rullst::telemetry::distributed::DistributedTelemetry, Box<dyn std::error::Error>> {
    use rullst::telemetry::distributed::{DistributedTelemetry, TelemetryConfig};
    use tracing_subscriber::prelude::*;

    let config = TelemetryConfig::try_new(
        "academy-web",
        std::env::var("ACADEMY_OTLP_TRACES_URL")?, // https://collector.example/v1/traces
        ["academy.request", "academy.publish", "academy.handle", "rullst.orm.query"],
    )?.with_sampling(0.1)?.require_production()?;
    let runtime = DistributedTelemetry::try_new(config)?;
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(runtime.layer()),
    )?;
    Ok(runtime)
}
```

Retain the returned runtime for the application lifetime. Install its layer once,
before `Server` initializes its legacy subscriber. It composes with an ordinary
`tracing_subscriber` registry and does not replace a global subscriber or provider
automatically. Other formatting/export layers have independent data policies.

Use stable, non-personal service and operation labels: 1–64 ASCII letters,
digits, dots, underscores or hyphens, with 1–64 distinct approved operations.
Operation names must match the actual span names. Do not put an account, email,
tenant, URL, SQL statement or request identifier into those configuration labels.
They are approved by the application, not automatically classified as harmless.

The endpoint is explicit, with the exact `/v1/traces` path and no URL credentials,
query or fragment. `with_bearer_token` accepts an explicit token up to 8 KiB;
`with_ca_certificate` accepts up to 64 KiB of PEM CA material and preserves
certificate/hostname checks. `require_production` rejects offline or HTTP
profiles. `loopback_for_tests` explicitly permits HTTP only at a literal loopback
IP, including a separately protected local collector.

Empty or `mock_*` endpoints select an explicit offline fixture: spans are counted
and discarded without network traffic. `is_mock()` exposes that choice. A failed
live collector never activates it. Offline counters do not prove interoperability
or delivery, and the fixture retains no span payloads.

## Parent trust and async execution

An incoming trace ID is correlation metadata, not proof of identity or tenant
membership. Apply authentication and domain authorization separately. Default
untrusted public ingress to `ParentPolicy::StartNew`; select `TrustedPeer` only
after the host has established the upstream identity and correlation boundary.
This also prevents an untrusted `sampled` flag from overriding root sampling.

```rust,no_run
# #[cfg(feature = "telemetry")]
async fn observed_request(headers: &rullst::http::HeaderMap) -> Result<(), rullst::telemetry::distributed::TelemetryError> {
    use rullst::telemetry::distributed::{ParentPolicy, TraceParent};
    use tracing::Instrument;

    let span = tracing::info_span!(parent: None, "academy.request", otel.kind = "server");
    TraceParent::apply_incoming(&span, headers, ParentPolicy::StartNew)?;
    async {
        // The application's separately authorized asynchronous work goes here.
    }.instrument(span).await;
    Ok(())
}
```

Use `Instrument` to enter spans while their futures are polled; do not retain
an entered-span or thread-local context guard across `.await`. Existing ORM
spans participate when their operation names are approved and the domain future
runs inside the parent span.

`TraceParent` accepts an exact 55-byte, lowercase, nonzero W3C version-00
`traceparent`. Duplicate HTTP values and unsupported/malformed trusted context
return `InvalidContext`; the host can explicitly choose to start a new root for
invalid tracing metadata without changing its authorization decision.
The minimized profile deliberately omits vendor `tracestate` and arbitrary
`baggage`; it is a documented subset, not full W3C forwarding conformance.
`capture` obtains a current span's outgoing parent; `inject` replaces traceparent
and removes those two metadata headers, leaving unrelated application headers
alone. Authorize the destination before sending any headers.

For Messaging, place `TraceContext::try_new(parent.as_str())` into
`PublishRequest::with_trace_context`. The consumer reads the retained envelope's
`trace_context`, validates it with `TraceParent::try_new`, and calls `attach_to`
on its new consumer span only after trusting that producer/topic. A worker's new
span ID remains distinct from the producer's. The
[independent-process fixture](../../rullst/tests/distributed_tracing.rs) executes
this journey through the shared-local SQLite broker and checks a nested ACK
operation. Redis and other validated carriers can carry the same values, but
this particular acceptance does not claim every transport or deployment.

## Metadata and bounded export

The profile exports approved operation/service names, trace/span/parent IDs,
kind, timing and status code. Before queueing, it removes application attributes,
events, links, vendor state and error descriptions. It also avoids ambient
resource detectors, replacing instrumentation-scope metadata with a fixed label.
The SDK span limits disable attribute/event/link retention for this provider.
This boundary does not sanitize other log layers, user-supplied configuration
labels, instrumentation allocations before SDK processing, or application data.

| Export resource | Default and supported limit |
| --- | --- |
| SDK queued spans | 256; configurable 1–4,096 |
| Batch size | 32; configurable 1–128 and no larger than the queue |
| Scheduled export interval | One second; configurable 100 milliseconds–thirty seconds |
| Total network operation | Three seconds; configurable 100 milliseconds–ten seconds |
| Encoded request | At most 1 MiB |
| Collector response | At most 16 KiB, including responses without Content-Length |
| Explicit lifecycle operation | SDK flush/shutdown deadlines; shutdown uses five seconds |

`with_limits` sets queue, batch, interval and network deadline. Export uses the
SDK's dedicated thread and a compatible blocking HTTP client created on that
thread. Request-side span completion never waits for collector I/O; a saturated
queue drops telemetry. These are export-queue bounds, not a quota for every
active application span or all process memory.

The transport disables redirects and ambient proxies, verifies TLS and accepts
only bounded OTLP/HTTP protobuf responses. Malformed data, partial acceptance,
HTTP errors and timeouts count as failed batches. It never logs arbitrary
collector error text. There is no application-side durable spool or automatic
retry of an uncertain batch. Explicit configuration controls endpoint, bearer
token, sampling, resources and queue bounds; ambient SDK header values are not
forwarded by this profile. Incompatible SDK compression settings fail closed.

`status()` exposes metadata-only counters. `forwarded_spans` means offered to the
SDK, so it includes spans later dropped during saturation. `accepted_batches`
means a valid fully accepted OTLP response, not proof of durable backend storage.
`failed_batches` records transport/protocol failures; `offline_spans` records the
explicit fixture; `rejected_spans` records disallowed names/invalid span metadata.
These counters do not form an exact per-span loss accounting system.

After ending the application's active spans, call `flush().await` and eventually
`shutdown().await` from Tokio. These move blocking SDK lifecycle work off the
request executor. Only one explicit flush is admitted at a time. A successful
flush does not recover previously dropped or failed batches; a shutdown timeout
does not prove all traces were delivered. A process kill can lose queued spans.
Operate collector authentication, network policy, sampling budgets, retention,
backups, availability and data access separately. Traces are not an audit ledger.

## Legacy initialization and executable evidence

`init_telemetry` remains the legacy general tracing subscriber. The v13 candidate
repairs its unsupported async-client/thread-processor combination with the same
compatible bounded HTTP transport. Its default is now OTLP/HTTP at
`http://127.0.0.1:4318/v1/traces`, rather than the gRPC port. An explicit
`OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is used exactly; the generic
`OTEL_EXPORTER_OTLP_ENDPOINT` is a base with `/v1/traces` appended. Review existing
environment configuration during migration. Legacy formatting, general span
fields, ambient resource detection and explicit SDK headers keep their separate
policy; they do not acquire the minimized profile automatically. Install the
owned pipeline above when its privacy and lifecycle controls are required.

The [protocol tests](../../rullst-core/tests/distributed_tracing/protocol.rs)
decode real protobuf exports and exercise stripped fields, parent relationships,
queue saturation, malformed/oversized/chunked/partial replies, redirects and
slow/unavailable collectors. The
[standard collector runner](../../.github/check-distributed-tracing.py) uses an
owned digest-pinned container, temporary CA/server identities, verified and
rejected TLS, and actual separate producer/consumer processes. It validates the
ancestry again in the collector's output, rather than accepting client counters
alone. This runs explicitly in the existing Linux workspace and coverage jobs.

The profile is grounded in the [W3C trace-context specification](https://www.w3.org/TR/trace-context/)
and the [OTLP protocol](https://opentelemetry.io/docs/specs/otlp/). Its bounded
subset and local evidence do not certify external tracing accounts, every
OpenTelemetry configuration, production retention or legal compliance.
