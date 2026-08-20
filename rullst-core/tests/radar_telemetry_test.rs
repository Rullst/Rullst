// tests/radar_telemetry_test.rs — Comprehensive coverage for Radar telemetry, metrics, and Prometheus export.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_core::radar::{
    RadarSnapshot, get_process_memory_mb, init_radar, render_prometheus_metrics,
};
use rullst_core::telemetry_spans::{SpanCollector, TraceSpan};

#[test]
fn test_radar_snapshot_collection_and_prometheus_export() {
    init_radar();

    // Collect radar snapshot (synchronous)
    let snapshot = RadarSnapshot::collect();
    assert!(snapshot.cpu_usage_percent >= 0.0);
    assert!(snapshot.memory_rss_mb >= 0.0);
    assert!(snapshot.uptime_seconds >= 0);

    // Memory query helper
    let mem = get_process_memory_mb();
    assert!(mem >= 0.0);

    // Prometheus metric render output verification
    let prom = render_prometheus_metrics(&snapshot);
    assert!(prom.contains("# HELP rullst_cpu_usage_percent"));
    assert!(prom.contains("# TYPE rullst_cpu_usage_percent gauge"));
    assert!(prom.contains("rullst_cpu_usage_percent"));
    assert!(prom.contains("# HELP rullst_memory_rss_bytes"));
    assert!(prom.contains("rullst_memory_rss_bytes"));
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
