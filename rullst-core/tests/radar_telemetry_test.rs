// tests/radar_telemetry_test.rs — Comprehensive coverage for Radar telemetry, metrics, and Prometheus export.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::radar::{
    RadarSnapshot, get_process_memory_mb, init_radar, render_prometheus_metrics,
};
use rullst_core::telemetry_spans::{SpanCollector, TraceSpan};

#[tokio::test]
async fn test_radar_snapshot_collection_and_prometheus_export() {
    init_radar();

    let snapshot = RadarSnapshot::collect_async().await;
    assert!(snapshot.cpu_usage_percent.is_none_or(|cpu| cpu >= 0.0));
    assert!(snapshot.memory_rss_mb.is_none_or(|memory| memory >= 0.0));
    assert!(snapshot.active_tokio_tasks.is_some());
    assert!(snapshot.tokio_latency_micros.is_some());

    // Memory query helper
    let mem = get_process_memory_mb();
    assert!(mem.is_none_or(|memory| memory >= 0.0));

    // Prometheus metric render output verification
    let prom = render_prometheus_metrics(&snapshot);
    assert_eq!(
        prom.contains("rullst_cpu_usage_percent"),
        snapshot.cpu_usage_percent.is_some()
    );
    assert_eq!(
        prom.contains("rullst_memory_rss_bytes"),
        snapshot.memory_rss_mb.is_some()
    );
    assert!(prom.contains("# HELP rullst_uptime_seconds"));
    assert!(prom.contains("rullst_uptime_seconds"));
    assert!(prom.contains("# HELP rullst_tokio_latency_microseconds"));
    assert!(prom.contains("rullst_tokio_latency_microseconds"));
    assert!(prom.contains("# HELP rullst_tokio_active_tasks"));
    assert!(prom.contains("rullst_tokio_active_tasks"));
}

#[test]
fn test_span_collector_lifecycle() {
    let collector = SpanCollector::new(10);
    assert!(collector.snapshot().is_empty());

    // Record spans
    collector.record(TraceSpan {
        name: "db_query".to_string(),
        kind: "sql".to_string(),
        duration_us: 1250,
        timestamp: 1700000000,
    });

    collector.record(TraceSpan {
        name: "http_request".to_string(),
        kind: "http".to_string(),
        duration_us: 4500,
        timestamp: 1700000001,
    });

    let spans = collector.snapshot();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].name, "db_query");
    assert_eq!(spans[1].name, "http_request");
}
