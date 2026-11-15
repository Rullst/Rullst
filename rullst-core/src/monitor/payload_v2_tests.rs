//! The `rullst.monitor.v2` schema: its exact field set, its redaction and how
//! it is served next to version 1.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::collector::Collector;
use super::payload::{Application, Document};
use super::payload_tests::{paths, populated_collector, queues, radar};
use super::payload_v2::{DocumentV2, SpanKindReport, security_report, span_summary};
use super::test_support::{BASE64_TOKEN, bearer, get, read};
use super::{Installed, MONITOR_PATH, MONITOR_V2_PATH, Monitor};
use crate::config::Environment;
use crate::security::{LiveSecurityEvent, SecurityStore};
use crate::server::dev_telemetry::request_scope::RequestQueries;
use crate::telemetry_spans::TraceSpan;
use axum::Router;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;
use tower::ServiceExt;

fn span(name: &str, kind: &str, duration_us: u64, timestamp: u64) -> TraceSpan {
    TraceSpan {
        name: name.to_string(),
        kind: kind.to_string(),
        duration_us,
        timestamp,
    }
}

fn base(collector: &Collector) -> Document {
    Document::new(
        Some(Application {
            name: "shop".to_string(),
            version: "1.4.0".to_string(),
        }),
        radar(),
        collector,
        None,
        queues(),
    )
}

fn field_set(value: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    paths(value, "", &mut found);
    found
}

// TM-MONITOR-04: adding a field is a reviewed schema change.
#[test]
fn version_2_has_every_version_1_field_plus_traces_and_security() {
    let collector = populated_collector();
    let store = SecurityStore::new();
    store.record_rbac_denial("someone", "/admin");
    let spans = [span("GET /", "http", 10, 1)];
    let v1 = serde_json::to_value(base(&collector)).unwrap();
    let v2 = serde_json::to_value(DocumentV2::new(
        base(&collector),
        &collector,
        &spans,
        &store,
    ))
    .unwrap();
    assert_eq!(v1["schema"], "rullst.monitor.v1");
    assert_eq!(v2["schema"], "rullst.monitor.v2");
    let counters = [
        "cswsh_blocks",
        "deception_hits",
        "dlp_secrets_masked",
        "honeypot_traps",
        "idor_warnings",
        "log_redactions",
        "login_jail_bans",
        "mfa_verifications",
        "prompt_injections_blocked",
        "rate_limit_blocks",
        "rbac_denials",
        "sanitizations",
        "schema_violations",
        "secure_headers_applied",
        "siem_dispatches",
        "sri_signed_assets",
        "timing_guard_protected",
        "zero_trust_mismatches",
    ];
    let mut expected = field_set(&v1);
    expected.extend(
        [
            "traces",
            "traces.sample_limit",
            "traces.sampled_total",
            "traces.requests",
            "traces.requests[]",
            "traces.requests[].at",
            "traces.requests[].method",
            "traces.requests[].route",
            "traces.requests[].status",
            "traces.requests[].duration_ms",
            "traces.requests[].db_operations",
            "traces.requests[].n_plus_one",
            "traces.spans",
            "traces.spans.kept",
            "traces.spans.kinds",
            "traces.spans.kinds[]",
            "traces.spans.kinds[].kind",
            "traces.spans.kinds[].count",
            "traces.spans.kinds[].total_us",
            "traces.spans.kinds[].max_us",
            "traces.spans.kinds[].last_at",
            "security",
            "security.events_total",
            "security.active_bans",
            "security.counters",
            "security.events",
            "security.events[]",
            "security.events[].type",
            "security.events[].at",
        ]
        .map(str::to_string),
    );
    expected.extend(counters.map(|name| format!("security.counters.{name}")));
    assert_eq!(field_set(&v2), expected);
    let without_schema = |mut value: Value| {
        let map = value.as_object_mut().unwrap();
        for key in ["schema", "generated_at", "traces", "security"] {
            map.remove(key);
        }
        value
    };
    assert_eq!(without_schema(v2), without_schema(v1));
}

#[test]
fn request_summaries_report_labels_and_rounded_milliseconds() {
    let collector = Collector::new();
    let queries = RequestQueries {
        operations: 4,
        fingerprints: vec!["Post.find (posts)".to_string(); 3],
    };
    collector.record_at(
        Duration::ZERO,
        &Method::PATCH,
        Some("/posts/{id}"),
        409,
        Duration::from_nanos(1_234_567),
        &queries,
    );
    let document = DocumentV2::new(base(&collector), &collector, &[], &SecurityStore::new());
    let traces = serde_json::to_value(&document.traces).unwrap();
    let request = &traces["requests"][0];
    assert_eq!(traces["sample_limit"], 50);
    assert_eq!(traces["sampled_total"], 1);
    assert_eq!(request["method"], "PATCH");
    assert_eq!(request["route"], "/posts/{id}");
    assert_eq!(request["status"], 409);
    assert_eq!(request["duration_ms"], 1.235);
    assert_eq!(request["db_operations"], 4);
    assert_eq!(request["n_plus_one"], true);
    assert!(request["at"].as_u64().unwrap() > 1_700_000_000);
    assert_eq!(traces["spans"], json!({"kept": 0, "kinds": []}));
}

