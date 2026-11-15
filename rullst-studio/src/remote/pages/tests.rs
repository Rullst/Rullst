//! Navigation, formatting and charts of the remote pages.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::history::{self, Range};
use super::{Chrome, Page, duration, layout, notice, page_link, query_value};
use crate::remote::document::MonitorDocument;
use crate::remote::history::Sample;
use crate::remote::server_test::{document, document_v2};

#[test]
fn pages_have_fixed_paths_and_project_links() {
    for (page, path) in [
        (Page::Projects, "/studio"),
        (Page::Overview, "/studio/radar"),
        (Page::Routes, "/studio/routes"),
        (Page::Queues, "/studio/queues"),
        (Page::Queries, "/studio/queries"),
        (Page::Traces, "/studio/traces"),
        (Page::Security, "/studio/security"),
        (Page::History, "/studio/history"),
        (Page::Alerts, "/studio/alerts"),
        (Page::Settings, "/studio/settings"),
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
    assert_eq!(page_link(Page::Settings, Some("shop")), "/studio/settings");
    assert_eq!(
        page_link(Page::Security, Some("shop")),
        "/studio/security?project=shop"
    );
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
        "🧵 Traces",
        "🛡️ Security",
        "📈 History",
        "🚨 Alerts",
        "⚙️ Settings",
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
        p99_ms: None,
        queue_pending: Some(4),
        queue_failed_total: Some(0),
        security_events_total: None,
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

#[test]
fn queue_failures_and_n_plus_one_findings_set_the_tone() {
    let html = super::monitor::overview("shop", &monitor_document());
    // The fixture's queue failed four jobs since start.
    assert!(html.contains(r#"text-amber-300"><div class="font-bold">Queues</div>"#));
    let mut value = document("/orders/{id}");
    value["queues"][0]["failed_total"] = serde_json::json!(0);
    value["database"]["n_plus_one"]["findings"] = serde_json::json!([]);
    let calm: MonitorDocument = serde_json::from_value(value).unwrap();
    let html = super::monitor::overview("shop", &calm);
    assert!(html.contains(r#"text-emerald-300"><div class="font-bold">Queues</div>"#));

    let queues = super::monitor::queues("shop", &monitor_document());
    assert!(
        queues.contains("<td class=\"px-3 py-2 break-all\">mail</td>"),
        "{queues}"
    );
    let queries = super::monitor::queries("shop", &calm);
    assert!(queries.contains("No repeated operations"), "{queries}");
    let queries = super::monitor::queries("shop", &monitor_document());
    assert!(!queries.contains("No repeated operations"));
    assert!(queries.contains("LineItem.find (line_items)"), "{queries}");
}

fn v2_document(edit: impl FnOnce(&mut serde_json::Value)) -> MonitorDocument {
    let mut value = document_v2("/orders/{id}");
    edit(&mut value);
    serde_json::from_value(value).unwrap()
}

fn card_value(html: &str, value: &str, hint: &str) -> bool {
    html.contains(&format!(
        r#">{value}</div><div class="text-xs text-slate-400 mt-1">{hint}</div>"#
    ))
}

#[test]
fn the_traces_page_lists_sampled_requests_and_span_totals() {
    let document = v2_document(|value| {
        let requests = &mut value["traces"]["requests"];
        let mut error = requests[0].clone();
        error["status"] = serde_json::json!(500);
        error["duration_ms"] = serde_json::json!(3.5);
        requests.as_array_mut().unwrap().push(error);
    });
    let html = super::traces::traces("shop", &document);
    assert!(html.contains("Traces · shop"));
    assert!(card_value(
        &html,
        "321",
        "at most one ordinary request per 100 ms"
    ));
    assert!(card_value(&html, "3", "the newest 50 kept"));
    assert!(
        card_value(&html, "2", "5xx among the shown requests"),
        "{html}"
    );
    assert!(card_value(
        &html,
        "1250.5 ms",
        "duration to the response headers"
    ));
    for cell in [
        "0m 10s ago",
        "/orders/{id}",
        "503",
        "1250.500 ms",
        "possible",
        "12.250 ms",
        "4.500 ms",
        "2.000 ms",
        "0m 20s ago",
    ] {
        assert!(
            html.contains(&format!(r#"<td class="px-3 py-2 break-all">{cell}</td>"#)),
            "{cell}: {html}"
        );
    }
    assert!(html.contains("7 spans in the application"));
    assert!(!html.contains("serves rullst.monitor.v1 only"));

    let quiet = v2_document(|value| {
        value["traces"]["requests"] = serde_json::json!([]);
        value["traces"]["spans"]["kinds"] = serde_json::json!([]);
    });
    let html = super::traces::traces("shop", &quiet);
    assert!(html.contains("No requests sampled yet"));
    assert!(card_value(&html, "—", "duration to the response headers"));
    assert!(!html.contains("<th class=\"px-3 py-2 text-left\">Kind</th>"));
}

#[test]
fn the_security_page_names_known_counters_and_lists_event_types() {
    let html = super::security::security("shop", &v2_document(|_| {}));
    assert!(html.contains("Security · shop"));
    assert!(card_value(
        &html,
        "17",
        "recorded since the process started"
    ));
    assert!(card_value(&html, "2", "peers banned by a honeypot"));
    assert!(card_value(&html, "40", "rate limits and lockouts"));
    assert!(card_value(
        &html,
        "3",
        "peers locked out after failed sign-ins"
    ));
    let refusals = html.find("Rate-limit and lockout refusals").unwrap();
    let jail = html.find("Login Jail lockouts").unwrap();
    let unknown = html.find(r#"break-all">/orders/{id}</td>"#).unwrap();
    assert!(refusals < jail && jail < unknown, "{html}");
    // Known counters appear by label only, absent ones not at all.
    assert!(!html.contains("rate_limit_blocks"));
    assert!(!html.contains("Deception trap hits"));
    assert!(html.contains("LOGIN_JAIL_TRIGGERED"));
    assert!(html.contains(r#"break-all">0m 5s ago</td>"#));

    let quiet = v2_document(|value| {
        value["security"]["events"] = serde_json::json!([]);
        value["security"]["counters"] = serde_json::json!({});
    });
    let html = super::security::security("shop", &quiet);
    assert!(html.contains("No security events kept"));
    assert!(card_value(&html, "0", "rate limits and lockouts"));
}

#[test]
fn version_1_documents_explain_why_traces_and_security_are_missing() {
    let document = monitor_document();
    for html in [
        super::traces::traces("shop", &document),
        super::security::security("shop", &document),
    ] {
        assert!(html.contains("It serves rullst.monitor.v1 only."), "{html}");
        assert!(!html.contains("<table"));
    }
}

#[test]
fn the_settings_page_marks_an_unusable_proxy() {
    use crate::remote::preferences::StudioSettings;
    use crate::remote::proxy::ProxyPlan;
    let settings = StudioSettings::default();
    let direct = super::preferences::page(&settings, &ProxyPlan::Direct, "t0k3n", None);
    assert!(direct.contains(r#"<p class="text-xs text-slate-300">Connections are direct.</p>"#));
    assert!(direct.contains(r#"name="form_token" value="t0k3n""#));
    let invalid = super::preferences::page(&settings, &ProxyPlan::Invalid, "t0k3n", None);
    assert!(invalid.contains(r#"<p class="text-xs text-red-300">The proxy cannot be used"#));
}
