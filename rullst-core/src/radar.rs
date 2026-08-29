//! Rullst Radar — process telemetry and Tokio runtime observations (`rullst::radar`).
#![cfg(not(target_arch = "wasm32"))]

use axum::{Json, Router, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static BOOT_TIME: AtomicU64 = AtomicU64::new(0);
static BOOT_INSTANT: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);
#[cfg(target_os = "linux")]
static PREVIOUS_CPU_SAMPLE: std::sync::LazyLock<std::sync::Mutex<Option<(u64, u64)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));
#[cfg(target_os = "windows")]
static PREVIOUS_WINDOWS_CPU_SAMPLE: std::sync::LazyLock<
    std::sync::Mutex<Option<(u64, std::time::Instant)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// Initializes the Radar boot time timestamp.
pub fn init_radar() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    BOOT_TIME.store(now, Ordering::Relaxed);
}

/// Instantaneous telemetry snapshot data model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadarSnapshot {
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// RSS memory consumption in MB, or `None` when the platform probe is unavailable.
    pub memory_rss_mb: Option<f64>,
    /// Process CPU utilization, or `None` until a real sample can be calculated.
    pub cpu_usage_percent: Option<f64>,
    /// Active Tokio tasks, or `None` when collection occurs outside a Tokio runtime.
    pub active_tokio_tasks: Option<usize>,
    /// Observed Tokio yield latency, populated by [`RadarSnapshot::collect_async`].
    pub tokio_latency_micros: Option<u64>,
    /// Unix timestamp of snapshot generation.
    pub timestamp: u64,
}

impl Default for RadarSnapshot {
    fn default() -> Self {
        Self::collect()
    }
}

impl RadarSnapshot {
    /// Collects live process telemetry.
    pub fn collect() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let boot = BOOT_TIME.load(Ordering::Relaxed);
        let uptime = if boot > 0 && now >= boot {
            now - boot
        } else {
            BOOT_INSTANT.elapsed().as_secs()
        };

        let memory_rss_mb = get_process_memory_mb();
        let cpu_usage_percent = get_process_cpu_usage();

        Self {
            uptime_seconds: uptime,
            memory_rss_mb,
            cpu_usage_percent,
            active_tokio_tasks: get_active_tasks_count(),
            tokio_latency_micros: None,
            timestamp: now,
        }
    }

    /// Collects a snapshot and measures one real Tokio scheduler yield.
    pub async fn collect_async() -> Self {
        let started = std::time::Instant::now();
        tokio::task::yield_now().await;
        let latency = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let mut snapshot = Self::collect();
        snapshot.tokio_latency_micros = Some(latency);
        snapshot
    }
}

