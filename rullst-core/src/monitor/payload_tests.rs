//! The `rullst.monitor.v1` schema: its exact field set and values.
#![allow(clippy::unwrap_used)]

use super::collector::Collector;
use super::payload::{Application, Document, Pending, QueueReport, database, http};
use crate::queue::QueueActivitySnapshot;
use crate::radar::RadarSnapshot;
use crate::server::dev_telemetry::request_scope::RequestQueries;
use axum::http::Method;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

/// Every field path of `value`; array elements appear as `[]`.
fn paths(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                out.insert(path.clone());
                paths(child, &path, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                let path = format!("{prefix}[]");
                out.insert(path.clone());
                paths(item, &path, out);
            }
        }
        _ => {}
    }
}

fn populated_collector() -> Collector {
    let collector = Collector::new();
    let queries = RequestQueries {
        operations: 4,
        fingerprints: vec!["Post.find (posts)".to_string(); 3],
    };
    collector.record_at(
        Duration::ZERO,
        &Method::GET,
        Some("/posts/{id}"),
        200,
        Duration::from_millis(3),
        &queries,
    );
    collector
}

fn radar() -> RadarSnapshot {
    RadarSnapshot {
        uptime_seconds: 42,
        memory_rss_mb: Some(12.5),
        cpu_usage_percent: Some(3.25),
        active_tokio_tasks: Some(7),
        tokio_latency_micros: Some(15),
        timestamp: 1_800_000_000,
    }
}

fn queues() -> Vec<QueueReport> {
    let activity = QueueActivitySnapshot {
        in_flight: 1,
        completed: 2,
        failed: 3,
        retried: 4,
        stalled_recovered: 5,
    };
    vec![
        QueueReport::new("mail", Pending::Observed { count: 9 }, activity),
        QueueReport::new(
            "reports",
            Pending::Unavailable { reason: "timeout" },
            QueueActivitySnapshot::default(),
        ),
    ]
}

fn document(orm_unavailable: Option<&'static str>) -> Value {
    let application = Application {
        name: "shop".to_string(),
        version: "1.4.0".to_string(),
    };
    let document = Document::new(
        Some(application),
        radar(),
        &populated_collector(),
        orm_unavailable,
        queues(),
    );
    serde_json::to_value(&document).unwrap()
}

