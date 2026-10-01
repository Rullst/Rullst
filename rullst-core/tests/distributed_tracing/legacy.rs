//! Legacy `init_telemetry` export contracts, each run in a child process
//! because the initializer installs process-global tracing state.

use super::collector::{Collector, Reply};
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use prost::Message;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Spans emitted by the bulk fixture, more than one SDK default batch (512).
pub const BULK_SPANS: usize = 600;
/// Attribute bytes per bulk span: 512 such spans exceed the 1 MiB request
/// limit, while the legacy batch of 64 stays far below it.
pub const BULK_ATTRIBUTE_BYTES: usize = 3_000;

const PROXY_VARIABLES: [&str; 8] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// Runs `fixture` against `endpoint` until the collector has received
/// `expected` spans (or 15 seconds passed), then lets it exit. Returns the
/// spans received and the child's stderr.
async fn run(
    fixture: &str,
    collector: &Collector,
    endpoint: &str,
    environment: &[(&str, &str)],
    expected: usize,
) -> (usize, String) {
    let directory = tempfile::tempdir().unwrap();
    let receipt = directory.path().join("observed");
    let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", fixture, "--nocapture"])
        .env("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT", endpoint)
        .env("OTEL_BSP_SCHEDULE_DELAY", "100")
        .env_remove("OTEL_BSP_MAX_EXPORT_BATCH_SIZE")
        .env("RUST_LOG", "info");
    for variable in PROXY_VARIABLES {
        command.env_remove(variable);
    }
    for (name, value) in environment {
        command.env(name, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
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
    let mut spans = 0;
    let _ = tokio::time::timeout(Duration::from_secs(15), async {
        while spans < expected {
            for request in collector.take().await {
                let batch = ExportTraceServiceRequest::decode(request.body).unwrap();
                spans += batch
                    .resource_spans
                    .iter()
                    .flat_map(|resource| &resource.scope_spans)
                    .map(|scope| scope.spans.len())
                    .sum::<usize>();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    // Give the exporter thread time to handle the collector's reply.
    tokio::time::sleep(Duration::from_millis(300)).await;
    tokio::fs::write(&receipt, b"observed").await.unwrap();
    let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "legacy fixture failed: {stderr}");
    (spans, stderr)
}

#[tokio::test]
async fn legacy_batches_of_large_spans_stay_under_the_request_limit() {
    let collector = Collector::start(Reply::Success).await;
    let (spans, stderr) = run(
        "legacy_bulk_process",
        &collector,
        &collector.endpoint,
        &[],
        BULK_SPANS,
    )
    .await;
    assert_eq!(spans, BULK_SPANS, "stderr: {stderr}");
    assert!(!stderr.contains("dropped a span batch"));
}

#[tokio::test]
async fn legacy_export_honours_the_proxy_environment() {
    let collector = Collector::start(Reply::Success).await;
    // The collector fixture also serves absolute-form proxy requests.
    let proxy = collector
        .endpoint
        .trim_end_matches("/v1/traces")
        .to_string();
    let (spans, stderr) = run(
        "legacy_process",
        &collector,
        "http://legacy-collector.invalid/v1/traces",
        &[("HTTP_PROXY", &proxy), ("http_proxy", &proxy)],
        1,
    )
    .await;
    assert_eq!(spans, 1, "stderr: {stderr}");
}

#[tokio::test]
async fn failed_legacy_exports_are_reported_on_stderr() {
    let collector = Collector::start(Reply::Unavailable).await;
    let (spans, stderr) = run("legacy_process", &collector, &collector.endpoint, &[], 1).await;
    assert_eq!(spans, 1);
    assert!(
        stderr.contains("Rullst telemetry: dropped a span batch"),
        "stderr: {stderr}"
    );
    assert!(!stderr.contains(&collector.endpoint));
}
