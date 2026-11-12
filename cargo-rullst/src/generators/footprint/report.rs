//! The versioned footprint report (`rullst.footprint.v1`). Measured figures
//! carry their method, or the reason they were not measured; the carbon
//! figure is a separately labelled estimate with its inputs. Keys are always
//! present (null when unknown) so scripts can rely on them.
use super::budget::Budgets;
use super::failure::Failure;
use super::page::PageWeight;
use super::production::Production;
use super::sci::SciEstimate;
use serde::Serialize;

pub(super) const SCHEMA_VERSION: &str = "rullst.footprint.v1";

pub(super) const NOTES: &[&str] = &[
    "Measured figures depend on the hardware, operating system, build profile, data and other running processes; compare runs only on the same machine and setup.",
    "The benchmark load generator runs on the same machine; its CPU use is not part of the app's CPU time.",
    "Response and page bytes are bodies as served over loopback, without headers; a CDN, proxy or another network changes what users receive.",
    "The SCI figure is an estimate computed only from the power, network and grid figures you supply; footprint never fetches carbon data and measures no energy.",
    "This report is a measurement and an estimate, not a certification, an environmental claim or a comparison with other software; if you publish these figures, you are responsible for how you word them.",
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
    pub page_weight: PageWeight,
    /// What the application served in production (`--monitor-url`).
    pub production: Production,
    pub sci_estimate: SciEstimate,
    /// The limits checked with `--check-budget` or `--budget`.
    pub budgets: Budgets,
    pub notes: &'static [&'static str],
    /// Why the app was not measured (`null` when it was): the figures that
    /// need no running app are still reported.
    pub failure: Option<Failure>,
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
            cpu_model: super::process::cpu_model(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct Target {
    /// `release_build` (started by footprint) or `existing_url`.
    pub mode: &'static str,
    /// `null` when the release build failed before a port was chosen.
    pub url: Option<String>,
    pub pid: Option<u32>,
    /// How the process was identified, or why it was not.
    pub process_lookup: String,
    pub executable: Option<String>,
    pub environment: Option<&'static str>,
}

/// Figures read from files, the container engine or the operating system.
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
    /// The `Accept-Encoding` every request sent.
    pub accept_encoding: Option<&'static str>,
    /// Body bytes of the completed responses, as served.
    pub response_bytes: Metric<u64>,
    pub bytes_per_request: Metric<f64>,
    /// The `Content-Encoding` values of the responses (`identity` for none).
    pub content_encodings: Vec<String>,
}

impl Benchmark {
    /// No benchmark, because none was asked for or because of `reason`.
    pub(super) fn not_run(reason: &'static str) -> Self {
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
            cpu_time_seconds: Metric::not_measured("seconds", reason),
            cpu_ms_per_request: Metric::not_measured("milliseconds", reason),
            accept_encoding: None,
            response_bytes: Metric::not_measured("bytes", reason),
            bytes_per_request: Metric::not_measured("bytes", reason),
            content_encodings: Vec::new(),
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
            Ok(seconds) => Metric::measured(seconds, "seconds", super::process::METHODS.cpu),
            Err(reason) => Metric::not_measured("seconds", reason),
        };
        let bytes_per_request = if outcome.responses > 0 {
            Metric::measured(
                outcome.body_bytes as f64 / outcome.responses as f64,
                "bytes",
                "response body bytes / completed requests",
            )
        } else {
            Metric::not_measured("bytes", "no request completed")
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
            accept_encoding: Some(super::load::ACCEPT_ENCODING),
            response_bytes: Metric::measured(
                outcome.body_bytes,
                "bytes",
                "body bytes of the completed responses as served (Content-Encoding kept), without headers",
            ),
            bytes_per_request,
            content_encodings: outcome.encodings.iter().cloned().collect(),
        }
    }
}
