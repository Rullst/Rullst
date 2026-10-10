use super::*;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn debug_formatting_matches_the_secret_free_display() {
    let errors = [
        AiCliError::Usage("pass an error id".to_string()),
        AiCliError::Fix("the error context is larger than 64 KiB".to_string()),
    ];
    for error in errors {
        assert_eq!(format!("{error:?}"), error.to_string());
        assert!(!format!("{error:?}").is_empty());
    }
}

#[test]
fn a_plan_without_findings_names_the_next_step_by_pending_changes() {
    let apply = "No source findings need the assistant. Apply the dependency plan with `cargo rullst upgrade`.";
    let current =
        "No source findings need the assistant and the dependencies already target this release.";
    assert_eq!(no_findings_next_step(0), current);
    assert_eq!(no_findings_next_step(1), apply);
    assert_eq!(no_findings_next_step(7), apply);
}

/// Runs `cargo rullst ai <args>` on a helper thread; `None` means it did not
/// return within the bound (for example, it waited on standard input).
fn run_bounded(args: &'static [&'static str]) -> Option<Result<(), String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let matches = command().get_matches_from(args);
        let result = run(&matches).map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    receiver.recv_timeout(Duration::from_secs(20)).ok()
}

#[test]
fn ai_fix_dispatches_to_the_error_fix_flow_and_rejects_a_malformed_id() {
    // The test process runs inside the cargo-rullst package, so a project
    // root exists; the malformed id is refused before any network access.
    let result = run_bounded(&["ai", "fix", "not-an-error-id"])
        .expect("`ai fix` must return instead of starting an interactive session");
    let error = result.expect_err("a malformed error id is refused");
    assert!(error.contains("error id"), "{error}");
}

fn parse(args: &[&str]) -> ArgMatches {
    command().try_get_matches_from(args).unwrap()
}

#[test]
fn the_command_declares_every_ai_subcommand() {
    let ai = command();
    assert_eq!(ai.get_name(), "ai");
    let names: Vec<&str> = ai.get_subcommands().map(Command::get_name).collect();
    for name in [
        "connect",
        "upgrade",
        "fix",
        "review",
        "disconnect",
        "status",
    ] {
        assert!(names.contains(&name), "missing `{name}` in {names:?}");
    }
}

#[test]
fn each_subcommand_routes_to_its_own_handler_with_its_own_arguments() {
    let cases: [(&[&str], Route); 8] = [
        (&["ai", "connect"], Route::Connect),
        (&["ai", "disconnect"], Route::Disconnect),
        (&["ai", "status", "--json"], Route::Status),
        (&["ai", "upgrade", "--to", "13.0.0"], Route::Upgrade),
        (
            &["ai", "fix", "0123456789abcdef0123456789abcdef"],
            Route::Fix,
        ),
        (&["ai", "review"], Route::Review),
        (&["ai"], Route::Session),
        (&["ai", "explain", "the", "router"], Route::Session),
    ];
    for (args, expected) in cases {
        let matches = parse(args);
        assert_eq!(route(&matches).0, expected, "{args:?}");
    }
    let status = parse(&["ai", "status", "--json"]);
    assert!(route(&status).1.get_flag("json"));
    let upgrade = parse(&["ai", "upgrade", "--to", "13.0.0"]);
    assert_eq!(
        route(&upgrade)
            .1
            .get_one::<String>("to")
            .map(String::as_str),
        Some("13.0.0")
    );
    let session = parse(&["ai", "explain", "the", "router"]);
    assert_eq!(
        route(&session)
            .1
            .get_many::<String>("goal")
            .map(|words| words.count()),
        Some(3)
    );
}

#[test]
fn a_one_shot_plan_never_reads_standard_input() {
    let plan_only = Mode::PlanOnly("--dry-run");
    assert!(!reads_stdin(true, &plan_only));
    assert!(reads_stdin(true, &Mode::Execute));
    assert!(reads_stdin(false, &plan_only));
    assert!(reads_stdin(false, &Mode::Execute));
}

#[test]
fn the_project_root_is_the_canonical_package_directory() {
    // Unit tests run with the package directory as the working directory.
    let expected = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    assert_eq!(project_root(), Some(expected));
}

#[test]
fn an_invalid_upgrade_target_is_reported_instead_of_starting_a_session() {
    let result =
        run_bounded(&["ai", "upgrade", "--to", "not-a-version"]).expect("`ai upgrade` must return");
    let error = result.expect_err("an invalid target version is refused");
    assert!(error.starts_with("the upgrade plan failed"), "{error}");
}

#[test]
fn an_invalid_model_stops_a_session_before_it_starts() {
    let result = run_bounded(&["ai", "--model", "bad model\u{7}", "explain"])
        .expect("a session with an invalid model must return");
    assert!(result.is_err(), "an invalid model name is refused");
}
