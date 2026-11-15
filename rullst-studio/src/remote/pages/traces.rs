//! The traces page: sampled recent requests and span totals of a
//! `rullst.monitor.v2` document.

use super::monitor::project_heading;
use super::{card, duration, notice, number, table};
use crate::remote::document::MonitorDocument;

/// The notice for a project that serves version 1 only.
pub(super) fn version_1_only() -> String {
    notice(
        "warn",
        "Not available from this application",
        "It serves rullst.monitor.v1 only. Request traces and the security radar need Rullst 13.0.0-alpha.6 or later on the server (GET /_rullst/monitor/v2).",
    )
}

/// `seconds` before the document was generated, as an age.
pub(super) fn ago(document: &MonitorDocument, at: u64) -> String {
    format!("{} ago", duration(document.generated_at.saturating_sub(at)))
}

/// Traces: the newest sampled requests, then the span totals.
pub(crate) fn traces(project: &str, document: &MonitorDocument) -> String {
    let mut out = project_heading(project, document, "Traces");
    let Some(traces) = &document.traces else {
        out.push_str(&version_1_only());
        return out;
    };
    let errors = traces
        .requests
        .iter()
        .filter(|request| request.status >= 500)
        .count();
    let slowest = traces
        .requests
        .iter()
        .map(|request| request.duration_ms)
        .fold(None, |slowest: Option<f64>, value| {
            Some(slowest.map_or(value, |slowest| slowest.max(value)))
        });
    out.push_str(r#"<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">"#);
    for (label, value, hint) in [
        (
            "Sampled since start",
            traces.sampled_total.to_string(),
            "at most one ordinary request per 100 ms".to_string(),
        ),
        (
            "Shown",
            traces.requests.len().to_string(),
            format!("the newest {} kept", traces.sample_limit),
        ),
        (
            "Server errors",
            errors.to_string(),
            "5xx among the shown requests".to_string(),
        ),
        (
            "Slowest shown",
            slowest.map_or_else(
                || "—".to_string(),
                |ms| format!("{} ms", number(Some(ms), 1)),
            ),
            "duration to the response headers".to_string(),
        ),
    ] {
        out.push_str(&card(label, &value, &hint));
    }
    out.push_str("</div>");
    if traces.requests.is_empty() {
        out.push_str(&notice(
            "ok",
            "No requests sampled yet",
            "The server has not kept a request summary since it started.",
        ));
    } else {
        let rows: Vec<Vec<String>> = traces
            .requests
            .iter()
            .map(|request| {
                vec![
                    ago(document, request.at),
                    request.method.clone(),
                    request.route.clone(),
                    request.status.to_string(),
                    format!("{} ms", number(Some(request.duration_ms), 3)),
                    request.db_operations.to_string(),
                    if request.n_plus_one {
                        "possible"
                    } else {
                        "—"
                    }
                    .to_string(),
                ]
            })
            .collect();
        out.push_str(&table(
            &[
                "When",
                "Method",
                "Route",
                "Status",
                "Duration",
                "ORM operations",
                "N+1",
            ],
            &rows,
        ));
    }
    let spans = &traces.spans;
    out.push_str(&format!(
        r#"<p class="text-sm text-slate-400">{} spans in the application's SpanCollector. Only totals per kind leave the server; span names never do.</p>"#,
        spans.kept
    ));
    if !spans.kinds.is_empty() {
        let rows: Vec<Vec<String>> = spans
            .kinds
            .iter()
            .map(|kind| {
                vec![
                    kind.kind.clone(),
                    kind.count.to_string(),
                    format!("{} ms", number(Some(kind.total_us as f64 / 1_000.0), 3)),
                    format!("{} ms", number(Some(kind.max_us as f64 / 1_000.0), 3)),
                    ago(document, kind.last_at),
                ]
            })
            .collect();
        out.push_str(&table(
            &["Kind", "Spans", "Total", "Longest", "Latest"],
            &rows,
        ));
    }
    out
}
