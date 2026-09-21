#![cfg(all(feature = "telemetry", feature = "messaging-sqlite"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]
use rullst::{messaging::*, telemetry::distributed::*};
use serde::{Deserialize, Serialize};
use std::{process::Stdio, time::Duration};
use tokio::io::AsyncWriteExt;
use tracing::Instrument;
use tracing_subscriber::prelude::*;

#[derive(Serialize, Deserialize)]
struct Config {
    role: String,
    endpoint: String,
    ca: Option<String>,
    database: String,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    context: String,
    exported: u64,
    failed: u64,
}

async fn worker(config: Config) -> Receipt {
    let mut telemetry = TelemetryConfig::try_new(
        format!("fixture-{}", config.role),
        &config.endpoint,
        ["academy.schedule", "academy.handle", "academy.persist"],
    )
    .unwrap();
    if let Some(ca) = &config.ca {
        telemetry = telemetry
            .with_ca_certificate(ca.as_bytes().to_vec())
            .unwrap();
    }
    let runtime = DistributedTelemetry::try_new(telemetry.require_production().unwrap()).unwrap();
    let subscriber = tracing_subscriber::registry().with(runtime.layer());
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let broker = SqliteBroker::connect(
        config.database,
        BrokerConfig::try_new("trace-fixture").unwrap(),
    )
    .await
    .unwrap();
    let context = match config.role.as_str() {
        "producer" => {
            broker
                .subscribe(
                    SubscriptionRequest::try_new("lessons", "workers", StartPosition::Earliest)
                        .unwrap(),
                )
                .await
                .unwrap();
            let span = tracing::info_span!(parent:None,"academy.schedule",otel.kind="producer",student="private-student");
            let context = TraceParent::capture(&span).unwrap();
            let carrier = TraceContext::try_new(context.as_str()).unwrap();
            let request = PublishRequest::try_new(
                "lessons",
                "lesson.ready",
                "lesson:42",
                b"private-message-body".to_vec(),
            )
            .unwrap()
            .with_trace_context(&carrier)
            .unwrap();
            broker.publish(request).instrument(span).await.unwrap();
            context
        }
        "consumer" => {
            let messages = broker
                .receive(
                    ReceiveRequest::try_new(
                        "lessons",
                        "workers",
                        "consumer-a",
                        1,
                        Duration::from_secs(10),
                    )
                    .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(messages.len(), 1);
            let carrier = messages[0].envelope().trace_context().unwrap().unwrap();
            let parent = TraceParent::try_new(carrier.traceparent()).unwrap();
            let span = tracing::info_span!(parent:None,"academy.handle",otel.kind="consumer",account="private-account");
            parent.attach_to(&span).unwrap();
            let context = TraceParent::capture(&span).unwrap();
            async {
                let child = tracing::info_span!(
                    "academy.persist",
                    sql = "private-sql",
                    otel.kind = "internal"
                );
                async {
                    broker.ack(messages[0].ack_token()).await.unwrap();
                }
                .instrument(child)
                .await;
            }
            .instrument(span)
            .await;
            context
        }
        _ => panic!("owned fixture role"),
    };
    runtime.flush().await.unwrap();
    let status = runtime.shutdown().await.unwrap();
    Receipt {
        context: context.as_str().to_owned(),
        exported: status.accepted_batches,
        failed: status.failed_batches,
    }
}

#[tokio::test]
#[ignore = "owned producer/consumer subprocess; bounded public fixture input on stdin"]
async fn fixture_process() {
    use std::io::Read;
    let mut input = Vec::new();
    std::io::stdin().take(8192).read_to_end(&mut input).unwrap();
    let config: Config = serde_json::from_slice(&input).unwrap();
    let receipt = worker(config).await;
    println!(
        "RULLST_TRACE_RECEIPT:{}",
        serde_json::to_string(&receipt).unwrap()
    );
}

async fn process(config: Config) -> Receipt {
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "fixture_process", "--nocapture"])
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
        .write_all(&serde_json::to_vec(&config).unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
        .await
        .unwrap()
        .unwrap();
    assert!(
        result.status.success(),
        "owned trace process failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.len() < 16384);
    let receipt = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .find_map(|line| {
            line.strip_prefix("RULLST_TRACE_RECEIPT:")
                .map(str::to_owned)
        })
        .unwrap();
    serde_json::from_str(&receipt).unwrap()
}

#[tokio::test]
#[ignore = "requires owned TLS OpenTelemetry Collector; executed explicitly in Linux CI"]
async fn independent_producer_and_consumer_reach_the_standard_collector() {
    let endpoint = std::env::var("RULLST_TEST_OTLP_ENDPOINT").expect("owned TLS collector");
    let ca = std::fs::read_to_string(std::env::var("RULLST_TEST_OTLP_CA").unwrap()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let database = format!(
        "sqlite:{}?mode=rwc",
        directory.path().join("messaging.db").to_string_lossy()
    );
    let config = |role: &str| Config {
        role: role.to_owned(),
        endpoint: endpoint.clone(),
        ca: Some(ca.clone()),
        database: database.clone(),
    };
    let producer = process(config("producer")).await;
    let consumer = process(config("consumer")).await;
    assert_eq!(&producer.context[3..35], &consumer.context[3..35]);
    assert_ne!(&producer.context[36..52], &consumer.context[36..52]);
    assert!(producer.exported > 0 && consumer.exported > 0);
    assert_eq!(producer.failed + consumer.failed, 0);
    let broker = SqliteBroker::connect(database, BrokerConfig::try_new("trace-fixture").unwrap())
        .await
        .unwrap();
    assert!(
        broker
            .receive(
                ReceiveRequest::try_new(
                    "lessons",
                    "workers",
                    "final-check",
                    1,
                    Duration::from_secs(10)
                )
                .unwrap()
            )
            .await
            .unwrap()
            .is_empty()
    );
}
