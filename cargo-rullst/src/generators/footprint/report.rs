//! The versioned footprint report (`rullst.cli-footprint.v1`). Every value
//! carries its method, or the reason it was not measured; keys are always
//! present (null when unknown) so scripts can rely on them.
use super::energy::{Energy, kwh};
use super::sci::Carbon;
use serde::Serialize;

pub(super) const SCHEMA_VERSION: &str = "rullst.cli-footprint.v1";

pub(super) const NOTES: [&str; 4] = [
    "The load generator runs on the same machine; its CPU use is excluded from the app's CPU time but included in RAPL package energy.",
    "Results depend on the hardware, operating system, power settings, build, data and other running processes; compare runs only on the same machine and setup.",
    "Grid intensity (I) and embodied emissions (M) are user-provided inputs; footprint never fetches them.",
    "This is a measurement report, not a certification or a comparison with other software.",
];

/// A value with how it was obtained, or why it was not.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Metric<T> {
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
    pub cli_version: &'static str,
    pub measured_at: String,
    pub machine: Machine,
    pub inputs: Inputs,
    pub target: Target,
    pub load: Load,
    pub process: Process,
    pub artifacts: Artifacts,
    pub energy: EnergyView,
    pub carbon: Carbon,
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
pub(crate) struct Inputs {
    pub url: Option<String>,
    pub path: String,
    pub duration_seconds: f64,
    pub concurrency: u16,
    pub grid_intensity_g_per_kwh: Option<f64>,
    pub cpu_watts: Option<f64>,
    pub embodied_g: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Target {
    /// `release_build` (started by footprint) or `existing_url`.
    pub mode: &'static str,
    pub url: String,
    pub pid: Option<u32>,
    /// How the process was identified, or why it was not.
    pub process_lookup: String,
    pub executable: Option<String>,
    pub environment: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Latency {
    pub p50: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Load {
    pub method: &'static str,
    pub elapsed_seconds: f64,
    pub requests: u64,
    pub errors: u64,
    pub transport_errors: u64,
    pub http_errors: u64,
    pub requests_per_second: Option<f64>,
    pub latency_ms: Latency,
    pub latency_percentile_method: &'static str,
    pub latency_samples: usize,
}

impl Load {
    pub(super) fn from_outcome(outcome: &super::load::Outcome) -> Self {
        Self {
            method: super::load::METHOD,
            elapsed_seconds: outcome.elapsed.as_secs_f64(),
            requests: outcome.responses,
            errors: outcome.errors(),
            transport_errors: outcome.transport_errors,
            http_errors: outcome.http_errors,
            requests_per_second: outcome.requests_per_second(),
            latency_ms: Latency {
                p50: outcome.latency_ms(50.0),
                p95: outcome.latency_ms(95.0),
                p99: outcome.latency_ms(99.0),
            },
            latency_percentile_method: "nearest rank over all completed requests",
            latency_samples: outcome.latencies_us.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Process {
    pub cpu_time_seconds: Metric<f64>,
    pub peak_rss_bytes: Metric<u64>,
    pub idle_rss_bytes: Metric<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Artifacts {
    pub binary_size_bytes: Metric<u64>,
    pub docker_image_size_bytes: Metric<u64>,
}

/// The energy figure flattened for stable keys.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct EnergyView {
    /// `measured`, `estimate` or `not_measured`.
    pub status: &'static str,
    pub joules: Option<f64>,
    pub kwh: Option<f64>,
    pub joules_per_request: Option<f64>,
    pub method: Option<String>,
    pub reason: Option<String>,
    pub rapl_zones: Vec<String>,
    pub cpu_seconds: Option<f64>,
    pub cpu_watts: Option<f64>,
}

impl EnergyView {
    pub(super) fn new(energy: &Energy, requests: u64) -> Self {
        let joules = energy.joules();
        let mut view = Self {
            status: "not_measured",
            joules,
            kwh: joules.map(kwh),
            joules_per_request: joules
                .filter(|_| requests > 0)
                .map(|joules| joules / requests as f64),
            method: None,
            reason: None,
            rapl_zones: Vec::new(),
            cpu_seconds: None,
            cpu_watts: None,
        };
        match energy {
            Energy::Measured { method, zones, .. } => {
                view.status = "measured";
                view.method = Some(method.clone());
                view.rapl_zones = zones.clone();
            }
            Energy::Estimate {
                method,
                cpu_seconds,
                cpu_watts,
                ..
            } => {
                view.status = "estimate";
                view.method = Some(method.clone());
                view.cpu_seconds = Some(*cpu_seconds);
                view.cpu_watts = Some(*cpu_watts);
            }
            Energy::NotMeasured { reason } => view.reason = Some(reason.clone()),
        }
        view
    }
}
