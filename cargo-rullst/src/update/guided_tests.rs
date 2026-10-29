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

fn options(scope: &str) -> Options {
    let prerelease = (!semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .pre
        .is_empty())
    .then_some("--prerelease");
    let matches = command()
        .try_get_matches_from(
            [
                "guided",
                "--to",
                env!("CARGO_PKG_VERSION"),
                "--scope",
                scope,
                "--root",
                "/private/cli with spaces",
                "--project",
                "project with 'quotes' $(literal)",
            ]
            .into_iter()
            .chain(prerelease),
        )
        .unwrap();
    Options::parse(&matches).unwrap()
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
        ["stage", ..] => json!({"directory":"/private/download with spaces"}),
        ["install", "review", ..] => {
            json!({"review_sha256":"a".repeat(64),"review":{"root":"/private/cli with spaces"}})
        }
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
        ["install" | "project", "apply", ..] => json!({"applied":true}),
        _ => return Err("unexpected composed command".into()),
    })
}

#[test]
fn decline_and_closed_input_never_reach_the_next_mutating_stage() {
    for (scope, answers, expected) in [
        ("cli", vec![], 0),
        ("cli", vec![true, false], 2),
        ("project", vec![], 0),
        ("project", vec![true, false, false], 2),
        ("project", vec![true, false, true, false], 4),
    ] {
        let mut ui = Scripted {
            answers: answers.into(),
            ..Default::default()
        };
        let mut calls = Vec::new();
        flow(&options(scope), &mut ui, |args| {
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
fn both_flow_reuses_exact_paths_and_separate_review_digests_without_shell_interpolation() {
    let mut ui = Scripted {
        answers: vec![true, true, true, false, true, true].into(),
        ..Default::default()
    };
    let mut calls = Vec::new();
    flow(&options("both"), &mut ui, |args| {
        calls.push(args.to_vec());
        reply(args)
    })
    .unwrap();
    assert_eq!(calls.len(), 8);
    assert_eq!(calls[2].last().unwrap(), &OsString::from("a".repeat(64)));
    assert_eq!(calls[7].last().unwrap(), &OsString::from("b".repeat(64)));
    assert!(calls[3].contains(&OsString::from("project with 'quotes' $(literal)")));
    assert!(calls[4].contains(&OsString::from("--dry-run")));
    assert!(!calls[4].contains(&OsString::from("--allow-project-code")));
    assert!(calls[5].contains(&OsString::from("--allow-project-code")));
    assert!(!calls[5].contains(&OsString::from("--allow-network")));
    assert_eq!(ui.shown.last().unwrap()["completed_scope"], "both");
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
        flow(&options("project"), &mut ui, |args| {
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
    let mut config = options("project");
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
    assert!(flow(&options("cli"), &mut ui, |_| Ok(json!({"wrong":"result"}))).is_err());
    assert_eq!(ui.answers.len(), 1);
}

#[test]
fn project_major_jump_stops_before_installation_even_with_explicit_cli_major_opt_in() {
    let major = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .major
        + 1;
    let matches = command()
        .try_get_matches_from([
            "guided",
            "--to",
            &format!("{major}.0.0"),
            "--scope",
            "both",
            "--root",
            "/private/cli",
            "--allow-major",
        ])
        .unwrap();
    assert!(
        Options::parse(&matches)
            .unwrap_err()
            .to_string()
            .contains("major-specific")
    );
}

#[test]
fn offline_guidance_is_limited_to_project_verification() {
    let prerelease = (!semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .pre
        .is_empty())
    .then_some("--prerelease");
    let parse = |scope: &str| {
        let matches = command()
            .try_get_matches_from(
                [
                    "guided",
                    "--to",
                    env!("CARGO_PKG_VERSION"),
                    "--scope",
                    scope,
                    "--root",
                    "/private/cli",
                    "--offline",
                ]
                .into_iter()
                .chain(prerelease),
            )
            .unwrap();
        Options::parse(&matches)
    };
    assert_eq!(
        parse("cli").unwrap_err().to_string(),
        "guided update rejected: offline mode forbids authenticated CLI installation; use --scope project for offline project verification"
    );
    assert!(parse("project").unwrap().offline);
    if !std::io::stdin().is_terminal() {
        let matches = command()
            .try_get_matches_from(["guided", "--to", "99.0.0", "--scope", "project"])
            .unwrap();
        assert!(run(&matches).is_err());
    }
}

#[test]
fn guided_steps_dispatch_to_the_checked_commands() {
    let stage = execute(&["stage", "--to", "99.0.0", "--offline"].map(OsString::from));
    assert_eq!(
        stage.unwrap_err().to_string(),
        "artifact verification rejected: offline mode forbids authenticated downloads"
    );
    let project = execute(
        &[
            "project",
            "verify",
            "--prepared",
            "/rullst-missing-preparation",
        ]
        .map(OsString::from),
    );
    assert_eq!(
        project.unwrap_err().to_string(),
        "project preparation rejected: review the preparation first; --allow-project-code is required because Cargo builds and tests execute project code"
    );
}

/// Runs `update guided --scope project --offline` when a parent test names a
/// report file; otherwise does nothing.
#[test]
fn guided_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_GUIDED_REPORT") else {
        return;
    };
    let project = std::env::var_os("RULLST_GUIDED_PROJECT").unwrap();
    let prerelease = (!semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .unwrap()
        .pre
        .is_empty())
    .then_some(OsString::from("--prerelease"));
    let matches = command()
        .try_get_matches_from(
            [
                "guided".into(),
                "--to".into(),
                env!("CARGO_PKG_VERSION").into(),
                "--scope".into(),
                "project".into(),
                "--project".into(),
                project,
                "--offline".into(),
            ]
            .into_iter()
            .chain(prerelease),
        )
        .unwrap();
    let outcome = match run(&matches) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    std::fs::write(report, outcome).unwrap();
}

/// The real terminal: `script` gives the fixture a pseudo-terminal.
#[cfg(target_os = "linux")]
mod terminal {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    const PROMPT: &str = "Copy the selected Git project";

    /// The fixture's outcome and terminal output. `pipe_stdout` sends its
    /// standard output through `cat`; `key` answers the first prompt once it
    /// is shown. `None` when `script` is not installed.
    fn in_terminal(pipe_stdout: bool, key: Option<&str>) -> Option<(String, String)> {
        if Command::new("script").arg("--version").output().is_err() {
            return None;
        }
        let directory = tempfile::tempdir().unwrap();
        let report = directory.path().join("report");
        let mut inner = format!(
            "'{}' --exact update::guided::tests::guided_child_fixture --nocapture",
            std::env::current_exe().unwrap().display()
        );
        if pipe_stdout {
            inner.push_str(" | cat");
        }
        let mut child = Command::new("script")
            .args(["-qec", &inner, "/dev/null"])
            .env("RULLST_GUIDED_REPORT", &report)
            .env("RULLST_GUIDED_PROJECT", directory.path())
            .env("XDG_CACHE_HOME", directory.path().join("xdg"))
            .env("TERM", "xterm")
            .env_remove("CARGO_NET_OFFLINE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut terminal = child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while let Ok(count) = terminal.read(&mut buffer) {
                if count == 0 || sender.send(buffer[..count].to_vec()).is_err() {
                    break;
                }
            }
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        let mut output = Vec::new();
        let mut key = key;
        loop {
            match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(chunk) => output.extend(chunk),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if let Some(answer) = key
                && String::from_utf8_lossy(&output).contains(PROMPT)
            {
                // Let the prompt switch the terminal to raw input first.
                std::thread::sleep(Duration::from_millis(300));
                input.write_all(answer.as_bytes()).unwrap();
                input.flush().unwrap();
                key = None;
            }
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                panic!(
                    "the guided flow did not finish: {}",
                    String::from_utf8_lossy(&output)
                );
            }
        }
        drop(input);
        child.wait().unwrap();
        Some((
            std::fs::read_to_string(report).unwrap_or_default(),
            String::from_utf8_lossy(&output).into_owned(),
        ))
    }

    #[test]
    fn a_terminal_shows_the_plan_and_every_prompt_defaults_to_no() {
        let Some((outcome, shown)) = in_terminal(false, Some("n")) else {
            return;
        };
        assert_eq!(outcome, "ok", "{shown}");
        assert!(shown.contains("\"release_notes\""), "{shown}");
        assert!(shown.contains("\"stopped\": true"), "{shown}");

        let (outcome, shown) = in_terminal(false, Some("y")).unwrap();
        assert_ne!(outcome, "ok", "{shown}");
        assert!(!outcome.is_empty(), "{shown}");
        assert!(!shown.contains("\"stopped\""), "{shown}");
    }

    #[test]
    fn standard_output_must_be_a_terminal_too() {
        let Some((outcome, shown)) = in_terminal(true, None) else {
            return;
        };
        assert_eq!(
            outcome,
            "guided update rejected: guided updates require an interactive terminal; use explicit update commands with --json and reviewed approval digests for automation",
            "{shown}"
        );
    }
}
