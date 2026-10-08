#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::metrics::Source;
use super::state::{App, ServerStatus};
use super::telemetry::{
    DatabaseReport, HttpReport, PollOutcome, QueueReport, RepeatedReport, RequestSample, SlowQuery,
    TelemetrySnapshot,
};
use super::{handle_key, ingest_telemetry, render};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use std::time::{Duration, Instant};

pub(super) const GENERATION: &str = "0123456789abcdef0123456789abcdef";

fn request(seq: u64, method: &str, path: &str, status: u16, duration_us: u64) -> RequestSample {
    RequestSample {
        seq,
        method: method.to_string(),
        path: path.to_string(),
        status,
        duration_us,
    }
}

fn snapshot(
    total: u64,
    recent: Vec<RequestSample>,
    database: DatabaseReport,
    queue: QueueReport,
) -> PollOutcome {
    let server_errors = recent.iter().filter(|r| r.status >= 500).count() as u64;
    let client_errors = recent
        .iter()
        .filter(|r| (400..500).contains(&r.status))
        .count() as u64;
    PollOutcome::Snapshot(TelemetrySnapshot {
        generation: GENERATION.to_string(),
        http: HttpReport {
            requests_total: total,
            client_errors_total: client_errors,
            server_errors_total: server_errors,
            recent,
        },
        database,
        repeated: RepeatedReport::NotReported,
        queue,
    })
}

fn observed_database() -> DatabaseReport {
    DatabaseReport::Observed {
        queries_total: 42,
        slow_total: 1,
        slow_threshold_ms: 100,
        recent_slow: vec![SlowQuery {
            seq: 1,
            operation: "select_many".to_string(),
            model: Some("Post".to_string()),
            table: Some("posts".to_string()),
            duration_us: 152_000,
        }],
    }
}

/// An application with two polls one second apart: four requests, one 5xx.
pub(super) fn live_app(colors: bool) -> (App, Instant) {
    let start = Instant::now();
    let mut app = App::new(3_000, true, "configured: SQLite".to_string(), colors, false);
    app.server_status = ServerStatus::Ready;
    app.metrics.own_generation(GENERATION);
    ingest_telemetry(
        &mut app,
        snapshot(
            1,
            vec![request(1, "GET", "/", 200, 900)],
            observed_database(),
            QueueReport::Observed { pending: 3 },
        ),
        start,
    );
    ingest_telemetry(
        &mut app,
        snapshot(
            4,
            vec![
                request(1, "GET", "/", 200, 900),
                request(2, "POST", "/orders", 201, 3_100),
                request(3, "GET", "/missing", 404, 450),
                request(4, "GET", "/reports/annual", 500, 48_000),
            ],
            observed_database(),
            QueueReport::Observed { pending: 3 },
        ),
        start + Duration::from_secs(1),
    );
    (app, start + Duration::from_secs(1))
}

fn terminal(app: &App, width: u16, height: u16, now: Instant) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render::ui_at(frame, app, now))
        .unwrap();
    terminal
}

