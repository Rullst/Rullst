//! `cargo rullst green --url` end to end in a child test process that serves
//! the app itself, so the process that owns the listening socket, its CPU
//! time and its output are known.
use super::super::*;
use super::support::{TestServer, between_markers, output_within};
use std::time::Duration;

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
    [[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\
    source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";

/// Runs `green --url <its own server> <RULLST_GREEN_ARGS>` when a parent test
/// names a report file; otherwise does nothing.
#[test]
fn green_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_GREEN_REPORT") else {
        return;
    };
    let server = TestServer::start();
    let arguments = std::env::var("RULLST_GREEN_ARGS").unwrap();
    let matches = command()
        .try_get_matches_from(
            ["green", "--url", server.base.as_str()]
                .into_iter()
                .chain(arguments.split('\u{1f}').filter(|word| !word.is_empty())),
        )
        .unwrap();
    let mut stdout = std::io::stdout();
    stdout.write_all(b"<<<green\n").unwrap();
    stdout.flush().unwrap();
    let outcome = match run(&matches) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    stdout.write_all(b"green>>>\n").unwrap();
    stdout.flush().unwrap();
    std::fs::write(report, format!("{}\n{outcome}", std::process::id())).unwrap();
}

/// The child's PID, the outcome of `run`, what it printed and its stderr.
fn green(arguments: &[&str]) -> (u32, String, String, String) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Cargo.lock"), LOCK).unwrap();
    let report = project.path().join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::green::tests::run::green_child_fixture",
            "--nocapture",
        ])
        .current_dir(project.path())
        .env("RULLST_GREEN_REPORT", &report)
        .env("RULLST_GREEN_ARGS", arguments.join("\u{1f}"))
        .env("NO_COLOR", "1");
    let output = output_within(command, Duration::from_secs(60));
    assert!(output.status.success(), "{output:?}");
    let report = std::fs::read_to_string(report).unwrap();
    let (pid, outcome) = report.split_once('\n').unwrap();
    (
        pid.parse().unwrap(),
        outcome.to_string(),
        between_markers(&output.stdout, "green"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn a_running_app_is_found_by_its_port_and_benchmarked_with_json_only_on_stdout() {
    let (pid, outcome, shown, stderr) = green(&["--bench", "--duration", "1s", "--json"]);
    assert_eq!(outcome, "ok");
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(report["schema"], "rullst.green.v1");
    assert_eq!(report["target"]["mode"], "existing_url");
    assert_eq!(report["target"]["pid"], pid);
    assert!(
        report["target"]["process_lookup"]
            .as_str()
            .unwrap()
            .starts_with("listening on port "),
        "{report}"
    );
    assert_eq!(report["target"]["environment"], serde_json::Value::Null);
    assert_eq!(report["measured"]["idle_rss_bytes"]["status"], "measured");
    assert_eq!(report["measured"]["dependency_count"]["value"], 1);
    assert_eq!(report["benchmark"]["status"], "run");
    assert_eq!(report["benchmark"]["path"], "/");
    assert_eq!(report["benchmark"]["concurrency"], 4);
    assert_eq!(
        report["benchmark"]["cpu_time_seconds"]["status"], "measured",
        "{report}"
    );
    assert!(report["benchmark"]["requests"].as_u64().unwrap() > 0);
    assert!(!stderr.contains("Benchmarking"), "{stderr}");
}

#[cfg(target_os = "linux")]
#[test]
fn the_terminal_report_announces_the_benchmark_on_stderr() {
    let (pid, outcome, shown, stderr) = green(&[
        "--bench",
        "--duration",
        "1s",
        "--concurrency",
        "1",
        "--path",
        "/missing",
    ]);
    assert_eq!(outcome, "ok");
    assert!(
        stderr.contains("Benchmarking GET /missing for 1 s with 1 connections..."),
        "{stderr}"
    );
    for expected in [
        "Rullst green · v",
        "Mode          existing app (--url)",
        &format!("Process       PID {pid}"),
        "Dependencies  1 ",
        "GET /missing · 1 s · 1 connections",
    ] {
        assert!(
            shown.contains(expected),
            "missing `{expected}` in:\n{shown}"
        );
    }
    assert!(!shown.trim_start().starts_with('{'), "{shown}");
}
