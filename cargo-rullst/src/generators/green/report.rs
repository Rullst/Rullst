//! The versioned green report (`rullst.green.v1`). Measured figures carry
//! their method, or the reason they were not measured; the carbon figure is
//! a separately labelled estimate with its inputs. Keys are always present
//! (null when unknown) so scripts can rely on them.
use super::sci::SciEstimate;
use serde::Serialize;

pub(super) const SCHEMA_VERSION: &str = "rullst.green.v1";

pub(super) const NOTES: [&str; 4] = [
    "Measured figures depend on the hardware, operating system, build profile, data and other running processes; compare runs only on the same machine and setup.",
    "The benchmark load generator runs on the same machine; its CPU use is not part of the app's CPU time.",
    "The SCI figure is an estimate computed only from the power figure and grid intensity you supply; green never fetches carbon data and measures no energy.",
    "This is a measurement report, not a certification or a comparison with other software.",
];

/// A measured value with how it was obtained, or why it was not.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Metric<T> {
    /// `measured` or `not_measured`.
    pub status: &'static str,
    pub value: Option<T>,
    pub unit: &'static str,
    pub method: Option<String>,
    pub reason: Option<String>,
}

impl<T> Metric<T> {
    pub(super) fn measured(value: T, unit: &'static str, method: impl Into<String>) -> Self {
        Self {
            status: "measured",
            value: Some(value),
            unit,
            method: Some(method.into()),
            reason: None,
        }
    }

    pub(super) fn not_measured(unit: &'static str, reason: impl Into<String>) -> Self {
        Self {
            status: "not_measured",
            value: None,
            unit,
            method: None,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Report {
    pub schema_version: &'static str,
    pub schema: &'static str,
    pub cli_version: &'static str,
    pub measured_at: String,
    pub machine: Machine,
    pub target: Target,
    pub measured: Measured,
    pub benchmark: Benchmark,
    pub sci_estimate: SciEstimate,
    pub notes: [&'static str; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Machine {
    pub os: &'static str,
    pub arch: &'static str,
    pub logical_cpus: Option<usize>,
    pub cpu_model: Option<String>,
}

impl Machine {
    pub(super) fn current() -> Self {
        Self {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            logical_cpus: std::thread::available_parallelism()
                .ok()
                .map(std::num::NonZeroUsize::get),
            cpu_model: super::procfs::cpu_model(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Target {
    /// `release_build` (started by green) or `existing_url`.
    pub mode: &'static str,
    pub url: String,
    pub pid: Option<u32>,
    /// How the process was identified, or why it was not.
    pub process_lookup: String,
    pub executable: Option<String>,
    pub environment: Option<&'static str>,
}

/// Figures read from files, the container engine or `/proc`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Measured {
    pub binary_size_bytes: Metric<u64>,
    pub image_size_bytes: Metric<u64>,
    pub idle_rss_bytes: Metric<u64>,
    pub dependency_count: Metric<u64>,
}

/// The optional bounded load and the app's CPU time during it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Benchmark {
    /// `run` or `not_run` (without `--bench`).
    pub status: &'static str,
    pub method: Option<&'static str>,
    pub path: Option<String>,
    pub duration_seconds: Option<f64>,
    pub concurrency: Option<u16>,
    pub elapsed_seconds: Option<f64>,
    pub requests: Option<u64>,
    pub transport_errors: Option<u64>,
    pub http_errors: Option<u64>,
    pub cpu_time_seconds: Metric<f64>,
    pub cpu_ms_per_request: Metric<f64>,
}

impl Benchmark {
    pub(super) fn not_run() -> Self {
        const REASON: &str = "no benchmark ran; pass --bench";
        Self {
            status: "not_run",
            method: None,
            path: None,
            duration_seconds: None,
            concurrency: None,
            elapsed_seconds: None,
            requests: None,
            transport_errors: None,
            http_errors: None,
            cpu_time_seconds: Metric::not_measured("seconds", REASON),
            cpu_ms_per_request: Metric::not_measured("milliseconds", REASON),
        }
    }

    /// A finished run; `cpu_seconds` is the app's CPU time during it, or
    /// why that was not measured.
    pub(super) fn run(
        path: &str,
        duration_seconds: f64,
        concurrency: u16,
        outcome: &super::load::Outcome,
        cpu_seconds: Result<f64, &'static str>,
    ) -> Self {
        let cpu_time_seconds = match cpu_seconds {
            Ok(seconds) => Metric::measured(
                seconds,
                "seconds",
                "utime + stime of the app process during the load (/proc/<pid>/stat)",
            ),
            Err(reason) => Metric::not_measured("seconds", reason),
        };
        let cpu_ms_per_request = match (cpu_seconds, outcome.responses) {
            (Ok(seconds), requests) if requests > 0 => Metric::measured(
                seconds * 1000.0 / requests as f64,
                "milliseconds",
                "CPU time / completed requests",
            ),
            (Ok(_), _) => Metric::not_measured("milliseconds", "no request completed"),
            (Err(reason), _) => Metric::not_measured("milliseconds", reason),
        };
        Self {
            status: "run",
            method: Some(super::load::METHOD),
            path: Some(path.to_string()),
            duration_seconds: Some(duration_seconds),
            concurrency: Some(concurrency),
            elapsed_seconds: Some(outcome.elapsed.as_secs_f64()),
            requests: Some(outcome.responses),
            transport_errors: Some(outcome.transport_errors),
            http_errors: Some(outcome.http_errors),
            cpu_time_seconds,
            cpu_ms_per_request,
        }
    }
}