/// Reads real RSS memory consumption of the active process in Megabytes (Windows, Linux, macOS).
pub fn get_process_memory_mb() -> Option<f64> {
    #[cfg(target_os = "windows")]
    {
        if let Some(mb) = get_windows_memory_mb() {
            return Some(mb);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(mb) = get_linux_memory_mb() {
            return Some(mb);
        }
    }

    None
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn get_windows_memory_mb() -> Option<f64> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: `GetCurrentProcess` returns a pseudo-handle valid in this process;
    // `counters` points to initialized writable storage of the exact size passed
    // to `K32GetProcessMemoryInfo`, and is not retained after this call.
    unsafe {
        let process = GetCurrentProcess();
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if K32GetProcessMemoryInfo(process, &mut counters, cb) != 0 {
            let bytes = counters.WorkingSetSize;
            let mb = (bytes as f64) / (1024.0 * 1024.0);
            return Some((mb * 10.0).round() / 10.0);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_linux_memory_mb() -> Option<f64> {
    if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(pages) = parts[1].parse::<u64>() {
                let bytes = pages * 4096;
                let mb = (bytes as f64) / (1024.0 * 1024.0);
                return Some((mb * 10.0).round() / 10.0);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_process_cpu_usage() -> Option<f64> {
    get_linux_process_cpu_usage()
}

#[cfg(target_os = "windows")]
fn get_process_cpu_usage() -> Option<f64> {
    get_windows_process_cpu_usage()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_process_cpu_usage() -> Option<f64> {
    None
}

#[cfg(target_os = "linux")]
fn get_linux_process_cpu_usage() -> Option<f64> {
    let process_stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_name = process_stat.get(process_stat.rfind(')')?.saturating_add(2)..)?;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    let process_ticks = fields
        .get(11)?
        .parse::<u64>()
        .ok()?
        .saturating_add(fields.get(12)?.parse::<u64>().ok()?);

    let system_stat = std::fs::read_to_string("/proc/stat").ok()?;
    let total_ticks = system_stat
        .lines()
        .next()?
        .split_whitespace()
        .skip(1)
        .try_fold(0_u64, |total, value| {
            value
                .parse::<u64>()
                .ok()
                .map(|ticks| total.saturating_add(ticks))
        })?;

    let mut previous = PREVIOUS_CPU_SAMPLE.lock().ok()?;
    let old_sample = previous.replace((process_ticks, total_ticks));
    let (old_process, old_total) = old_sample?;
    let total_delta = total_ticks.saturating_sub(old_total);
    if total_delta == 0 {
        return None;
    }

    let process_delta = process_ticks.saturating_sub(old_process);
    let logical_cpus = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    let percent = (process_delta as f64 / total_delta as f64) * logical_cpus as f64 * 100.0;
    Some(percent.clamp(0.0, logical_cpus as f64 * 100.0))
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn get_windows_process_cpu_usage() -> Option<f64> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let sampled_at = std::time::Instant::now();
    let process_time_100ns = unsafe {
        // SAFETY: `GetCurrentProcess` returns a pseudo-handle valid in this process. Every
        // `FILETIME` pointer references initialized writable storage for the duration of the
        // call, and Windows does not retain any pointer after `GetProcessTimes` returns.
        let process = GetCurrentProcess();
        let mut creation: FILETIME = std::mem::zeroed();
        let mut exit: FILETIME = std::mem::zeroed();
        let mut kernel: FILETIME = std::mem::zeroed();
        let mut user: FILETIME = std::mem::zeroed();
        if GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) == 0 {
            return None;
        }
        filetime_100ns(kernel).saturating_add(filetime_100ns(user))
    };

    let mut previous = PREVIOUS_WINDOWS_CPU_SAMPLE.lock().ok()?;
    let old_sample = previous.replace((process_time_100ns, sampled_at));
    let (old_process_time_100ns, old_sampled_at) = old_sample?;
    calculate_windows_cpu_percent(
        process_time_100ns.saturating_sub(old_process_time_100ns),
        sampled_at.saturating_duration_since(old_sampled_at),
        std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1),
    )
}

#[cfg(target_os = "windows")]
fn filetime_100ns(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

#[cfg(target_os = "windows")]
fn calculate_windows_cpu_percent(
    process_delta_100ns: u64,
    wall_elapsed: std::time::Duration,
    logical_cpus: usize,
) -> Option<f64> {
    let wall_seconds = wall_elapsed.as_secs_f64();
    if wall_seconds <= f64::EPSILON {
        return None;
    }

    let process_seconds = process_delta_100ns as f64 / 10_000_000.0;
    let max_percent = logical_cpus.max(1) as f64 * 100.0;
    Some(((process_seconds / wall_seconds) * 100.0).clamp(0.0, max_percent))
}

fn get_active_tasks_count() -> Option<usize> {
    tokio::runtime::Handle::try_current()
        .ok()
        .map(|handle| handle.metrics().num_alive_tasks())
}

/// Formats the current `RadarSnapshot` into Prometheus text format for `/metrics` scraping.
pub fn render_prometheus_metrics(snapshot: &RadarSnapshot) -> String {
    use std::fmt::Write as _;

    let mut metrics = format!(
        r###"# HELP rullst_uptime_seconds Process uptime in seconds.
# TYPE rullst_uptime_seconds counter
rullst_uptime_seconds {}
"###,
        snapshot.uptime_seconds
    );

    if let Some(memory) = snapshot.memory_rss_mb {
        let _ = write!(
            metrics,
            "\n# HELP rullst_memory_rss_bytes Process RSS memory consumption in bytes.\n# TYPE rullst_memory_rss_bytes gauge\nrullst_memory_rss_bytes {}\n",
            (memory * 1024.0 * 1024.0) as u64
        );
    }
    if let Some(cpu) = snapshot.cpu_usage_percent {
        let _ = write!(
            metrics,
            "\n# HELP rullst_cpu_usage_percent Process CPU utilization percentage.\n# TYPE rullst_cpu_usage_percent gauge\nrullst_cpu_usage_percent {cpu:.2}\n"
        );
    }
    if let Some(tasks) = snapshot.active_tokio_tasks {
        let _ = write!(
            metrics,
            "\n# HELP rullst_tokio_active_tasks Total active Tokio tasks count.\n# TYPE rullst_tokio_active_tasks gauge\nrullst_tokio_active_tasks {tasks}\n"
        );
    }
    if let Some(latency) = snapshot.tokio_latency_micros {
        let _ = write!(
            metrics,
            "\n# HELP rullst_tokio_latency_microseconds Observed Tokio scheduler yield latency in microseconds.\n# TYPE rullst_tokio_latency_microseconds gauge\nrullst_tokio_latency_microseconds {latency}\n"
        );
    }
    metrics
}

/// Endpoint handler for Prometheus `/metrics` scraper.
pub async fn prometheus_metrics_handler() -> impl IntoResponse {
    let snapshot = RadarSnapshot::collect_async().await;
    let text = render_prometheus_metrics(&snapshot);
    (
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        text,
    )
}

/// Endpoint handler for JSON Radar snapshot (`GET /api/radar`).
pub async fn api_radar_handler() -> impl IntoResponse {
    Json(RadarSnapshot::collect_async().await)
}

/// Returns an Axum `Router` mounting the Prometheus `/metrics` exporter endpoint.
pub fn radar_metrics_router() -> Router {
    Router::new().route("/metrics", get(prometheus_metrics_handler))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
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

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cpu_percentage_uses_process_time_delta_and_is_bounded() {
        let percent =
            calculate_windows_cpu_percent(2_500_000, std::time::Duration::from_secs(1), 8);
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
}
