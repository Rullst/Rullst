//! The `rullst.monitor.v1` document as Studio reads it from a remote project.
//!
//! The document comes from another machine, so it is untrusted: unknown
//! fields are ignored (later minor additions), every list and label is
//! bounded by [`MonitorDocument::validate`], and pages escape every string.

use serde::Deserialize;

/// The only schema this Studio reads.
pub const MONITOR_SCHEMA: &str = "rullst.monitor.v1";
const MAX_ROUTES: usize = 512;
const MAX_FINDINGS: usize = 128;
const MAX_QUEUES: usize = 32;
const MAX_LABEL_CHARS: usize = 512;

/// A monitoring document.
#[derive(Debug, Clone, Deserialize)]
pub struct MonitorDocument {
    /// Always [`MONITOR_SCHEMA`].
    pub schema: String,
    /// Unix time of generation, in seconds.
    pub generated_at: u64,
    /// The application's name and version, when it reports them.
    pub application: Option<Application>,
    /// The framework's name and version.
    pub framework: Framework,
    /// Process uptime in seconds.
    pub uptime_seconds: u64,
    /// The process radar.
    pub process: Process,
    /// Request statistics.
    pub http: Http,
    /// ORM operation counts and possible N+1 findings.
    pub database: Database,
    /// Registered queues.
    pub queues: Vec<Queue>,
}

/// Application name and version.
#[derive(Debug, Clone, Deserialize)]
pub struct Application {
    /// Name.
    pub name: String,
    /// Version.
    pub version: String,
}

/// Framework name and version.
#[derive(Debug, Clone, Deserialize)]
pub struct Framework {
    /// Name.
    pub name: String,
    /// Version.
    pub version: String,
}

/// Process radar values.
#[derive(Debug, Clone, Deserialize)]
pub struct Process {
    /// Resident memory in MiB.
    pub memory_rss_mb: Option<f64>,
    /// CPU use in percent of one CPU.
    pub cpu_usage_percent: Option<f64>,
    /// Live Tokio tasks.
    pub active_tokio_tasks: Option<u64>,
    /// Observed scheduler yield latency.
    pub tokio_latency_micros: Option<u64>,
}

/// Responses per status class.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct StatusClasses {
    /// `1xx`.
    #[serde(rename = "1xx")]
    pub informational: u64,
    /// `2xx`.
    #[serde(rename = "2xx")]
    pub success: u64,
    /// `3xx`.
    #[serde(rename = "3xx")]
    pub redirection: u64,
    /// `4xx`.
    #[serde(rename = "4xx")]
    pub client_error: u64,
    /// `5xx`.
    #[serde(rename = "5xx")]
    pub server_error: u64,
}

/// Latency percentiles in milliseconds.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Latency {
    /// Median.
    pub p50: f64,
    /// 90th percentile.
    pub p90: f64,
    /// 95th percentile.
    pub p95: f64,
    /// 99th percentile.
    pub p99: f64,
    /// Maximum.
    pub max: f64,
}

/// The recent window of one route or of the whole application.
#[derive(Debug, Clone, Deserialize)]
pub struct Window {
    /// Seconds the window covers.
    pub covered_seconds: u64,
    /// Requests in the window.
    pub requests: u64,
    /// Requests per second over the covered time.
    pub requests_per_second: f64,
    /// Status classes in the window.
    pub status: StatusClasses,
    /// Percentiles, `None` without requests.
    pub latency_ms: Option<Latency>,
}

/// Request statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct Http {
    /// Window length.
    pub window_seconds: u64,
    /// Route patterns kept by the server.
    pub route_limit: u64,
    /// Whether later patterns share the `(other)` bucket.
    pub routes_overflowed: bool,
    /// Requests since start.
    pub requests_total: u64,
    /// Status classes since start.
    pub status_total: StatusClasses,
    /// The application-wide window.
    pub window: Window,
    /// Per route pattern and method.
    pub routes: Vec<Route>,
}

/// One route pattern and method.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    /// Normalized method.
    pub method: String,
    /// Route pattern.
    pub route: String,
    /// Requests since start.
    pub requests_total: u64,
    /// Status classes since start.
    pub status_total: StatusClasses,
    /// The route's window.
    pub window: Window,
}

/// ORM observations.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Database {
    /// ORM operations are observed.
    Observed {
        /// Outermost ORM operations started by requests.
        queries_total: u64,
        /// Possible N+1 findings.
        n_plus_one: NPlusOne,
    },
    /// ORM operations cannot be observed in that process.
    Unavailable {
        /// A fixed reason.
        reason: String,
    },
}

/// Possible N+1 findings.
#[derive(Debug, Clone, Deserialize)]
pub struct NPlusOne {
    /// Repetitions within one request that count.
    pub threshold: u64,
    /// Findings since start.
    pub findings_total: u64,
    /// Kept findings, most recent first.
    pub findings: Vec<Finding>,
}

/// One repeated ORM operation.
#[derive(Debug, Clone, Deserialize)]
pub struct Finding {
    /// Method.
    pub method: String,
    /// Route pattern.
    pub route: String,
    /// ORM label fingerprint.
    pub fingerprint: String,
    /// Requests in which it repeated.
    pub requests: u64,
    /// Most repetitions in one request.
    pub max_occurrences: u64,
    /// Unix time of the latest occurrence.
    pub last_seen: u64,
}

/// One registered queue.
#[derive(Debug, Clone, Deserialize)]
pub struct Queue {
    /// Name.
    pub name: String,
    /// Pending jobs in the store.
    pub pending: Pending,
    /// Running handlers in the reporting process.
    pub in_flight: u64,
    /// Completed by the reporting process.
    pub completed_total: u64,
    /// Failed in the reporting process.
    pub failed_total: u64,
    /// Returned to pending by the reporting process.
    pub retried_total: u64,
    /// Stalled leases recovered by the reporting process.
    pub stalled_recovered_total: u64,
}

/// A queue's pending count.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Pending {
    /// Read from the store.
    Observed {
        /// Pending jobs.
        count: u64,
    },
    /// The store did not answer in time or failed.
    Unavailable {
        /// A fixed reason.
        reason: String,
    },
}

impl MonitorDocument {
    /// Checks the schema and the bounds of every list and label.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != MONITOR_SCHEMA {
            return Err("unsupported schema");
        }
        if self.http.routes.len() > MAX_ROUTES || self.queues.len() > MAX_QUEUES {
            return Err("too many entries");
        }
        let mut labels: Vec<&str> = vec![&self.framework.name, &self.framework.version];
        if let Some(application) = &self.application {
            labels.extend([application.name.as_str(), application.version.as_str()]);
        }
        for route in &self.http.routes {
            labels.extend([route.method.as_str(), route.route.as_str()]);
        }
        match &self.database {
            Database::Observed { n_plus_one, .. } => {
                if n_plus_one.findings.len() > MAX_FINDINGS {
                    return Err("too many entries");
                }
                for finding in &n_plus_one.findings {
                    labels.extend([
                        finding.method.as_str(),
                        finding.route.as_str(),
                        finding.fingerprint.as_str(),
                    ]);
                }
            }
            Database::Unavailable { reason } => labels.push(reason),
        }
        for queue in &self.queues {
            labels.push(&queue.name);
            if let Pending::Unavailable { reason } = &queue.pending {
                labels.push(reason);
            }
        }
        if labels
            .iter()
            .any(|label| label.chars().count() > MAX_LABEL_CHARS)
        {
            return Err("label too long");
        }
        Ok(())
    }
}
