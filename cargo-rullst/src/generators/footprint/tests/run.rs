//! `cargo rullst footprint --url` end to end in a child test process that serves
//! the app itself, so the process that owns the listening socket, its CPU
//! time and its output are known.
// The end-to-end tests that use these helpers measure /proc and run on Linux
// only; on macOS and Windows the helpers are compiled but unused.
#![cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))]
use super::super::*;
use super::support::{TestServer, between_markers, output_within};
use std::time::Duration;

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
    [[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\
    source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";

/// Runs `footprint --url <its own server> <RULLST_FOOTPRINT_ARGS>` when a parent test
/// names a report file (`RULLST_FOOTPRINT_URL` replaces the server's URL);
/// otherwise does nothing.
#[test]
fn footprint_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_FOOTPRINT_REPORT") else {
        return;
    };
    let server = TestServer::start();
    let url = std::env::var("RULLST_FOOTPRINT_URL").unwrap_or_else(|_| server.base.to_string());
    let arguments = std::env::var("RULLST_FOOTPRINT_ARGS").unwrap();
    let matches = command()
        .try_get_matches_from(
            ["footprint", "--url", url.as_str()]
                .into_iter()
                .chain(arguments.split('\u{1f}').filter(|word| !word.is_empty())),
        )
        .unwrap();
    let mut stdout = std::io::stdout();
    stdout.write_all(b"<<<footprint\n").unwrap();
    stdout.flush().unwrap();
    let outcome = match run(&matches) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    stdout.write_all(b"footprint>>>\n").unwrap();
    stdout.flush().unwrap();
    std::fs::write(report, format!("{}\n{outcome}", std::process::id())).unwrap();
}

/// The child's PID, the outcome of `run`, what it printed and its stderr.
fn footprint(arguments: &[&str]) -> (u32, String, String, String) {
    footprint_at(None, arguments)
}

/// [`footprint`] measuring `url` instead of the child's own server.
fn footprint_at(url: Option<&str>, arguments: &[&str]) -> (u32, String, String, String) {
    footprint_with(&[], &[], url, arguments)
}

