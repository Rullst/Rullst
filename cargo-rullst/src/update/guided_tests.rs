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
    let matches = command()
        .try_get_matches_from([
            "guided",
            "--to",
            env!("CARGO_PKG_VERSION"),
            "--scope",
            scope,
            "--root",
            "/private/cli with spaces",
            "--project",
            "project with 'quotes' $(literal)",
        ])
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
