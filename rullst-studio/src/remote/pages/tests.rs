//! Navigation, formatting and charts of the remote pages.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::history::{self, Range};
use super::{Chrome, Page, duration, layout, notice, page_link, query_value};
use crate::remote::document::MonitorDocument;
use crate::remote::history::Sample;
use crate::remote::server_test::document;

#[test]
fn pages_have_fixed_paths_and_project_links() {
    for (page, path) in [
        (Page::Projects, "/studio"),
        (Page::Overview, "/studio/radar"),
        (Page::Routes, "/studio/routes"),
        (Page::Queues, "/studio/queues"),
        (Page::Queries, "/studio/queries"),
        (Page::History, "/studio/history"),
        (Page::Alerts, "/studio/alerts"),
    ] {
        assert_eq!(page.path(), path);
        assert_eq!(page_link(page, None), path);
    }
    assert_eq!(query_value("my shop&x"), "my%20shop%26x");
    assert_eq!(
        page_link(Page::Routes, Some("my shop")),
        "/studio/routes?project=my%20shop"
    );
    assert_eq!(page_link(Page::Projects, Some("shop")), "/studio");
    assert_eq!(page_link(Page::Alerts, Some("shop")), "/studio/alerts");
}

fn chrome(projects: &[String], active: Page, open_alerts: u64) -> String {
    let chrome = Chrome {
        projects,
        selected: Some("shop"),
        active,
        open_alerts,
    };
    layout(&chrome, "Title", "<p>content</p>")
}

const ACTIVE: &str = "px-3 py-1 rounded-lg bg-slate-800 text-white";
const BADGE: &str = "px-2 py-0.5 rounded-full bg-rose-500/10";

#[test]
fn the_navigation_marks_the_active_page_and_counts_open_alerts() {
    let projects = ["shop".to_string()];
    let page = chrome(&projects, Page::Routes, 0);
    assert_eq!(page.matches(ACTIVE).count(), 1);
    assert!(page.contains(&format!(
        r#"<a href="/studio/routes?project=shop" class="{ACTIVE}"#
    )));
    for label in [
        "🗂️ Projects",
        "📡 Overview",
        "🛣️ Routes",
        "📬 Queues",
        "🔁 N+1 Queries",
        "📈 History",
        "🚨 Alerts",
    ] {
        assert!(page.contains(&format!("<span>{label}</span>")), "{label}");
    }
    assert!(!page.contains(BADGE));

    let page = chrome(&projects, Page::Alerts, 3);
    assert_eq!(page.matches(BADGE).count(), 1);
    assert!(page.contains(&format!(
        r#"<span>🚨 Alerts</span> <span class="{BADGE} text-rose-400 border border-rose-500/30">3</span>"#
    )));
}

#[test]
fn notices_take_the_colour_of_their_tone() {
    for (tone, colour) in [
        ("ok", "text-emerald-300"),
        ("warn", "text-amber-300"),
        ("error", "text-red-300"),
    ] {
        let html = notice(tone, "Title", "Detail");
        assert!(html.contains(colour), "{tone}");
    }
}

#[test]
fn durations_read_as_minutes_hours_or_days() {
    for (seconds, text) in [
        (0, "0m 0s"),
        (184, "3m 4s"),
        (3_599, "59m 59s"),
        (7_384, "2h 3m"),
        (86_399, "23h 59m"),
        (93_784, "1d 2h"),
        (1_036_800, "12d 0h"),
    ] {
        assert_eq!(duration(seconds), text, "{seconds}");
    }
}

#[test]
fn history_ranges_parse_and_span_their_period() {
    assert_eq!(Range::parse(Some("1h")), Range::Hour);
    assert_eq!(Range::parse(Some("7d")), Range::Week);
    assert_eq!(Range::parse(Some("24h")), Range::Day);
    assert_eq!(Range::parse(None), Range::Day);
    assert_eq!(Range::Hour.seconds(), 3_600);
    assert_eq!(Range::Day.seconds(), 86_400);
    assert_eq!(Range::Week.seconds(), 604_800);
}

fn sample(at: u64, p95_ms: Option<f64>) -> Sample {
    Sample {
        at,
        reachable: true,
        requests_per_second: Some(0.5),
        window_requests: Some(10),
        error_rate_percent: Some(1.0),
        p95_ms,
        queue_pending: Some(4),
        queue_failed_total: Some(0),
    }
}

fn polylines(svg: &str) -> Vec<&str> {
    svg.split(r#"<polyline points=""#)
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn a_chart_scales_to_its_range_and_breaks_at_missing_values() {
    let samples = [
        sample(1_000, None),
        sample(1_000, Some(10.0)),
        sample(1_050, Some(5.0)),
        sample(1_075, None),
        sample(1_100, Some(2.5)),
    ];
    let svg = history::chart(&samples, 1_000, 100, |sample| sample.p95_ms);
    assert_eq!(
        polylines(&svg),
        ["0.0,0.0 300.0,50.0", "600.0,75.0"],
        "{svg}"
    );
    assert!(svg.ends_with(r#"<div class="text-xs text-slate-500">max 10.00</div>"#));

    let flat = [sample(1_000, Some(0.0)), sample(1_100, Some(0.0))];
    let svg = history::chart(&flat, 1_000, 100, |sample| sample.p95_ms);
    assert_eq!(polylines(&svg), ["0.0,100.0 600.0,100.0"], "{svg}");
}

#[test]
fn the_history_page_links_its_ranges_and_charts_pending_jobs() {
    let samples = [sample(1_000, Some(10.0))];
    let html = history::page("shop", &samples, Range::Week, 1_100, true);
    for range in ["1h", "24h", "7d"] {
        assert!(
            html.contains(&format!(
                r#"<a href="/studio/history?project=shop&amp;range={range}" class="px-3 py-1 rounded-lg border"#
            )),
            "{range}"
        );
    }
    assert!(
        html.contains(r#"range=7d" class="px-3 py-1 rounded-lg border border-sky-500 text-white"#)
    );
    assert_eq!(html.matches("border-sky-500 text-white").count(), 1);
    // Pending jobs is the only series whose largest value is 4.
    assert!(html.contains(">max 4.00<"), "{html}");
}

fn monitor_document() -> MonitorDocument {
    serde_json::from_value(document("/orders/{id}")).unwrap()
}

#[test]
fn the_overview_shows_rates_latency_and_status_classes() {
    let html = super::monitor::overview("shop", &monitor_document());
    for expected in [
        "5.00%",
        "25.0 ms",
        "p99 50.0 ms",
        "2xx 1150 · 3xx 10 · 4xx 30 · 5xx 10",
    ] {
        assert!(html.contains(expected), "{expected}");
    }
    assert!(!html.contains("1xx"));
    // One possible N+1 finding is a warning; none is fine.
    assert!(html.contains(r#"text-amber-300"><div class="font-bold">Database</div>"#));

    let mut value = document("/orders/{id}");
    value["http"]["window"]["requests"] = serde_json::json!(0);
    value["http"]["status_total"]["1xx"] = serde_json::json!(3);
    value["http"]["window"]["latency_ms"] = serde_json::Value::Null;
    value["database"]["n_plus_one"]["findings_total"] = serde_json::json!(0);
    let quiet: MonitorDocument = serde_json::from_value(value).unwrap();
    let html = super::monitor::overview("shop", &quiet);
    assert!(html.contains("1xx 3 · 2xx 1150"), "{html}");
    assert!(html.contains("p99 —"), "{html}");
    assert!(html.contains(r#"text-emerald-300"><div class="font-bold">Database</div>"#));
}
