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

/// Reads real RSS memory consumption of the active process in Megabytes.
///
/// Probes exist for Windows (working set) and Linux (`VmRSS`); every other
/// platform, including macOS, returns `None`.
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

/// Reads `VmRSS` from `/proc/self/status`. It is reported in KiB, so unlike
/// `statm` pages it does not depend on the kernel page size (4, 16 or 64 KiB).
#[cfg(target_os = "linux")]
fn get_linux_memory_mb() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let mb = parse_vm_rss_kib(&status)? as f64 / 1024.0;
    Some((mb * 10.0).round() / 10.0)
}

#[cfg(target_os = "linux")]
fn parse_vm_rss_kib(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Sums the aggregate `cpu` line of `/proc/stat` and counts its `cpuN` lines,
/// the host CPUs that aggregate covers.
///
/// Only the first eight columns (`user` to `steal`) are summed: the kernel
/// already counts `guest` and `guest_nice` inside `user` and `nice`.
#[cfg(target_os = "linux")]
fn parse_proc_stat(stat: &str) -> Option<(u64, usize)> {
    let mut aggregate = stat.lines().next()?.split_whitespace();
    if aggregate.next()? != "cpu" {
        return None;
    }
    let total = aggregate.take(8).try_fold(0_u64, |total, value| {
        value
            .parse::<u64>()
            .ok()
            .map(|ticks| total.saturating_add(ticks))
    })?;
    let cpus = stat
        .lines()
        .filter_map(|line| line.split_whitespace().next()?.strip_prefix("cpu"))
        .filter(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        .count();
    Some((total, cpus.max(1)))
}

/// Process CPU time over wall time, in percent of one CPU. `total_delta` spans
/// every host CPU in `/proc/stat`, so it is scaled by the host CPU count, not
/// by the cgroup- or affinity-limited `available_parallelism`.
#[cfg(target_os = "linux")]
fn linux_cpu_percent(process_delta: u64, total_delta: u64, host_cpus: usize) -> Option<f64> {
    if total_delta == 0 {
        return None;
    }
    let host_cpus = host_cpus.max(1) as f64;
    let percent = (process_delta as f64 / total_delta as f64) * host_cpus * 100.0;
    Some(percent.clamp(0.0, host_cpus * 100.0))
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
    let (total_ticks, host_cpus) = parse_proc_stat(&system_stat)?;

    let mut previous = PREVIOUS_CPU_SAMPLE.lock().ok()?;
    let old_sample = previous.replace((process_ticks, total_ticks));
    let (old_process, old_total) = old_sample?;
    linux_cpu_percent(
        process_ticks.saturating_sub(old_process),
        total_ticks.saturating_sub(old_total),
        host_cpus,
    )
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

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_memory_probe_reads_this_process() {
        assert!(get_linux_memory_mb().is_some_and(|mb| mb > 0.0));
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
