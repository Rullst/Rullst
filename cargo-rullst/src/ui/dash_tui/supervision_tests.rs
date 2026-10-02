#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The dashboard's view of the supervisor: restarts, busy states and the
//! supervised process generation.

use super::metrics::Source;
use super::metrics_render_tests::{GENERATION, live_app, screen};
use super::state::{App, ServerStatus};
use super::telemetry::{DatabaseReport, HttpReport, PollOutcome, QueueReport, TelemetrySnapshot};
use super::{handle_key, ingest_telemetry};
use crate::generators::dev::{DevCommand, DevState, DevStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Instant;

fn state(status: DevStatus, generation: &str) -> DevState {
    DevState {
        status,
        generation: Some(generation.to_string()),
        rebuilding: false,
    }
}

#[test]
fn restart_is_queued_once_for_a_running_application() {
    let (logs, _rx) = tokio::sync::mpsc::channel(4);
    let (commands, mut command_rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(3_000, true, "not configured".into(), false, false);
    let restart = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);

    app.server_status = ServerStatus::Starting;
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(command_rx.try_recv().is_err());
    assert!(
        app.action_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("still starting"))
    );

    app.server_status = ServerStatus::Ready;
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(matches!(command_rx.try_recv(), Ok(DevCommand::Restart)));
    assert!(command_rx.try_recv().is_err());
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("supervisor is busy"))
    );

    super::apply_state(
        &mut app,
        &state(
            DevStatus::Exited(exit_status(1)),
            "0123456789abcdef0123456789abcdef",
        ),
    );
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(matches!(command_rx.try_recv(), Ok(DevCommand::Restart)));
    let output = screen(&app, 120, 30, Instant::now());
    assert!(output.contains("Restarting the application"));
    // The supervisor logs the restart; the dashboard does not repeat it.
    assert!(
        !app.system_logs()
            .iter()
            .any(|line| line.contains("Restarting the application"))
    );
    // Statuses of the process being replaced do not end the notice...
    super::apply_state(&mut app, &state(DevStatus::Ready, GENERATION));
    assert!(app.action_notice.is_some());
    // ...the new generation's do, once it is no longer starting.
    super::apply_state(&mut app, &state(DevStatus::Starting, OTHER_GENERATION));
    assert!(app.action_notice.is_some());
    super::apply_state(&mut app, &state(DevStatus::Ready, OTHER_GENERATION));
    assert_eq!(app.server_status, ServerStatus::Ready);
    assert!(app.action_notice.is_none());
    app.action_notice = Some("API docs unavailable".to_string());
    super::apply_state(&mut app, &state(DevStatus::Ready, OTHER_GENERATION));
    assert!(app.action_notice.is_some());
}

#[cfg(unix)]
fn exit_status(code: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code << 8)
}

#[cfg(windows)]
fn exit_status(code: i32) -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code as u32)
}

const OTHER_GENERATION: &str = "fedcba9876543210fedcba9876543210";

#[test]
fn restart_is_refused_while_the_supervisor_is_busy_and_reports_its_outcome() {
    let (logs, _rx) = tokio::sync::mpsc::channel(4);
    let (commands, mut command_rx) = tokio::sync::mpsc::channel(1);
    let restart = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    let mut app = App::new(3_000, true, "not configured".into(), false, false);
    super::apply_state(&mut app, &state(DevStatus::Ready, GENERATION));

    // A migration has left the command channel empty but occupies the supervisor.
    app.migration_running = true;
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(command_rx.try_recv().is_err());
    assert!(
        app.action_notice
            .as_deref()
            .is_some_and(|n| n.contains("migrating"))
    );
    app.migration_running = false;

    let mut rebuilding = state(DevStatus::Ready, GENERATION);
    rebuilding.rebuilding = true;
    super::apply_state(&mut app, &rebuilding);
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(command_rx.try_recv().is_err());
    assert!(
        app.action_notice
            .as_deref()
            .is_some_and(|n| n.contains("rebuilding"))
    );
    super::apply_state(&mut app, &state(DevStatus::Ready, GENERATION));
    assert!(!app.rebuilding);

    // A restarted process that never becomes ready replaces the notice.
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(matches!(command_rx.try_recv(), Ok(DevCommand::Restart)));
    super::apply_state(&mut app, &state(DevStatus::Unverified, OTHER_GENERATION));
    assert!(
        app.action_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("readiness was not confirmed"))
    );

    // One that exits during startup too, even if Starting was never observed.
    assert!(!handle_key(restart, &mut app, &logs, &commands));
    assert!(matches!(command_rx.try_recv(), Ok(DevCommand::Restart)));
    super::apply_state(
        &mut app,
        &state(DevStatus::Exited(exit_status(101)), GENERATION),
    );
    assert!(
        app.action_notice
            .as_deref()
            .is_some_and(|notice| notice.contains("restarted application exited"))
    );
    assert!(app.pending_restart.is_none());
}

#[test]
fn another_process_on_the_port_is_named_instead_of_showing_its_metrics() {
    let (mut app, now) = live_app(false);
    let foreign = PollOutcome::Snapshot(TelemetrySnapshot {
        generation: OTHER_GENERATION.to_string(),
        http: HttpReport {
            requests_total: 900,
            client_errors_total: 0,
            server_errors_total: 0,
            recent: Vec::new(),
        },
        database: DatabaseReport::Unknown,
        queue: QueueReport::Unknown,
    });
    ingest_telemetry(&mut app, foreign, now);
    assert_eq!(app.metrics.source, Source::Foreign);
    let output = screen(&app, 120, 36, now);
    assert!(output.contains("ANOTHER PROCESS ON THE PORT"), "{output}");
    assert!(output.contains("127.0.0.1:3000 is answered by a process"));
    assert!(!output.contains("req/s"));
    assert!(
        app.system_logs()
            .iter()
            .any(|line| line.contains("did not start"))
    );
    assert!(screen(&app, 140, 22, now).contains("metrics: another process on the port"));

    // The generation the supervisor publishes makes the process its own.
    super::apply_state(&mut app, &state(DevStatus::Ready, OTHER_GENERATION));
    let own = PollOutcome::Snapshot(TelemetrySnapshot {
        generation: OTHER_GENERATION.to_string(),
        http: HttpReport {
            requests_total: 1,
            client_errors_total: 0,
            server_errors_total: 0,
            recent: Vec::new(),
        },
        database: DatabaseReport::Unknown,
        queue: QueueReport::Unknown,
    });
    ingest_telemetry(&mut app, own, now);
    assert_eq!(app.metrics.source, Source::Live);
}
