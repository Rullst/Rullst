//! Possible N+1 findings: parsing, the warning panel and the state shown when
//! the application's telemetry cannot correlate queries with requests.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::metrics_render_tests::{GENERATION, live_app, screen};
use super::state::App;
use super::telemetry::{
    DatabaseReport, HttpReport, MAX_REPEATED, PollOutcome, QueueReport, RepeatedQuery,
    RepeatedReport, TelemetrySnapshot, parse_payload,
};
use super::{ingest_telemetry, render};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn document(database: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": "rullst.dev-telemetry.v1",
        "generation": GENERATION,
        "http": {"requests_total": 0, "client_errors_total": 0, "server_errors_total": 0},
        "database": database,
        "queue": {"state": "not_configured"}
    }))
    .unwrap()
}

#[test]
fn reported_findings_are_validated_and_cleaned() {
    let mut recent = vec![
        // Below the reported threshold, or beyond the counter: dropped.
        json!({"seq": 1, "method": "GET", "route": "/a", "fingerprint": "A.find (a)", "occurrences": 2}),
        json!({"seq": 9, "method": "GET", "route": "/b", "fingerprint": "B.find (b)", "occurrences": 5}),
        json!({"seq": 2, "method": "GET", "route": "/posts/{id}\u{1b}[2J",
               "fingerprint": "Post.find (posts)", "occurrences": 12}),
    ];
    recent.extend((0..MAX_REPEATED).map(|_| {
        json!({"seq": 3, "method": "GET", "route": "/c", "fingerprint": "C.find (c)", "occurrences": 3})
    }));
    let body = document(json!({
        "state": "observed",
        "queries_total": 40,
        "slow_queries_total": 0,
        "slow_threshold_ms": 100,
        "repeated_threshold": 3,
        "repeated_queries_total": 3,
        "recent_repeated": recent,
    }));
    let snapshot = parse_payload(&body).unwrap();
    let RepeatedReport::Observed {
        threshold,
        total,
        recent,
    } = snapshot.repeated
    else {
        panic!("repetitions should be observed");
    };
    assert_eq!((threshold, total), (3, 3));
    // Only the newest MAX_REPEATED entries are read.
    assert_eq!(recent.len(), MAX_REPEATED);
    assert!(
        recent
            .iter()
            .all(|finding| finding.fingerprint == "C.find (c)")
    );

    let body = document(json!({
        "state": "observed",
        "queries_total": 40,
        "slow_queries_total": 0,
        "slow_threshold_ms": 100,
        "repeated_threshold": 3,
        "repeated_queries_total": 2,
        "recent_repeated": [
            {"seq": 1, "method": "GET", "route": "/a", "fingerprint": "A.find (a)", "occurrences": 2},
            {"seq": 2, "method": "GET", "route": "/posts/{id}\u{1b}[2J",
             "fingerprint": "Post.find (posts)", "occurrences": 12}
        ],
    }));
    let RepeatedReport::Observed { recent, .. } = parse_payload(&body).unwrap().repeated else {
        panic!("repetitions should be observed");
    };
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].route, "/posts/{id}?[2J");
    assert_eq!(recent[0].occurrences, 12);
}

#[test]
fn telemetry_without_request_correlation_is_not_reported() {
    let older_core = document(json!({
        "state": "observed",
        "queries_total": 40,
        "slow_queries_total": 0,
        "slow_threshold_ms": 100,
    }));
    assert_eq!(
        parse_payload(&older_core).unwrap().repeated,
        RepeatedReport::NotReported
    );
    let unavailable = document(json!({"state": "unavailable", "reason": "orm_spans_filtered"}));
    assert_eq!(
        parse_payload(&unavailable).unwrap().repeated,
        RepeatedReport::NotReported
    );
}

fn with_findings(colors: bool) -> (App, Instant) {
    let (mut app, now) = live_app(colors);
    let now = now + Duration::from_secs(1);
    ingest_telemetry(
        &mut app,
        PollOutcome::Snapshot(TelemetrySnapshot {
            generation: GENERATION.to_string(),
            http: HttpReport {
                requests_total: 4,
                client_errors_total: 1,
                server_errors_total: 1,
                recent: Vec::new(),
            },
            database: DatabaseReport::Observed {
                queries_total: 60,
                slow_total: 1,
                slow_threshold_ms: 100,
                recent_slow: Vec::new(),
            },
            repeated: RepeatedReport::Observed {
                threshold: 3,
                total: 1,
                recent: vec![RepeatedQuery {
                    seq: 1,
                    method: "GET".to_string(),
                    route: "/posts/{id}".to_string(),
                    fingerprint: "Comment.select_many (comments)".to_string(),
                    occurrences: 12,
                }],
            },
            queue: QueueReport::Observed { pending: 3 },
        }),
        now,
    );
    (app, now)
}

#[test]
fn the_warning_panel_shows_route_fingerprint_count_hint_and_docs_in_plain_text() {
    let (app, now) = with_findings(false);
    let output = screen(&app, 120, 44, now);
    for expected in [
        "POSSIBLE N+1 QUERIES · ≥3 identical ORM operations in one request",
        "▲ GET /posts/{id}  Comment.select_many (comments)  ×12 in one request",
        "Hint: load the related rows eagerly or batch the lookups into one query.",
        render::N_PLUS_ONE_DOCS_URL,
        "1 possible N+1 since start",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }

    let mut terminal = Terminal::new(TestBackend::new(120, 44)).unwrap();
    terminal
        .draw(|frame| render::ui_at(frame, &app, now))
        .unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|cell| cell.fg == Color::Reset)
    );
}

#[test]
fn without_findings_no_panel_is_drawn_and_the_inspector_names_the_missing_telemetry() {
    // `live_app` polls carry no request-correlated ORM data.
    let (app, now) = live_app(false);
    let output = screen(&app, 120, 44, now);
    assert!(!output.contains("POSSIBLE N+1"));
    for expected in [
        "N+1 check     N+1 detection needs",
        "request-correlated ORM telemetry",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }

    let (mut app, now) = with_findings(false);
    app.metrics.repeated = Some(RepeatedReport::Observed {
        threshold: 3,
        total: 0,
        recent: Vec::new(),
    });
    let output = screen(&app, 120, 44, now);
    assert!(!output.contains("POSSIBLE N+1"));
    assert!(output.contains("none yet (≥3 per request)"), "{output}");

    app.metrics.repeated = Some(RepeatedReport::NotReported);
    app.metrics.database = Some(DatabaseReport::SpansFiltered);
    let output = screen(&app, 120, 44, now);
    assert!(output.contains("N+1 detection needs ORM"), "{output}");
}
