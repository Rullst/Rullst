//! Pages rendering one project's `rullst.monitor.v1` document.

use super::{card, duration, heading, notice, number, table};
use crate::remote::document::{Database, Latency, MonitorDocument, Pending, StatusClasses};

fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        "—".to_string()
    } else {
        format!("{:.2}%", part as f64 * 100.0 / whole as f64)
    }
}

fn latency(latency: Option<&Latency>, pick: fn(&Latency) -> f64) -> String {
    latency
        .map(|latency| format!("{} ms", number(Some(pick(latency)), 1)))
        .unwrap_or_else(|| "—".to_string())
}

fn classes(status: &StatusClasses) -> String {
    let informational = if status.informational > 0 {
        format!("1xx {} · ", status.informational)
    } else {
        String::new()
    };
    format!(
        "{informational}2xx {} · 3xx {} · 4xx {} · 5xx {}",
        status.success, status.redirection, status.client_error, status.server_error
    )
}

/// The header shared by the monitoring pages.
fn project_heading(project: &str, document: &MonitorDocument, title: &str) -> String {
    let application = document
        .application
        .as_ref()
        .map(|application| format!("{} {}", application.name, application.version))
        .unwrap_or_else(|| "application not reported".to_string());
    heading(
        &format!("{title} · {project}"),
        &format!(
            "{application} on {} {} · document generated at Unix time {}",
            document.framework.name, document.framework.version, document.generated_at
        ),
    )
}

