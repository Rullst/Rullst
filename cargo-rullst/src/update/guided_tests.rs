use super::*;
use std::collections::VecDeque;

#[derive(Default)]
struct Scripted {
    answers: VecDeque<bool>,
    shown: Vec<Value>,
    prompts: Vec<String>,
}
impl Ui for Scripted {
    fn show(&mut self, report: &Value) -> Result<()> {
        self.shown.push(report.clone());
        Ok(())
    }
    fn confirm(&mut self, prompt: &str) -> Result<bool> {
        self.prompts.push(prompt.into());
        Ok(self.answers.pop_front().unwrap_or(false))
    }
}

fn options() -> Options {
    let matches = command()
        .try_get_matches_from([
            "guided",
            "--to",
            env!("CARGO_PKG_VERSION"),
            "--project",
            "project with 'quotes' $(literal)",
        ])
        .unwrap();
    Options::parse_with(&matches, None).unwrap()
}

fn reply(args: &[OsString]) -> Result<Value> {
    // Every composed command must pass the production parser, including all
    // independent consent flags. These reports carry no external trust evidence.
    super::super::command().try_get_matches_from(
        std::iter::once(OsString::from("update")).chain(args.iter().cloned()),
    )?;
    let words = args
        .iter()
        .map(|word| word.to_str().unwrap())
        .collect::<Vec<_>>();
    Ok(match words.as_slice() {
        ["project", "prepare", ..] => {
            json!({"prepared_directory":"/private/preparation with spaces"})
        }
        ["project", "verify", ..] if words.contains(&"--dry-run") => {
            json!({"commands":["cargo check","cargo test"],"execution_authorized":false})
        }
        ["project", "verify", ..] => {
            json!({"verified_directory":"/private/verification with spaces"})
        }
        ["project", "review", ..] => json!({"review_sha256":"b".repeat(64),"diff":"-old\n+new"}),
        ["project", "apply", ..] => json!({"applied":true}),
        _ => return Err("unexpected composed command".into()),
    })
}

#[test]
fn decline_and_closed_input_never_reach_the_next_mutating_stage() {
    for (answers, expected) in [
        (vec![], 0),
        (vec![true, false, false], 2),
        (vec![true, false, true, false], 4),
    ] {
        let mut ui = Scripted {
            answers: answers.into(),
            ..Default::default()
        };
        let mut calls = Vec::new();
        flow(&options(), &mut ui, |args| {
            calls.push(args.to_vec());
            reply(args)
        })
        .unwrap();
        assert_eq!(calls.len(), expected);
        assert_eq!(ui.shown.last().unwrap()["stopped"], true);
        assert!(
            calls
                .iter()
                .all(|args| args.get(1).is_none_or(|arg| arg != "apply"))
        );
    }
}

#[test]
fn the_project_flow_reuses_exact_paths_and_the_review_digest_without_shell_interpolation() {
    let mut ui = Scripted {
        answers: vec![true, false, true, true].into(),
        ..Default::default()
    };
    let mut calls = Vec::new();
    flow(&options(), &mut ui, |args| {
        calls.push(args.to_vec());
        reply(args)
    })
    .unwrap();
    assert_eq!(calls.len(), 5);
    assert!(calls[0].contains(&OsString::from("project with 'quotes' $(literal)")));
    assert!(calls[1].contains(&OsString::from("--dry-run")));
    assert!(!calls[1].contains(&OsString::from("--allow-project-code")));
    assert!(calls[2].contains(&OsString::from("--allow-project-code")));
    assert!(!calls[2].contains(&OsString::from("--allow-network")));
    assert_eq!(calls[4].last().unwrap(), &OsString::from("b".repeat(64)));
    assert_eq!(ui.shown.last().unwrap()["completed"], "project");
    assert_eq!(
        ui.shown
            .iter()
            .find_map(|v| v.get("recovery_args"))
            .unwrap()[0],
        "project"
    );
}

#[test]
fn failed_verification_stops_before_review_or_application_and_does_not_consume_approval() {
    let mut ui = Scripted {
        answers: vec![true, true, true, true].into(),
        ..Default::default()
    };
    let mut calls = Vec::new();
    assert!(
        flow(&options(), &mut ui, |args| {
            calls.push(args.to_vec());
            if args.contains(&OsString::from("--allow-project-code")) {
                return Err("project test failed".into());
            }
            reply(args)
        })
        .is_err()
    );
    assert_eq!(calls.len(), 3);
    assert!(calls[1].contains(&OsString::from("--allow-network")));
    assert!(calls[2].contains(&OsString::from("--allow-network")));
    assert_eq!(ui.answers.len(), 1);
}

#[test]
fn offline_project_never_offers_network_and_malformed_results_stop_before_apply() {
    let mut config = options();
    config.offline = true;
    let mut ui = Scripted {
        answers: vec![true, true, true].into(),
        ..Default::default()
    };
    flow(&config, &mut ui, reply).unwrap();
    assert_eq!(ui.prompts.len(), 3);
    let mut ui = Scripted {
        answers: vec![true, true].into(),
        ..Default::default()
    };
    assert!(flow(&options(), &mut ui, |_| Ok(json!({"wrong":"result"}))).is_err());
    assert_eq!(ui.answers.len(), 1);
}

