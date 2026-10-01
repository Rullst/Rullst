//! Unit tests for Radar telemetry probes and the Prometheus exporter.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_radar_prometheus_metrics_endpoint() {
    init_radar();
    let app = radar_metrics_router();

    let req = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 10000)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("rullst_uptime_seconds"));
    if get_process_memory_mb().is_some() {
        assert!(body_str.contains("rullst_memory_rss_bytes"));
    }
    if get_active_tasks_count().is_some() {
        assert!(body_str.contains("rullst_tokio_active_tasks"));
    }
}

#[test]
fn test_render_prometheus_metrics_all_fields() {
    let snapshot = RadarSnapshot {
        uptime_seconds: 120,
        memory_rss_mb: Some(50.0),
        cpu_usage_percent: Some(2.5),
        active_tokio_tasks: Some(4),
        tokio_latency_micros: Some(15),
        timestamp: 1700000000,
    };
    let metrics = render_prometheus_metrics(&snapshot);
    assert!(metrics.contains("rullst_uptime_seconds 120"));
    assert!(metrics.contains("rullst_memory_rss_bytes 52428800"));
    assert!(metrics.contains("rullst_cpu_usage_percent 2.50"));
    assert!(metrics.contains("rullst_tokio_active_tasks 4"));
    assert!(metrics.contains("rullst_tokio_latency_microseconds 15"));

    let empty_snapshot = RadarSnapshot {
        uptime_seconds: 60,
        memory_rss_mb: None,
        cpu_usage_percent: None,
        active_tokio_tasks: None,
        tokio_latency_micros: None,
        timestamp: 1700000000,
    };
    let empty_metrics = render_prometheus_metrics(&empty_snapshot);
    assert!(empty_metrics.contains("rullst_uptime_seconds 60"));
    assert!(!empty_metrics.contains("rullst_memory_rss_bytes"));
    assert!(!empty_metrics.contains("rullst_cpu_usage_percent"));
    assert!(!empty_metrics.contains("rullst_tokio_active_tasks"));
    assert!(!empty_metrics.contains("rullst_tokio_latency_microseconds"));
}

#[cfg(target_os = "linux")]
#[test]
fn linux_probes_do_not_assume_page_size_or_cgroup_cpu_count() {
    let status = "Name:\tapp\nVmHWM:\t  9000 kB\nVmRSS:\t   65536 kB\n";
    assert_eq!(parse_vm_rss_kib(status), Some(65_536));
    assert_eq!(parse_vm_rss_kib("Name:\tapp\n"), None);

    let mut stat = String::from("cpu  3200 0 0 0 0 0 0 0 0 0\n");
    for cpu in 0..32 {
        stat.push_str(&format!("cpu{cpu} 100 0 0 0 0 0 0 0 0 0\n"));
    }
    stat.push_str("intr 1 2 3\nctxt 4\ncpufreq 5\n");
    assert_eq!(parse_proc_stat(&stat), Some((3200, 32)));
    assert_eq!(parse_proc_stat("intr 1\n"), None);
    // guest (400) and guest_nice (100) are already inside user and nice.
    assert_eq!(
        parse_proc_stat("cpu  1000 50 200 3000 10 5 5 30 400 100\ncpu0 1 0 0 0\n"),
        Some((4300, 1))
    );

    // Two CPUs saturated for one second on a 32-CPU host at 100 Hz.
    assert_eq!(linux_cpu_percent(200, 3200, 32), Some(200.0));
    assert_eq!(linux_cpu_percent(200, 0, 32), None);
    assert_eq!(linux_cpu_percent(u64::MAX, 1, 2), Some(200.0));
}

#[test]
fn a_recorded_boot_time_is_never_replaced() {
    let cell = AtomicU64::new(0);
    record_boot_time_once(&cell);
    let first = cell.load(Ordering::Relaxed);
    assert!(first > 0);
    cell.store(42, Ordering::Relaxed);
    record_boot_time_once(&cell);
    assert_eq!(cell.load(Ordering::Relaxed), 42);
}

#[cfg(target_os = "linux")]
#[test]
fn linux_memory_probe_reads_this_process() {
    assert!(get_linux_memory_mb().is_some_and(|mb| mb > 0.0));
}

#[cfg(target_os = "windows")]
#[test]
fn windows_cpu_percentage_uses_process_time_delta_and_is_bounded() {
    let percent = calculate_windows_cpu_percent(2_500_000, std::time::Duration::from_secs(1), 8);
    assert_eq!(percent, Some(25.0));

    let bounded =
        calculate_windows_cpu_percent(100_000_000, std::time::Duration::from_millis(1), 2);
    assert_eq!(bounded, Some(200.0));
    assert_eq!(
        calculate_windows_cpu_percent(1, std::time::Duration::ZERO, 1),
        None
    );
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn windows_cpu_probe_produces_a_second_sample() {
    let _ = RadarSnapshot::collect();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(RadarSnapshot::collect().cpu_usage_percent.is_some());
}

#[tokio::test]
async fn test_radar_snapshot_collect_and_api() {
    init_radar();
    let snapshot = RadarSnapshot::collect_async().await;
    assert!(snapshot.memory_rss_mb.is_none_or(|memory| memory > 0.0));
    assert!(snapshot.tokio_latency_micros.is_some());
    assert!(snapshot.timestamp > 0);

    let default_snapshot = RadarSnapshot::default();
    assert!(default_snapshot.timestamp > 0);

    let resp = api_radar_handler().await.into_response();
    assert_eq!(resp.status(), StatusCode::OK);
}