// TM-MONITOR-04: adding a field is a reviewed schema change.
#[test]
fn the_document_has_exactly_the_v1_fields() {
    let mut found = BTreeSet::new();
    paths(&document(None), "", &mut found);
    let window = [
        "covered_seconds",
        "latency_ms",
        "latency_ms.max",
        "latency_ms.p50",
        "latency_ms.p90",
        "latency_ms.p95",
        "latency_ms.p99",
        "requests",
        "requests_per_second",
        "status",
        "status.1xx",
        "status.2xx",
        "status.3xx",
        "status.4xx",
        "status.5xx",
    ];
    let classes = ["1xx", "2xx", "3xx", "4xx", "5xx"];
    let mut expected: BTreeSet<String> = [
        "application",
        "application.name",
        "application.version",
        "database",
        "database.n_plus_one",
        "database.n_plus_one.findings",
        "database.n_plus_one.findings[]",
        "database.n_plus_one.findings[].fingerprint",
        "database.n_plus_one.findings[].last_seen",
        "database.n_plus_one.findings[].max_occurrences",
        "database.n_plus_one.findings[].method",
        "database.n_plus_one.findings[].requests",
        "database.n_plus_one.findings[].route",
        "database.n_plus_one.findings_total",
        "database.n_plus_one.threshold",
        "database.queries_total",
        "database.state",
        "framework",
        "framework.name",
        "framework.version",
        "generated_at",
        "http",
        "http.requests_total",
        "http.route_limit",
        "http.routes",
        "http.routes[]",
        "http.routes[].method",
        "http.routes[].requests_total",
        "http.routes[].route",
        "http.routes[].status_total",
        "http.routes[].window",
        "http.routes_overflowed",
        "http.status_total",
        "http.window",
        "http.window_seconds",
        "process",
        "process.active_tokio_tasks",
        "process.cpu_usage_percent",
        "process.memory_rss_mb",
        "process.tokio_latency_micros",
        "queues",
        "queues[]",
        "queues[].completed_total",
        "queues[].failed_total",
        "queues[].in_flight",
        "queues[].name",
        "queues[].pending",
        "queues[].pending.count",
        "queues[].pending.reason",
        "queues[].pending.state",
        "queues[].retried_total",
        "queues[].stalled_recovered_total",
        "schema",
        "uptime_seconds",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    for prefix in ["http.window", "http.routes[].window"] {
        expected.extend(window.iter().map(|field| format!("{prefix}.{field}")));
    }
    for prefix in ["http.status_total", "http.routes[].status_total"] {
        expected.extend(classes.iter().map(|class| format!("{prefix}.{class}")));
    }
    assert_eq!(found, expected);
}

#[test]
fn the_document_reports_the_collected_values() {
    let value = document(None);
    assert_eq!(value["schema"], "rullst.monitor.v1");
    assert_eq!(
        value["application"],
        json!({"name": "shop", "version": "1.4.0"})
    );
    assert_eq!(value["framework"]["name"], "rullst");
    assert_eq!(value["framework"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["uptime_seconds"], 42);
    assert_eq!(
        value["process"],
        json!({
            "memory_rss_mb": 12.5,
            "cpu_usage_percent": 3.25,
            "active_tokio_tasks": 7,
            "tokio_latency_micros": 15,
        })
    );
    assert!(value["generated_at"].as_u64().unwrap() > 1_700_000_000);
    assert_eq!(value["http"]["requests_total"], 1);
    assert_eq!(value["http"]["route_limit"], 256);
    assert_eq!(value["http"]["window_seconds"], 300);
    assert_eq!(value["http"]["routes_overflowed"], false);
    assert_eq!(value["http"]["routes"][0]["route"], "/posts/{id}");
    assert_eq!(
        value["database"]["n_plus_one"],
        json!({
            "threshold": 3,
            "findings_total": 1,
            "findings": [{
                "method": "GET",
                "route": "/posts/{id}",
                "fingerprint": "Post.find (posts)",
                "requests": 1,
                "max_occurrences": 3,
                "last_seen": value["database"]["n_plus_one"]["findings"][0]["last_seen"],
            }],
        })
    );
    assert_eq!(value["database"]["queries_total"], 4);
    assert_eq!(value["database"]["state"], "observed");
    assert_eq!(
        value["queues"],
        json!([
            {
                "name": "mail",
                "pending": {"state": "observed", "count": 9},
                "in_flight": 1,
                "completed_total": 2,
                "failed_total": 3,
                "retried_total": 4,
                "stalled_recovered_total": 5,
            },
            {
                "name": "reports",
                "pending": {"state": "unavailable", "reason": "timeout"},
                "in_flight": 0,
                "completed_total": 0,
                "failed_total": 0,
                "retried_total": 0,
                "stalled_recovered_total": 0,
            },
        ])
    );
    assert_eq!(value["application"]["name"], "shop");
}

#[test]
fn an_unobservable_database_reports_only_the_reason() {
    assert_eq!(
        document(Some("orm_spans_filtered"))["database"],
        json!({"state": "unavailable", "reason": "orm_spans_filtered"})
    );
    let collector = Collector::new();
    assert_eq!(
        serde_json::to_value(database(&collector, Some("subscriber_not_installed"))).unwrap(),
        json!({"state": "unavailable", "reason": "subscriber_not_installed"})
    );
}

#[test]
fn http_reports_totals_window_rate_and_percentiles() {
    let collector = Collector::new();
    let none = RequestQueries::default();
    for (status, millis) in [(200, 2), (200, 4), (503, 40)] {
        collector.record_at(
            Duration::from_secs(10),
            &Method::GET,
            Some("/orders"),
            status,
            Duration::from_millis(millis),
            &none,
        );
    }
    let report = serde_json::to_value(http(&collector, Duration::from_secs(30))).unwrap();
    let window = json!({
        "covered_seconds": 30,
        "requests": 3,
        "requests_per_second": 0.1,
        "status": {"1xx": 0, "2xx": 2, "3xx": 0, "4xx": 0, "5xx": 1},
        "latency_ms": {"p50": 5.0, "p90": 40.0, "p95": 40.0, "p99": 40.0, "max": 40.0},
    });
    let status = json!({"1xx": 0, "2xx": 2, "3xx": 0, "4xx": 0, "5xx": 1});
    assert_eq!(
        report,
        json!({
            "window_seconds": 300,
            "route_limit": 256,
            "routes_overflowed": false,
            "requests_total": 3,
            "status_total": status,
            "window": window,
            "routes": [{
                "method": "GET",
                "route": "/orders",
                "requests_total": 3,
                "status_total": status,
                "window": window,
            }],
        })
    );

    // Ten minutes later the window is empty; the totals remain.
    let later = serde_json::to_value(http(&collector, Duration::from_secs(610))).unwrap();
    assert_eq!(later["requests_total"], 3);
    assert_eq!(later["window"]["requests"], 0);
    assert_eq!(later["window"]["requests_per_second"], 0.0);
    assert_eq!(later["window"]["latency_ms"], Value::Null);
}

#[test]
fn the_rate_is_rounded_to_thousandths() {
    let collector = Collector::new();
    let none = RequestQueries::default();
    for _ in 0..2 {
        collector.record_at(
            Duration::from_secs(1),
            &Method::GET,
            Some("/"),
            200,
            Duration::from_millis(1),
            &none,
        );
    }
    // 2 requests over 3 seconds.
    let report = serde_json::to_value(http(&collector, Duration::from_secs(3))).unwrap();
    assert_eq!(report["window"]["requests_per_second"], 0.667);
}
