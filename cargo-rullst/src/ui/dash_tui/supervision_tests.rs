#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The dashboard's view of the supervisor: the supervised process generation.

use super::ingest_telemetry;
use super::metrics::Source;
use super::metrics_render_tests::{live_app, screen};
use super::telemetry::{DatabaseReport, HttpReport, PollOutcome, QueueReport, TelemetrySnapshot};
use crate::generators::dev::{DevState, DevStatus};

fn state(status: DevStatus, generation: &str) -> DevState {
    DevState {
        status,
        generation: Some(generation.to_string()),
    }
}

const OTHER_GENERATION: &str = "fedcba9876543210fedcba9876543210";

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
