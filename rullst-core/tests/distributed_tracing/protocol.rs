use super::collector::{Collector, Reply};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use rullst_core::telemetry::distributed::*;
use std::time::{Duration, Instant};
use tracing_subscriber::prelude::*;

fn configured(collector: &Collector) -> DistributedTelemetry {
    DistributedTelemetry::try_new(
        TelemetryConfig::loopback_for_tests(
            "fixture-service",
            &collector.endpoint,
            ["fixture.request", "fixture.publish"],
        )
        .unwrap()
        .with_bearer_token("public-fixture-token")
        .unwrap()
        .with_limits(16, 8, Duration::from_secs(30), Duration::from_millis(250))
        .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn legacy_initializer_exports_from_a_tokio_application_with_the_compatible_client() {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    let collector = Collector::start(Reply::Success).await;
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("observed");
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "legacy_process", "--nocapture"])
        .env("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", &collector.endpoint)
        .env("OTEL_BSP_SCHEDULE_DELAY", "100")
        .env("RUST_LOG", "info")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&receipt).unwrap())
        .await
        .unwrap();
    let received = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let received = collector.take().await;
            if !received.is_empty() {
                break received;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let batch = ExportTraceServiceRequest::decode(received[0].body.clone()).unwrap();
    assert!(
        batch
            .resource_spans
            .iter()
            .flat_map(|resource| &resource.scope_spans)
            .flat_map(|scope| &scope.spans)
            .any(|span| span.name == "legacy.fixture")
    );
    tokio::fs::write(&receipt, b"collector received legacy span")
        .await
        .unwrap();
    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        output.status.success(),
        "legacy fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn real_otlp_http_exports_relationships_without_sensitive_fields() {
    let collector = Collector::start(Reply::Success).await;
    let runtime = configured(&collector);
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    let (root_context, child_context) = tracing::subscriber::with_default(subscriber, || {
        let root = tracing::info_span!(parent:None,"fixture.request",password="private-span-value",otel.kind="server");
        let root_context = TraceParent::capture(&root).unwrap();
        let child = tracing::info_span!(parent:&root,"fixture.publish",email="private-email",otel.kind="producer");
        child.in_scope(|| tracing::error!("private-event-message"));
        let child_context = TraceParent::capture(&child).unwrap();
        (root_context, child_context)
    });
    runtime.flush().await.unwrap();
    assert_eq!(runtime.status().accepted_batches, 1);
    let received = collector.take().await;
    assert_eq!(received.len(), 1);
    assert_eq!(
        received[0].headers.get("authorization").unwrap(),
        "Bearer public-fixture-token"
    );
    assert!(!received[0].body.windows(7).any(|bytes| bytes == b"private"));
    let batch = ExportTraceServiceRequest::decode(received[0].body.clone()).unwrap();
    let resource = &batch.resource_spans[0];
    let attributes = &resource.resource.as_ref().unwrap().attributes;
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].key, "service.name");
    let spans = &resource.scope_spans[0].spans;
    assert_eq!(spans.len(), 2);
    let root = spans
        .iter()
        .find(|span| span.name == "fixture.request")
        .unwrap();
    let child = spans
        .iter()
        .find(|span| span.name == "fixture.publish")
        .unwrap();
    assert_eq!(root.trace_id, child.trace_id);
    assert_eq!(root.span_id, child.parent_span_id);
    assert_ne!(root.span_id, child.span_id);
    assert_eq!(
        &root_context.as_str()[3..35],
        &child_context.as_str()[3..35]
    );
    for span in spans {
        assert!(span.attributes.is_empty());
        assert!(span.events.is_empty());
        assert!(span.links.is_empty());
        assert!(span.trace_state.is_empty());
        assert!(
            span.status
                .as_ref()
                .is_none_or(|status| status.message.is_empty())
        );
    }
    assert_eq!(runtime.shutdown().await.unwrap().failed_batches, 0);
}

#[tokio::test]
async fn collector_failures_are_bounded_observed_and_never_downgrade_to_mock() {
    for reply in [
        Reply::Malformed,
        Reply::Oversized,
        Reply::Chunked,
        Reply::Partial,
        Reply::Unavailable,
        Reply::Redirect,
        Reply::Slow,
    ] {
        let collector = Collector::start(reply.clone()).await;
        let runtime = configured(&collector);
        let subscriber = tracing_subscriber::registry().with(runtime.layer());
        tracing::subscriber::with_default(subscriber, || {
            let _span = tracing::info_span!("fixture.request");
        });
        let began = Instant::now();
        assert!(runtime.flush().await.is_err(), "{reply:?} must fail flush");
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "export deadline for {reply:?}"
        );
        assert!(!runtime.is_mock());
        assert_eq!(runtime.status().failed_batches, 1);
        assert_eq!(runtime.status().accepted_batches, 0);
        assert_eq!(runtime.status().offline_spans, 0);
        assert_eq!(
            collector.take().await.len(),
            1,
            "no automatic retry or redirect for {reply:?}"
        );
        runtime.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn queue_pressure_never_blocks_request_side_span_completion() {
    let collector = Collector::start(Reply::Held).await;
    let runtime = DistributedTelemetry::try_new(
        TelemetryConfig::loopback_for_tests(
            "fixture-service",
            &collector.endpoint,
            ["fixture.request"],
        )
        .unwrap()
        .with_limits(2, 1, Duration::from_secs(30), Duration::from_secs(3))
        .unwrap(),
    )
    .unwrap();
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    let began = Instant::now();
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..1000 {
            let _span = tracing::info_span!("fixture.request", body = "private-value");
        }
    });
    assert!(
        began.elapsed() < Duration::from_secs(2),
        "emission must not await slow collector I/O"
    );
    assert_eq!(runtime.status().forwarded_spans, 1000);
    collector.release();
    runtime.flush().await.unwrap();
    let status = runtime.shutdown().await.unwrap();
    assert!(
        (1..=5).contains(&status.accepted_batches),
        "bounded queue must shed excess telemetry: {status:?}"
    );
    assert_eq!(status.failed_batches, 0);
}

#[tokio::test]
#[ignore = "requires owned TLS OpenTelemetry Collector; executed explicitly in Linux CI"]
async fn verified_ca_succeeds_while_untrusted_ca_and_wrong_hostname_fail() {
    let endpoint = std::env::var("RULLST_TEST_OTLP_ENDPOINT").expect("owned TLS collector");
    let ca = std::fs::read(std::env::var("RULLST_TEST_OTLP_CA").unwrap()).unwrap();
    for (endpoint, trust, success) in [
        (endpoint.clone(), false, false),
        (endpoint.replace("localhost", "127.0.0.1"), true, false),
        (endpoint, true, true),
    ] {
        let mut config = TelemetryConfig::try_new("fixture-tls", endpoint, ["fixture.request"])
            .unwrap()
            .with_limits(16, 8, Duration::from_secs(30), Duration::from_secs(3))
            .unwrap();
        if trust {
            config = config.with_ca_certificate(ca.clone()).unwrap();
        }
        let runtime = DistributedTelemetry::try_new(config.require_production().unwrap()).unwrap();
        let subscriber = tracing_subscriber::registry().with(runtime.layer());
        tracing::subscriber::with_default(subscriber, || {
            let _span = tracing::info_span!("fixture.request");
        });
        assert_eq!(runtime.flush().await.is_ok(), success);
        assert_eq!(runtime.status().accepted_batches, u64::from(success));
        assert_eq!(runtime.status().failed_batches, u64::from(!success));
        runtime.shutdown().await.unwrap();
    }
}