fn rows(app: &App, width: u16, height: u16, now: Instant) -> Vec<String> {
    let terminal = terminal(app, width, height, now);
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

pub(super) fn screen(app: &App, width: u16, height: u16, now: Instant) -> String {
    rows(app, width, height, now).join("\n")
}

#[test]
fn the_live_metrics_row_matches_its_snapshot() {
    let (app, now) = live_app(false);
    let rows = rows(&app, 120, 36, now);
    // Two polls one second apart; the p95 history is [900 µs, 48 ms].
    let expected = [
        " ╭ TRAFFIC ─────────────────────────────╮╭ RECENT REQUESTS ──────────────────────────╮╭ DATABASE · QUEUE ─────────────╮",
        " │ req/s    3.0  last 10 s              ││ 500 GET       48.0 ms  /reports/annual    ││ queries  42  ORM, since start │",
        " │ requests 4  4xx 1  5xx 1             ││ 404 GET        450 µs  /missing           ││ slow     1  ≥ 100 ms          │",
        " │ errors   1 (33.3%)  5xx of 3 in 60 s ││ 201 POST       3.1 ms  /orders            ││   Post.select_many 152 ms     │",
        " │ latency  p50 900 µs · p95 48.0 ms    ││ 200 GET        900 µs  /                  ││ queue    3  pending           │",
        " │                                    █ ││                                           ││                               │",
        " │                                    █ ││                                           ││                               │",
        " │                                    █ ││                                           ││                               │",
        " │ p95/poll                          ▂█ ││                                           ││                               │",
        " ╰──────────────────────────────────────╯╰───────────────────────────────────────────╯╰───────────────────────────────╯",
    ];
    assert_eq!(rows[5..15], expected, "\n{}", rows.join("\n"));
}

#[test]
fn compact_terminals_keep_traffic_and_database_panels() {
    let (app, now) = live_app(false);
    let output = screen(&app, 100, 30, now);
    for expected in [
        "TRAFFIC",
        "req/s",
        "p95 48.0 ms",
        "DATABASE · QUEUE",
        "queries  42",
        "errors   1 (33.3%)",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
    assert!(!output.contains("RECENT REQUESTS"));
    assert!(output.contains("[r] restart"));
    assert!(output.contains("[?] help"));
}

#[test]
fn short_terminals_summarize_metrics_in_the_header() {
    let (app, now) = live_app(false);
    let output = screen(&app, 140, 22, now);
    assert!(!output.contains("TRAFFIC"));
    assert!(
        output.contains("3.0 req/s · p95 48.0 ms · 5xx 1"),
        "{output}"
    );

    let mut missing = App::new(3_000, true, "not configured".into(), false, false);
    ingest_telemetry(&mut missing, PollOutcome::NotServed, now);
    assert!(screen(&missing, 140, 22, now).contains("metrics: not available (?)"));
}

#[test]
fn a_missing_endpoint_shows_how_to_enable_telemetry_instead_of_numbers() {
    let now = Instant::now();
    let mut app = App::new(3_000, true, "not configured".into(), false, false);
    ingest_telemetry(&mut app, PollOutcome::NotServed, now);
    let output = screen(&app, 150, 36, now);
    for expected in [
        "TELEMETRY NOT AVAILABLE",
        "does not serve GET /_rullst/dev-telemetry",
        "How to enable:",
        "rullst::Server",
        "RULLST_ENV=development",
        "press r to restart",
        "cli_reference.html#cargo-rullst-dash",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
    assert!(!output.contains("req/s"));
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("Telemetry not available"))
    );

    // Earlier numbers are hidden too once the endpoint disappears.
    let (mut live, now) = live_app(false);
    ingest_telemetry(&mut live, PollOutcome::Rejected("unsupported schema"), now);
    let output = screen(&live, 150, 36, now);
    assert!(output.contains("rejected (unsupported schema)"));
    assert!(!output.contains("req/s"));
}

#[test]
fn waiting_and_paused_states_are_labelled() {
    let now = Instant::now();
    let mut app = App::new(4_100, true, "not configured".into(), false, false);
    assert!(screen(&app, 120, 36, now).contains("Waiting for the application on 127.0.0.1:4100"));
    ingest_telemetry(&mut app, PollOutcome::Unreachable, now);
    assert_eq!(app.metrics.source, Source::Unreachable);
    assert!(screen(&app, 120, 36, now).contains("Waiting for the application"));

    let (mut live, now) = live_app(false);
    ingest_telemetry(&mut live, PollOutcome::Unreachable, now);
    let output = screen(&live, 120, 36, now);
    assert!(output.contains("TRAFFIC · paused"));
    assert!(
        live.system_logs()
            .iter()
            .any(|line| line.contains("Live metrics paused"))
    );
}

#[test]
fn unreported_database_and_queue_values_say_how_to_report_them() {
    let now = Instant::now();
    for (database, queue, expected) in [
        (
            DatabaseReport::SubscriberNotInstalled,
            QueueReport::NotConfigured,
            ["custom tracing subscriber", "Server::with_dev_queue(q)"],
        ),
        (
            DatabaseReport::SpansFiltered,
            QueueReport::Timeout,
            ["RUST_LOG hides rullst_orm", "no answer in 250 ms"],
        ),
        (
            DatabaseReport::Unknown,
            QueueReport::DriverError,
            ["not in this app version", "see the application logs"],
        ),
    ] {
        let mut app = App::new(3_000, true, "not configured".into(), false, false);
        app.metrics.own_generation(GENERATION);
        ingest_telemetry(&mut app, snapshot(0, Vec::new(), database, queue), now);
        let output = screen(&app, 140, 36, now);
        for text in expected {
            assert!(output.contains(text), "missing {text}:\n{output}");
        }
        assert!(output.contains("not reported"));
        assert!(output.contains("No requests yet"));
    }
}

#[test]
fn colors_follow_status_and_no_color_renders_plain_cells() {
    let (plain, now) = live_app(false);
    let plain = terminal(&plain, 120, 36, now);
    assert!(
        plain
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|cell| cell.fg == Color::Reset)
    );

    let (colored, now) = live_app(true);
    let colored = terminal(&colored, 120, 36, now);
    let buffer = colored.backend().buffer();
    // Row 6 holds the newest request, the 500 response.
    let red = (0..120_u16)
        .any(|x| buffer[(x, 6)].symbol() == "5" && buffer[(x, 6)].fg == Color::Rgb(255, 70, 95));
    assert!(red, "the 5xx request should be drawn in red");
}

#[test]
fn help_opens_with_question_mark_and_any_key_closes_it() {
    let (logs, _rx) = tokio::sync::mpsc::channel(4);
    let (commands, _command_rx) = tokio::sync::mpsc::channel(1);
    let (mut app, now) = live_app(false);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);

    assert!(!handle_key(
        key(KeyCode::Char('?')),
        &mut app,
        &logs,
        &commands
    ));
    assert!(app.show_help);
    let output = screen(&app, 120, 36, now);
    for expected in [
        "HELP",
        "restart the application (same build)",
        "quit and stop the application",
        "/_rullst/dev-telemetry",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
    // Esc closes the help instead of quitting.
    assert!(!handle_key(key(KeyCode::Esc), &mut app, &logs, &commands));
    assert!(!app.show_help);
    assert!(!screen(&app, 120, 36, now).contains("restart the application (same build)"));

    assert!(!handle_key(
        key(KeyCode::Char('?')),
        &mut app,
        &logs,
        &commands
    ));
    assert!(handle_key(
        key(KeyCode::Char('q')),
        &mut app,
        &logs,
        &commands
    ));
}

#[test]
fn width_thresholds_are_terminal_widths() {
    let (app, now) = live_app(false);
    // The recent-requests panel needs a 105-column terminal.
    assert!(screen(&app, 105, 36, now).contains("RECENT REQUESTS"));
    assert!(!screen(&app, 104, 36, now).contains("RECENT REQUESTS"));
    // The footer lists d and Tab from 120 columns.
    let full = screen(&app, 120, 36, now);
    assert!(full.contains("[d] api docs"), "{full}");
    assert!(full.contains("[tab] focus"));
    let short = screen(&app, 119, 36, now);
    assert!(!short.contains("[d] api docs"));
    assert!(!short.contains("[tab] focus"));
}