#[test]
fn a_major_jump_stops_before_any_step_and_cli_installation_flags_are_gone() {
    let major = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .major
        + 1;
    let matches = command()
        .try_get_matches_from(["guided", "--to", &format!("{major}.0.0")])
        .unwrap();
    assert!(
        Options::parse(&matches)
            .unwrap_err()
            .to_string()
            .contains("target major's migration rules")
    );
    for flag in ["--scope", "--root", "--allow-major", "--prerelease"] {
        assert!(
            command()
                .try_get_matches_from(["guided", "--to", "13.0.0", flag, "x"])
                .is_err(),
            "{flag}"
        );
    }
}

#[test]
fn prompts_need_a_terminal_on_both_standard_streams() {
    assert!(require_terminal(true, true).is_ok());
    for (stdin, stdout) in [(true, false), (false, true), (false, false)] {
        let error = require_terminal(stdin, stdout).unwrap_err().to_string();
        assert!(
            error.starts_with(
                "guided update rejected: guided updates require an interactive terminal;"
            ),
            "{stdin} {stdout}: {error}"
        );
    }
}

#[test]
fn offline_comes_from_the_flag_or_cargo_net_offline() {
    let offline = |flag: bool, variable: Option<&str>| {
        let mut words = vec!["guided", "--to", env!("CARGO_PKG_VERSION")];
        if flag {
            words.push("--offline");
        }
        let matches = command().try_get_matches_from(words).unwrap();
        Options::parse_with(&matches, variable.map(std::ffi::OsStr::new))
            .unwrap()
            .offline
    };
    assert!(!offline(false, None));
    assert!(!offline(false, Some("false")));
    assert!(offline(true, None));
    assert!(offline(false, Some("true")));
    assert!(offline(true, Some("1")));
}

#[test]
fn only_project_operations_are_executed() {
    let empty = tempfile::tempdir().unwrap();
    let words = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
    let prepare = words(&[
        "project",
        "prepare",
        "--project",
        empty.path().to_str().unwrap(),
        "--to",
        "13.0.0+build",
    ]);
    assert_eq!(
        execute(&prepare).unwrap_err().to_string(),
        "project preparation rejected: build metadata and oversized versions are unsupported"
    );
    assert_eq!(
        execute(&words(&["guided", "--to", "13.0.0"]))
            .unwrap_err()
            .to_string(),
        "guided update rejected: unsupported guided operation"
    );
}

/// Runs one terminal step when a parent test names a report file; otherwise
/// does nothing. Markers separate the step's output from the harness's.
#[test]
fn guided_child_fixture() {
    use std::io::Write;
    let Some(report) = std::env::var_os("RULLST_GUIDED_REPORT") else {
        return;
    };
    let mut stdout = std::io::stdout();
    stdout.write_all(b"<<<guided\n").unwrap();
    stdout.flush().unwrap();
    let outcome = match std::env::var("RULLST_GUIDED_MODE").unwrap().as_str() {
        "show" => Terminal
            .show(&json!({"path": "a\u{1b}[2Jb"}))
            .map(|()| "shown".to_string()),
        "confirm" => Terminal
            .confirm("Proceed?")
            .map(|answer| format!("answered {answer}")),
        "run" => {
            let matches = command()
                .try_get_matches_from(["guided", "--to", env!("CARGO_PKG_VERSION")])
                .unwrap();
            run(&matches).map(|()| "ran".to_string())
        }
        other => panic!("unknown mode {other}"),
    };
    stdout.write_all(b"guided>>>\n").unwrap();
    stdout.flush().unwrap();
    let outcome = outcome.unwrap_or_else(|error| format!("error: {error}"));
    std::fs::write(report, outcome).unwrap();
}

/// The outcome of `mode` in a child test process whose standard streams are
/// not terminals, and what it printed.
fn in_child(mode: &str) -> (String, String) {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("report");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "update::guided::tests::guided_child_fixture",
            "--nocapture",
        ])
        .current_dir(directory.path())
        .env("RULLST_GUIDED_REPORT", &report)
        .env("RULLST_GUIDED_MODE", mode)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let shown = stdout
        .split("<<<guided\n")
        .nth(1)
        .and_then(|rest| rest.split("guided>>>\n").next())
        .unwrap()
        .to_string();
    (std::fs::read_to_string(report).unwrap(), shown)
}

#[test]
fn the_terminal_shows_reports_as_escaped_json() {
    let (outcome, shown) = in_child("show");
    assert_eq!(outcome, "shown");
    assert_eq!(shown, "{\n  \"path\": \"a\\u001b[2Jb\"\n}\n");
}

#[test]
fn without_a_terminal_nothing_is_approved_or_run() {
    let (outcome, shown) = in_child("confirm");
    assert_eq!(outcome, "error: IO error: not a terminal");
    assert_eq!(shown, "");
    let (outcome, shown) = in_child("run");
    assert!(
        outcome.starts_with(
            "error: guided update rejected: guided updates require an interactive terminal;"
        ),
        "{outcome}"
    );
    assert_eq!(shown, "");
}