/// Overview: process radar, request totals and window, database and queues.
pub(crate) fn overview(project: &str, document: &MonitorDocument) -> String {
    let http = &document.http;
    let window = &http.window;
    let mut out = project_heading(project, document, "Overview");
    out.push_str(r#"<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">"#);
    for (label, value, hint) in [
        (
            "Uptime",
            duration(document.uptime_seconds),
            "process uptime".to_string(),
        ),
        (
            "Requests / s",
            number(Some(window.requests_per_second), 3),
            format!(
                "over the last {} s (window {} s)",
                window.covered_seconds, http.window_seconds
            ),
        ),
        (
            "5xx rate",
            percent(window.status.server_error, window.requests),
            format!(
                "{} of {} in the window",
                window.status.server_error, window.requests
            ),
        ),
        (
            "p95 latency",
            latency(window.latency_ms.as_ref(), |latency| latency.p95),
            format!(
                "p99 {}",
                latency(window.latency_ms.as_ref(), |latency| latency.p99)
            ),
        ),
        (
            "Memory",
            number(document.process.memory_rss_mb, 1),
            "resident MiB".to_string(),
        ),
        (
            "CPU",
            number(document.process.cpu_usage_percent, 1),
            "percent of one CPU".to_string(),
        ),
        (
            "Tokio tasks",
            document
                .process
                .active_tokio_tasks
                .map(|tasks| tasks.to_string())
                .unwrap_or_else(|| "—".to_string()),
            document
                .process
                .tokio_latency_micros
                .map(|micros| format!("scheduler latency {micros} µs"))
                .unwrap_or_else(|| "scheduler latency —".to_string()),
        ),
        (
            "Requests since start",
            http.requests_total.to_string(),
            classes(&http.status_total),
        ),
    ] {
        out.push_str(&card(label, &value, &hint));
    }
    out.push_str("</div>");
    if http.routes_overflowed {
        out.push_str(&notice(
            "warn",
            "Route limit reached",
            &format!(
                "The server keeps {} route patterns; later ones share the (other) bucket.",
                http.route_limit
            ),
        ));
    }
    out.push_str(&match &document.database {
        Database::Observed {
            queries_total,
            n_plus_one,
        } => notice(
            if n_plus_one.findings_total > 0 { "warn" } else { "ok" },
            "Database",
            &format!(
                "{queries_total} ORM operations started by requests; {} possible N+1 findings (threshold {}).",
                n_plus_one.findings_total, n_plus_one.threshold
            ),
        ),
        Database::Unavailable { reason } => notice(
            "warn",
            "Database observations unavailable",
            &format!("The server reports: {reason}."),
        ),
    });
    let pending: u64 = document
        .queues
        .iter()
        .filter_map(|queue| match queue.pending {
            Pending::Observed { count } => Some(count),
            Pending::Unavailable { .. } => None,
        })
        .sum();
    let failed: u64 = document.queues.iter().map(|queue| queue.failed_total).sum();
    out.push_str(&notice(
        if failed > 0 { "warn" } else { "ok" },
        "Queues",
        &format!(
            "{} registered queues, {pending} pending jobs, {failed} failed since start.",
            document.queues.len()
        ),
    ));
    out
}

/// Routes: per pattern and method, the window and the totals.
pub(crate) fn routes(project: &str, document: &MonitorDocument) -> String {
    let mut out = project_heading(project, document, "Routes");
    let mut routes: Vec<_> = document.http.routes.iter().collect();
    routes.sort_by(|left, right| {
        right
            .window
            .requests
            .cmp(&left.window.requests)
            .then_with(|| right.requests_total.cmp(&left.requests_total))
    });
    if routes.is_empty() {
        out.push_str(&notice(
            "ok",
            "No requests yet",
            "The server has not recorded a request since it started.",
        ));
        return out;
    }
    let rows: Vec<Vec<String>> = routes
        .iter()
        .map(|route| {
            let window = &route.window;
            let latency_ms = window.latency_ms.as_ref();
            vec![
                route.method.clone(),
                route.route.clone(),
                window.requests.to_string(),
                number(Some(window.requests_per_second), 3),
                window.status.client_error.to_string(),
                window.status.server_error.to_string(),
                percent(window.status.server_error, window.requests),
                latency(latency_ms, |latency| latency.p50),
                latency(latency_ms, |latency| latency.p90),
                latency(latency_ms, |latency| latency.p95),
                latency(latency_ms, |latency| latency.p99),
                latency(latency_ms, |latency| latency.max),
                route.requests_total.to_string(),
                route.status_total.server_error.to_string(),
            ]
        })
        .collect();
    out.push_str(&table(
        &[
            "Method",
            "Route",
            "Window",
            "Req/s",
            "4xx",
            "5xx",
            "5xx rate",
            "p50",
            "p90",
            "p95",
            "p99",
            "Max",
            "Total",
            "Total 5xx",
        ],
        &rows,
    ));
    out
}

/// Queues: pending count and this process's worker activity.
pub(crate) fn queues(project: &str, document: &MonitorDocument) -> String {
    let mut out = project_heading(project, document, "Queues");
    if document.queues.is_empty() {
        out.push_str(&notice(
            "ok",
            "No queues registered",
            "Register queues with Monitor::with_queue on the server to see them here.",
        ));
        return out;
    }
    let rows: Vec<Vec<String>> = document
        .queues
        .iter()
        .map(|queue| {
            vec![
                queue.name.clone(),
                match &queue.pending {
                    Pending::Observed { count } => count.to_string(),
                    Pending::Unavailable { reason } => format!("unavailable ({reason})"),
                },
                queue.in_flight.to_string(),
                queue.completed_total.to_string(),
                queue.failed_total.to_string(),
                queue.retried_total.to_string(),
                queue.stalled_recovered_total.to_string(),
            ]
        })
        .collect();
    out.push_str(&table(
        &[
            "Queue",
            "Pending",
            "In flight",
            "Completed",
            "Failed",
            "Retried",
            "Stalled recovered",
        ],
        &rows,
    ));
    out
}

/// N+1 findings: repeated ORM operations per route.
pub(crate) fn queries(project: &str, document: &MonitorDocument) -> String {
    let mut out = project_heading(project, document, "N+1 queries");
    match &document.database {
        Database::Unavailable { reason } => out.push_str(&notice(
            "warn",
            "Database observations unavailable",
            &format!("The server reports: {reason}."),
        )),
        Database::Observed { n_plus_one, .. } if n_plus_one.findings.is_empty() => {
            out.push_str(&notice(
                "ok",
                "No repeated operations",
                &format!(
                    "No request repeated one ORM operation {} times or more.",
                    n_plus_one.threshold
                ),
            ));
        }
        Database::Observed { n_plus_one, .. } => {
            let rows: Vec<Vec<String>> = n_plus_one
                .findings
                .iter()
                .map(|finding| {
                    vec![
                        finding.method.clone(),
                        finding.route.clone(),
                        finding.fingerprint.clone(),
                        finding.requests.to_string(),
                        finding.max_occurrences.to_string(),
                        duration(document.generated_at.saturating_sub(finding.last_seen)) + " ago",
                    ]
                })
                .collect();
            out.push_str(&format!(
                r#"<p class="text-sm text-slate-400">{} findings since start; an operation repeated at least {} times within one request. Repetition can be intentional: inspect the route.</p>"#,
                n_plus_one.findings_total, n_plus_one.threshold
            ));
            out.push_str(&table(
                &[
                    "Method",
                    "Route",
                    "Operation",
                    "Requests",
                    "Most in one request",
                    "Last seen",
                ],
                &rows,
            ));
        }
    }
    out
}
