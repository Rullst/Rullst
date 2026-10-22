//! The history page: charts of the values the poller kept for one project.

use super::{esc, heading, notice, number, query_value, table};
use crate::remote::history::Sample;

/// Most points drawn per chart; longer series are thinned evenly.
const MAX_POINTS: usize = 300;
const WIDTH: f64 = 600.0;
const HEIGHT: f64 = 100.0;

/// The period a history page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Range {
    Hour,
    Day,
    Week,
}

impl Range {
    /// The range for a `range` query value; anything else is a day.
    pub(crate) fn parse(value: Option<&str>) -> Self {
        match value {
            Some("1h") => Self::Hour,
            Some("7d") => Self::Week,
            _ => Self::Day,
        }
    }

    pub(crate) fn seconds(self) -> u64 {
        match self {
            Self::Hour => 3_600,
            Self::Day => 86_400,
            Self::Week => 7 * 86_400,
        }
    }

    fn value(self) -> &'static str {
        match self {
            Self::Hour => "1h",
            Self::Day => "24h",
            Self::Week => "7d",
        }
    }
}

/// An SVG line of `values` over the range, broken where a value is missing.
pub(crate) fn chart(
    samples: &[Sample],
    start: u64,
    span: u64,
    value: fn(&Sample) -> Option<f64>,
) -> String {
    let step = samples.len().div_ceil(MAX_POINTS).max(1);
    let points: Vec<(u64, Option<f64>)> = samples
        .iter()
        .step_by(step)
        .map(|sample| (sample.at, value(sample).filter(|value| value.is_finite())))
        .collect();
    let max = points
        .iter()
        .filter_map(|(_, value)| *value)
        .fold(0.0_f64, f64::max);
    let scale = if max > 0.0 { max } else { 1.0 };
    let span = span.max(1) as f64;
    let mut lines = String::new();
    let mut current = String::new();
    for (at, value) in points {
        match value {
            Some(value) => {
                let x = (at.saturating_sub(start) as f64 / span * WIDTH).clamp(0.0, WIDTH);
                let y = HEIGHT - (value.max(0.0) / scale * HEIGHT).clamp(0.0, HEIGHT);
                current.push_str(&format!("{x:.1},{y:.1} "));
            }
            None if !current.is_empty() => {
                lines.push_str(&polyline(&current));
                current.clear();
            }
            None => {}
        }
    }
    if !current.is_empty() {
        lines.push_str(&polyline(&current));
    }
    format!(
        r##"<svg viewBox="0 0 {WIDTH} {HEIGHT}" width="100%" height="100" preserveAspectRatio="none" role="img" aria-label="chart"><rect x="0" y="0" width="{WIDTH}" height="{HEIGHT}" fill="#020617" />{lines}</svg><div class="text-xs text-slate-500">max {}</div>"##,
        number(Some(max), 2)
    )
}

fn polyline(points: &str) -> String {
    format!(
        r##"<polyline points="{}" fill="none" stroke="#38bdf8" stroke-width="2" vector-effect="non-scaling-stroke" />"##,
        points.trim_end()
    )
}

/// Reads one charted value from a sample.
type Metric = fn(&Sample) -> Option<f64>;

fn integer(value: Option<u64>) -> Option<f64> {
    value.map(|value| value as f64)
}

/// The history page for `project`.
pub(crate) fn page(
    project: &str,
    samples: &[Sample],
    range: Range,
    now: u64,
    available: bool,
) -> String {
    let mut out = heading(
        &format!("History · {project}"),
        "Aggregates the background poller kept on this machine for watched projects.",
    );
    let links: String = [Range::Hour, Range::Day, Range::Week]
        .iter()
        .map(|option| {
            let active = if *option == range {
                "border-sky-500 text-white"
            } else {
                "border-slate-700 text-slate-300"
            };
            format!(
                r#"<a href="/studio/history?project={}&amp;range={}" class="px-3 py-1 rounded-lg border {active} text-xs font-bold">{}</a>"#,
                query_value(project),
                option.value(),
                option.value()
            )
        })
        .collect();
    out.push_str(&format!(
        r#"<div class="flex items-center gap-2">{links}</div>"#
    ));
    if !available {
        out.push_str(&notice(
            "warn",
            "History is unavailable",
            "The history database in your data directory could not be opened.",
        ));
        return out;
    }
    if samples.is_empty() {
        out.push_str(&notice(
            "ok",
            "No history yet",
            "Turn on Watch for this project on the Alerts page; the poller then keeps its values here.",
        ));
        return out;
    }
    let start = now.saturating_sub(range.seconds());
    let unreachable = samples.iter().filter(|sample| !sample.reachable).count();
    out.push_str(&format!(
        r#"<p class="text-sm text-slate-400">{} samples, {unreachable} unreachable.</p>"#,
        samples.len()
    ));
    out.push_str(r#"<div class="grid grid-cols-1 md:grid-cols-2 gap-4">"#);
    let charts: [(&str, Metric); 4] = [
        ("Requests per second", |sample| sample.requests_per_second),
        ("5xx rate (%)", |sample| sample.error_rate_percent),
        ("p95 latency (ms)", |sample| sample.p95_ms),
        ("Pending jobs", |sample| integer(sample.queue_pending)),
    ];
    for (title, value) in charts {
        out.push_str(&format!(
            r#"<div class="p-4 bg-slate-900/80 border border-slate-800 rounded-xl space-y-2"><div class="text-xs uppercase font-bold tracking-wider text-slate-400">{}</div>{}</div>"#,
            esc(title),
            chart(samples, start, range.seconds(), value)
        ));
    }
    out.push_str("</div>");
    let rows: Vec<Vec<String>> = samples
        .iter()
        .rev()
        .take(20)
        .map(|sample| {
            vec![
                sample.at.to_string(),
                if sample.reachable { "yes" } else { "no" }.to_string(),
                number(sample.requests_per_second, 3),
                number(sample.error_rate_percent, 2),
                number(sample.p95_ms, 1),
                sample
                    .queue_pending
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "—".to_string()),
                sample
                    .queue_failed_total
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ]
        })
        .collect();
    out.push_str(&table(
        &[
            "Unix time",
            "Reachable",
            "Req/s",
            "5xx %",
            "p95 ms",
            "Pending",
            "Failed total",
        ],
        &rows,
    ));
    out
}
