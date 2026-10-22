//! The `rullst.monitor.v1` document.
//!
//! Every field is an aggregate or a label the application chose (route
//! patterns, queue names, its name and version) or the ORM's static
//! `Model.operation (table)` fingerprint. `payload_tests.rs` pins the exact
//! field set; adding a field is a schema change.

use super::MONITOR_SCHEMA;
use super::collector::{Collector, Finding, MAX_ROUTES, RouteSnapshot, total_window, unix_seconds};
use super::window::{Latency, StatusClasses, WINDOW_SECONDS, WindowCounts, covered_seconds};
use crate::query_patterns::N_PLUS_ONE_THRESHOLD;
use crate::queue::QueueActivitySnapshot;
use crate::radar::RadarSnapshot;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub(super) struct Document {
    pub(super) schema: &'static str,
    pub(super) generated_at: u64,
    pub(super) application: Option<Application>,
    pub(super) framework: Framework,
    pub(super) uptime_seconds: u64,
    pub(super) process: Process,
    pub(super) http: Http,
    pub(super) database: Database,
    pub(super) queues: Vec<QueueReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct Application {
    pub(super) name: String,
    pub(super) version: String,
}

#[derive(Debug, Serialize)]
pub(super) struct Framework {
    pub(super) name: &'static str,
    pub(super) version: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct Process {
    pub(super) memory_rss_mb: Option<f64>,
    pub(super) cpu_usage_percent: Option<f64>,
    pub(super) active_tokio_tasks: Option<usize>,
    pub(super) tokio_latency_micros: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(super) struct Http {
    pub(super) window_seconds: u64,
    pub(super) route_limit: usize,
    pub(super) routes_overflowed: bool,
    pub(super) requests_total: u64,
    pub(super) status_total: StatusClasses,
    pub(super) window: WindowReport,
    pub(super) routes: Vec<RouteReport>,
}

#[derive(Debug, Serialize)]
pub(super) struct WindowReport {
    pub(super) covered_seconds: u64,
    pub(super) requests: u64,
    pub(super) requests_per_second: f64,
    pub(super) status: StatusClasses,
    pub(super) latency_ms: Option<Latency>,
}

#[derive(Debug, Serialize)]
pub(super) struct RouteReport {
    pub(super) method: &'static str,
    pub(super) route: Arc<str>,
    pub(super) requests_total: u64,
    pub(super) status_total: StatusClasses,
    pub(super) window: WindowReport,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Database {
    Observed {
        queries_total: u64,
        n_plus_one: NPlusOne,
    },
    Unavailable {
        reason: &'static str,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct NPlusOne {
    pub(super) threshold: u64,
    pub(super) findings_total: u64,
    pub(super) findings: Vec<FindingReport>,
}

#[derive(Debug, Serialize)]
pub(super) struct FindingReport {
    pub(super) method: &'static str,
    pub(super) route: Arc<str>,
    pub(super) fingerprint: String,
    pub(super) requests: u64,
    pub(super) max_occurrences: u64,
    pub(super) last_seen: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct QueueReport {
    pub(super) name: String,
    pub(super) pending: Pending,
    pub(super) in_flight: u64,
    pub(super) completed_total: u64,
    pub(super) failed_total: u64,
    pub(super) retried_total: u64,
    pub(super) stalled_recovered_total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Pending {
    Observed { count: u64 },
    Unavailable { reason: &'static str },
}

impl QueueReport {
    pub(super) fn new(name: &str, pending: Pending, activity: QueueActivitySnapshot) -> Self {
        Self {
            name: name.to_string(),
            pending,
            in_flight: activity.in_flight,
            completed_total: activity.completed,
            failed_total: activity.failed,
            retried_total: activity.retried,
            stalled_recovered_total: activity.stalled_recovered,
        }
    }
}

impl Document {
    /// Assembles the document. `orm_unavailable` is why ORM operations cannot
    /// be observed, if they cannot.
    pub(super) fn new(
        application: Option<Application>,
        radar: RadarSnapshot,
        collector: &Collector,
        orm_unavailable: Option<&'static str>,
        queues: Vec<QueueReport>,
    ) -> Self {
        Self {
            schema: MONITOR_SCHEMA,
            generated_at: unix_seconds(),
            application,
            framework: Framework {
                name: "rullst",
                version: env!("CARGO_PKG_VERSION"),
            },
            uptime_seconds: radar.uptime_seconds,
            process: Process {
                memory_rss_mb: radar.memory_rss_mb,
                cpu_usage_percent: radar.cpu_usage_percent,
                active_tokio_tasks: radar.active_tokio_tasks,
                tokio_latency_micros: radar.tokio_latency_micros,
            },
            http: http(collector, collector.elapsed()),
            database: database(collector, orm_unavailable),
            queues,
        }
    }
}

pub(super) fn http(collector: &Collector, now: Duration) -> Http {
    let routes = collector.routes();
    let mut requests_total = 0_u64;
    let mut status_total = StatusClasses::default();
    for route in &routes {
        requests_total = requests_total.saturating_add(route.stats.requests_total);
        status_total.add(&route.stats.status_total);
    }
    Http {
        window_seconds: WINDOW_SECONDS,
        route_limit: MAX_ROUTES,
        routes_overflowed: collector.overflowed(),
        requests_total,
        status_total,
        window: window_report(&total_window(&routes, now), now),
        routes: routes
            .iter()
            .map(|route| route_report(route, now))
            .collect(),
    }
}

fn route_report(route: &RouteSnapshot, now: Duration) -> RouteReport {
    RouteReport {
        method: route.method,
        route: Arc::clone(&route.route),
        requests_total: route.stats.requests_total,
        status_total: route.stats.status_total,
        window: window_report(&route.stats.window(now), now),
    }
}

fn window_report(counts: &WindowCounts, now: Duration) -> WindowReport {
    let covered = covered_seconds(now);
    let rate = counts.requests as f64 / covered as f64;
    WindowReport {
        covered_seconds: covered,
        requests: counts.requests,
        requests_per_second: (rate * 1_000.0).round() / 1_000.0,
        status: counts.status,
        latency_ms: counts.histogram.latency(),
    }
}

pub(super) fn database(collector: &Collector, unavailable: Option<&'static str>) -> Database {
    if let Some(reason) = unavailable {
        return Database::Unavailable { reason };
    }
    let (findings_total, findings) = collector.findings();
    Database::Observed {
        queries_total: collector.queries(),
        n_plus_one: NPlusOne {
            threshold: u64::try_from(N_PLUS_ONE_THRESHOLD).unwrap_or(u64::MAX),
            findings_total,
            findings: findings.into_iter().map(finding_report).collect(),
        },
    }
}

fn finding_report(finding: Finding) -> FindingReport {
    FindingReport {
        method: finding.method,
        route: finding.route,
        fingerprint: finding.fingerprint,
        requests: finding.requests,
        max_occurrences: finding.max_occurrences,
        last_seen: finding.last_seen,
    }
}
