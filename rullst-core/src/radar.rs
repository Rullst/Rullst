//! Rullst Radar — Kernel-Level Telemetry & Tokio Runtime Visualizer (`rullst::radar`)
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

fn get_process_cpu_usage() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        get_linux_process_cpu_usage()
    }

    #[cfg(not(target_os = "linux"))]
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
    )
    ;

    if let Some(memory) = snapshot.memory_rss_mb {
        let _ = write!(metrics, "\n# HELP rullst_memory_rss_bytes Process RSS memory consumption in bytes.\n# TYPE rullst_memory_rss_bytes gauge\nrullst_memory_rss_bytes {}\n", (memory * 1024.0 * 1024.0) as u64);
    }
    if let Some(cpu) = snapshot.cpu_usage_percent {
        let _ = write!(metrics, "\n# HELP rullst_cpu_usage_percent Process CPU utilization percentage.\n# TYPE rullst_cpu_usage_percent gauge\nrullst_cpu_usage_percent {cpu:.2}\n");
    }
    if let Some(tasks) = snapshot.active_tokio_tasks {
        let _ = write!(metrics, "\n# HELP rullst_tokio_active_tasks Total active Tokio tasks count.\n# TYPE rullst_tokio_active_tasks gauge\nrullst_tokio_active_tasks {tasks}\n");
    }
    if let Some(latency) = snapshot.tokio_latency_micros {
        let _ = write!(metrics, "\n# HELP rullst_tokio_latency_microseconds Observed Tokio scheduler yield latency in microseconds.\n# TYPE rullst_tokio_latency_microseconds gauge\nrullst_tokio_latency_microseconds {latency}\n");
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
        assert!(body_str.contains("rullst_memory_rss_bytes"));
        assert!(body_str.contains("rullst_tokio_active_tasks"));
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