#[test]
fn span_totals_are_per_kind_and_never_carry_names() {
    let spans = [
        span(
            "SELECT * FROM users WHERE email = 'a@example.com'",
            "sql",
            300,
            20,
        ),
        span("GET /users/42?token=secret", "http", 50, 30),
        span("SELECT 1", "sql", 700, 10),
        span("prompt", "llm", 9, 40),
        span("other", "other", 1, 5),
    ];
    let summary = span_summary(&spans);
    assert_eq!(summary.kept, 5);
    assert_eq!(
        summary.kinds,
        [
            SpanKindReport {
                kind: "http",
                count: 1,
                total_us: 50,
                max_us: 50,
                last_at: 30
            },
            SpanKindReport {
                kind: "sql",
                count: 2,
                total_us: 1_000,
                max_us: 700,
                last_at: 20
            },
            SpanKindReport {
                kind: "other",
                count: 2,
                total_us: 10,
                max_us: 9,
                last_at: 40
            },
        ]
    );
    let encoded = serde_json::to_string(&summary).unwrap();
    for leaked in [
        "SELECT",
        "example.com",
        "/users/42",
        "secret",
        "prompt",
        "llm",
    ] {
        assert!(!encoded.contains(leaked), "{leaked} leaked");
    }
}

// TM-MONITOR-05: the radar carries no client address, user or path.
#[test]
fn the_security_radar_keeps_counters_and_event_types_only() {
    let store = SecurityStore::new();
    store.record_honeypot_trap("203.0.113.9", "/wp-admin/setup.php");
    store.record_rbac_denial("alice@example.com", "/invoices/7");
    store.inc_rate_limit_blocks();
    store.inc_login_jail_bans();
    let report = security_report(&store);
    assert_eq!(report.events_total, 2);
    assert_eq!(report.active_bans, 1);
    assert_eq!(report.counters.honeypot_traps, 1);
    assert_eq!(report.counters.rbac_denials, 1);
    assert_eq!(report.counters.rate_limit_blocks, 1);
    assert_eq!(report.counters.login_jail_bans, 1);
    let types: Vec<&str> = report
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert_eq!(types, ["RBAC_ACCESS_DENIED", "HONEYPOT_TRAP_TRIGGERED"]);
    assert!(report.events.iter().all(|event| event.at > 1_700_000_000));
    let encoded = serde_json::to_string(&report).unwrap();
    for leaked in [
        "203.0.113.9",
        "alice",
        "example.com",
        "wp-admin",
        "invoices",
    ] {
        assert!(!encoded.contains(leaked), "{leaked} leaked");
    }
}

#[test]
fn event_times_are_unix_seconds_and_the_total_outlives_the_buffer() {
    let store = SecurityStore::new();
    let mut event = LiveSecurityEvent::local("WAF_BLOCK", "details", "198.51.100.4");
    event.timestamp_str = "2026-11-15T00:00:00.000Z".to_string();
    for _ in 0..60 {
        store.push_local_event(event.clone());
    }
    let report = security_report(&store);
    assert_eq!(report.events_total, 60);
    assert_eq!(report.events.len(), 50);
    assert_eq!(report.events[0].event_type, "WAF_BLOCK");
    assert_eq!(report.events[0].at, 1_794_700_800);
}

fn app() -> Router {
    let installed = Installed::prepare(
        Monitor::new().with_token(BASE64_TOKEN),
        Environment::Development,
    )
    .unwrap()
    .unwrap();
    let router = Router::new().route("/", axum::routing::get(|| async { "home" }));
    installed.record(installed.mount(router))
}

async fn fetch(app: &Router, path: &str, token: bool) -> (StatusCode, Value) {
    let authorization = bearer(BASE64_TOKEN);
    let headers: &[(&str, &str)] = if token {
        &[(authorization.0, authorization.1.as_str())]
    } else {
        &[]
    };
    let response = app
        .clone()
        .oneshot(get(path, "192.0.2.10:40000", headers))
        .await
        .unwrap();
    let (status, body) = read(response).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn both_versions_are_served_with_the_same_credentials_and_are_not_counted() {
    let app = app();
    assert_eq!(
        fetch(&app, MONITOR_V2_PATH, false).await.0,
        StatusCode::UNAUTHORIZED
    );
    let home = get("/", "192.0.2.10:40000", &[]);
    assert_eq!(
        app.clone().oneshot(home).await.unwrap().status(),
        StatusCode::OK
    );
    let (status, v1) = fetch(&app, MONITOR_PATH, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v1["schema"], "rullst.monitor.v1");
    assert!(v1.get("traces").is_none() && v1.get("security").is_none());
    let (status, v2) = fetch(&app, MONITOR_V2_PATH, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v2["schema"], "rullst.monitor.v2");
    // Only `/` is counted: neither monitor path is.
    assert_eq!(v2["http"]["requests_total"], 1);
    assert_eq!(v2["traces"]["requests"][0]["route"], "/");
    assert!(v2["security"]["counters"].is_object());
}