/// [`footprint_at`] in a project that also holds `files`, with `variables`
/// in the child's environment.
pub(super) fn footprint_with(
    files: &[(&str, &str)],
    variables: &[(&str, &str)],
    url: Option<&str>,
    arguments: &[&str],
) -> (u32, String, String, String) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("Cargo.lock"), LOCK).unwrap();
    for (name, text) in files {
        std::fs::write(project.path().join(name), text).unwrap();
    }
    let report = project.path().join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::footprint::tests::run::footprint_child_fixture",
            "--nocapture",
        ])
        .current_dir(project.path())
        .env("RULLST_FOOTPRINT_REPORT", &report)
        .env("RULLST_FOOTPRINT_ARGS", arguments.join("\u{1f}"))
        .env("NO_COLOR", "1");
    if let Some(url) = url {
        command.env("RULLST_FOOTPRINT_URL", url);
    }
    command.env_remove("RULLST_MONITOR_TOKEN");
    for (name, value) in variables {
        command.env(name, value);
    }
    let output = output_within(command, Duration::from_secs(60));
    assert!(output.status.success(), "{output:?}");
    let report = std::fs::read_to_string(report).unwrap();
    let (pid, outcome) = report.split_once('\n').unwrap();
    (
        pid.parse().unwrap(),
        outcome.to_string(),
        between_markers(&output.stdout, "footprint"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn a_running_app_is_found_by_its_port_and_benchmarked_with_json_only_on_stdout() {
    let (pid, outcome, shown, stderr) = footprint(&["--bench", "--duration", "1s", "--json"]);
    assert_eq!(outcome, "ok");
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(report["schema"], "rullst.footprint.v1");
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
    let requests = report["benchmark"]["requests"].as_u64().unwrap();
    assert!(requests > 0);
    assert!(!stderr.contains("Benchmarking"), "{stderr}");
    // Every "ok" body is two bytes, served uncompressed.
    assert_eq!(
        report["benchmark"]["response_bytes"]["value"].as_u64(),
        Some(2 * requests)
    );
    assert_eq!(report["benchmark"]["bytes_per_request"]["value"], 2.0);
    assert_eq!(
        report["benchmark"]["content_encodings"],
        serde_json::json!(["identity"])
    );
    assert_eq!(report["page_weight"]["status"], "not_run");
}

#[test]
fn the_page_is_weighed_before_the_benchmark() {
    let (_, outcome, shown, stderr) = footprint(&["--page-weight", "--json"]);
    assert_eq!(outcome, "ok");
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    let page = &report["page_weight"];
    assert_eq!(page["status"], "measured", "{report}");
    assert_eq!(page["first_visit"]["requests"], 1);
    assert_eq!(page["first_visit"]["bytes"], 2);
    assert_eq!(page["repeat_visit"]["requests"], 1);
    assert_eq!(page["resources"][0]["url"], "/");
    assert_eq!(report["benchmark"]["status"], "not_run");
    assert!(!stderr.contains("Weighing"), "{stderr}");

    let (_, outcome, shown, stderr) = footprint(&["--page-weight", "--path", "/missing"]);
    assert_eq!(outcome, "ok");
    assert!(
        stderr.contains("Weighing GET /missing and the resources it references..."),
        "{stderr}"
    );
    for expected in [
        "Page weight\n  First visit   7 B",
        "1 requests, GET /missing and its resources, as served (uncompressed)",
        "Repeat visit  7 B",
        "Third party   none",
    ] {
        assert!(
            shown.contains(expected),
            "missing `{expected}` in:\n{shown}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_terminal_report_announces_the_benchmark_on_stderr() {
    let (pid, outcome, shown, stderr) = footprint(&[
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
        "Rullst footprint · v",
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

/// Nothing answers on the port: the JSON report still carries the figures
/// that need no running app, with the failure inside, and run fails.
#[test]
fn an_unreachable_app_still_reports_the_static_figures_and_the_failure() {
    let port = super::super::target::free_port().unwrap();
    let url = format!("http://127.0.0.1:{port}");
    let (_, outcome, shown, stderr) =
        footprint_at(Some(&url), &["--bench", "--page-weight", "--json"]);
    assert_eq!(outcome, format!("Nothing answered on {url}/."));
    let report: serde_json::Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(report["schema"], "rullst.footprint.v1");
    assert_eq!(report["measured"]["dependency_count"]["value"], 1);
    assert_eq!(report["failure"]["stage"], "reach");
    assert_eq!(report["failure"]["title"], "App not reachable");
    assert_eq!(report["target"]["url"], format!("{url}/"));
    assert_eq!(report["target"]["pid"], serde_json::Value::Null);
    for reason in [
        &report["measured"]["binary_size_bytes"]["reason"],
        &report["measured"]["idle_rss_bytes"]["reason"],
        &report["benchmark"]["cpu_time_seconds"]["reason"],
    ] {
        assert_eq!(reason, "the app was not reachable", "{report}");
    }
    assert_eq!(report["benchmark"]["status"], "not_run");
    assert_eq!(report["page_weight"]["status"], "not_measured");
    assert_eq!(report["page_weight"]["reason"], "the app was not reachable");
    assert!(stderr.contains("App not reachable"), "{stderr}");
    assert!(!stderr.contains("Benchmarking"), "{stderr}");

    let (_, outcome, shown, _) = footprint_at(Some(&url), &[]);
    assert_eq!(outcome, format!("Nothing answered on {url}/."));
    for expected in [
        "Dependencies  1 ",
        "Process       NOT RUNNING",
        "Not measured\n  App not reachable: the app was not reachable",
    ] {
        assert!(
            shown.contains(expected),
            "missing `{expected}` in:\n{shown}"
        );
    }
}
