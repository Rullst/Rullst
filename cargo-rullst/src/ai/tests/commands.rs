use super::*;
use crate::ai::process::{Capture, run};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn allowlisted_scaffolds_and_diagnostics_validate() {
    for command in [
        &["make:model", "Post", "--migration"][..],
        &["make:controller", "posts", "--api"],
        &[
            "make:omni",
            "--platform=desktop,android",
            "--backend-url",
            "https://api.example.com",
        ],
        &["generate:ai-context", "--check"],
        &[
            "generate:api",
            "--schema",
            "api/openapi.json",
            "--output",
            "src/api",
        ],
        &["db:status"],
        &["doctor"],
        &["audit", "--idor"],
        &["inspect", "routes"],
    ] {
        let invocation = validate_rullst(args(command)).unwrap();
        assert_eq!(
            invocation.display(),
            format!("cargo rullst {}", command.join(" "))
        );
    }
    // Migrations are allowed here; the caller also requires a development
    // project and an individual confirmation.
    let migrate = validate_rullst(args(&["db:migrate"])).unwrap();
    assert!(migrate.always_confirm() && !migrate.mutates());
    assert!(
        !validate_rullst(args(&["make:model", "Post"]))
            .unwrap()
            .always_confirm()
    );
    assert!(
        validate_rullst(args(&["make:model", "Post"]))
            .unwrap()
            .mutates()
    );
    assert!(!validate_rullst(args(&["doctor"])).unwrap().mutates());
    assert!(
        !validate_rullst(args(&["audit", "--idor"]))
            .unwrap()
            .mutates()
    );
    assert!(
        validate_rullst(args(&["audit", "--sbom"]))
            .unwrap()
            .mutates()
    );
    // `--report` writes SECURITY_REPORT.<format> like `--compliance`.
    for report in [
        &["audit", "--report", "json"][..],
        &["audit", "--report=md"],
    ] {
        assert!(validate_rullst(args(report)).unwrap().mutates());
    }
}

#[test]
fn every_allowlisted_command_exists_in_the_cli() {
    let cli = <crate::cli::Cli as clap::CommandFactory>::command();
    let mut names: Vec<String> = cli
        .get_subcommands()
        .map(|command| command.get_name().to_string())
        .collect();
    for extra in [
        crate::generators::privacy::command(),
        crate::generators::age_gate::command(),
        crate::generators::api_contract::command(),
    ] {
        names.push(extra.get_name().to_string());
    }
    for allowed in RULLST_ALLOWLIST {
        assert!(
            names.iter().any(|name| name == allowed),
            "{allowed} is not a CLI command"
        );
    }
}

#[test]
fn dangerous_or_unknown_commands_are_refused() {
    for command in [
        "deploy",
        "foundry:deploy",
        "foundry:init",
        "upgrade",
        "update",
        "pkg",
        "new",
        "dev",
        "eject",
        "hook:install",
        "db:rollback",
        "db:seed",
        "generate:models",
        "make:models-from-db",
        "ai",
        "rm",
        "",
    ] {
        assert!(
            matches!(
                validate_rullst(args(&[command])),
                Err(CommandError::NotAllowed(_))
            ),
            "{command}"
        );
    }
    assert_eq!(validate_rullst(Vec::new()), Err(CommandError::Arity));
    assert_eq!(
        validate_rullst(args(&[
            "make:model",
            "a",
            "b",
            "c",
            "d",
            "e",
            "f",
            "g",
            "h",
            "i"
        ])),
        Err(CommandError::Arity)
    );
}

#[test]
fn hostile_arguments_are_refused() {
    for argument in [
        "Post; rm -rf /",
        "$(whoami)",
        "`id`",
        "a|b",
        "a&b",
        "a>b",
        "../secrets",
        "src/../../x",
        "/etc/passwd",
        "~/.ssh/id_rsa",
        "two words",
        "quote\"d",
        "new\nline",
        "-",
        "--",
        "---x",
        "-1",
        "--=x",
        "é",
        // Values are checked however they are spelled.
        "--output=/home/user/.config",
        "--schema=~/api.json",
        "C:/Users/me/.ssh/id_rsa",
        "c:secrets",
        "--output=C:/Windows",
        "--output=d:",
    ] {
        assert!(
            matches!(
                validate_rullst(args(&["make:model", argument])),
                Err(CommandError::Argument(_))
            ),
            "{argument:?}"
        );
    }
    assert!(validate_rullst(args(&["make:model", &"x".repeat(129)])).is_err());
    for refused in [
        &["doctor", "--fix"][..],
        &["audit", "--network"],
        &["audit", "--network=yes"],
        &["audit", "--report", "json", "--output", "report.json"],
        &["audit", "--report", "--output=report.md"],
        // `inspect` prints any other target as a file.
        &["inspect", ".env"],
        &["inspect", "Cargo.toml"],
        &["inspect", "routes", "models"],
    ] {
        assert!(matches!(
            validate_rullst(args(refused)),
            Err(CommandError::Argument(_))
        ));
    }
}

#[test]
fn ordinary_values_and_urls_still_validate() {
    for command in [
        &["make:omni", "--platform=desktop,android"][..],
        &["make:omni", "--backend-url=https://api.example.com"],
        &["make:mail", "Welcome", "--welcome"],
        &[
            "generate:api",
            "--schema=api/openapi.json",
            "--output=src/api",
        ],
    ] {
        assert!(validate_rullst(args(command)).is_ok(), "{command:?}");
    }
    assert_eq!(
        path_values(&args(&[
            "generate:api",
            "--schema",
            "a.json",
            "--output=src/api",
            "--check"
        ])),
        vec![(PathKind::File, "a.json"), (PathKind::Directory, "src/api")]
    );
}

#[test]
fn cargo_is_limited_to_check_and_test_with_known_flags() {
    for command in [
        &["check"][..],
        &["check", "--all-targets"],
        &["test"],
        &["test", "--lib", "models::tests"],
        &["test", "--test", "api_cli", "--no-fail-fast"],
    ] {
        let invocation = validate_cargo(args(command)).unwrap();
        assert_eq!(invocation.display(), format!("cargo {}", command.join(" ")));
        assert!(!invocation.mutates());
    }
    for command in [
        &["build"][..],
        &["run"],
        &["install", "evil"],
        &["check", "--manifest-path", "../other/Cargo.toml"],
        &["check", "--target-dir", "/tmp"],
        &["check", "--config", "build.rustc-wrapper=sh"],
        &["check", "-Zunstable-options"],
        &["test", "a", "b"],
        &["test", "--", "--nocapture"],
        &["test", "--test"],
        &["test", "--test", "../x"],
        &["check", "filter"],
    ] {
        assert!(validate_cargo(args(command)).is_err(), "{command:?}");
    }
}

#[test]
fn capture_keeps_a_bounded_head_and_tail() {
    let mut capture = Capture::default();
    capture.push(b"start\n");
    capture.push(&vec![b'x'; 64 * 1024]);
    capture.push(b"\nend\n");
    let text = capture.text();
    assert!(text.starts_with("start\n"));
    assert!(text.ends_with("\nend\n"));
    assert!(text.contains("bytes omitted"));
    assert!(text.len() < 20 * 1024);
}

#[test]
fn run_captures_output_and_status_without_a_shell() {
    let directory = tempfile::tempdir().unwrap();
    // Constructed directly: `--version` is not part of the allowlist, but it
    // exercises the runner with the same cargo binary the tests use.
    let invocation = Invocation {
        kind: CommandKind::Cargo,
        args: args(&["--version"]),
    };
    let mut lines = Vec::new();
    let outcome = run(&invocation, directory.path(), |line| {
        lines.push(line.to_string())
    })
    .unwrap();
    assert!(outcome.success, "{}", outcome.output);
    assert_eq!(outcome.status, "exit status 0");
    assert!(outcome.output.starts_with("cargo "));
    assert!(lines.iter().any(|line| line.starts_with("cargo ")));

    let failing = Invocation {
        kind: CommandKind::Cargo,
        args: args(&["check"]),
    };
    // No manifest in an empty directory: cargo fails and the runner reports it.
    let outcome = run(&failing, directory.path(), |_| {}).unwrap();
    assert!(!outcome.success);
    assert!(outcome.output.contains("Cargo.toml"));
}

/// Runs `script` with `/bin/sh -c` through the runner and returns the
/// outcome, the elapsed time and the PID the script printed first.
#[cfg(unix)]
fn run_shell(
    script: &str,
    deadline: Option<std::time::Duration>,
) -> (crate::ai::process::Outcome, std::time::Duration, i32) {
    let directory = tempfile::tempdir().unwrap();
    TEST_RULLST_PROGRAM.with(|program| *program.borrow_mut() = Some("/bin/sh".into()));
    TEST_DEADLINE.with(|cell| cell.set(deadline));
    let invocation = Invocation {
        kind: CommandKind::Rullst,
        args: args(&["-c", script]),
    };
    let started = std::time::Instant::now();
    let outcome = run(&invocation, directory.path(), |_| {});
    TEST_RULLST_PROGRAM.with(|program| *program.borrow_mut() = None);
    TEST_DEADLINE.with(|cell| cell.set(None));
    let outcome = outcome.unwrap();
    let pid = outcome
        .output
        .lines()
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    (outcome, started.elapsed(), pid)
}

/// Whether `pid` ends within a few seconds (it may be a zombie briefly).
#[cfg(unix)]
fn ends(pid: i32) -> bool {
    let pid = rustix::process::Pid::from_raw(pid).unwrap();
    (0..100).any(|_| {
        let gone = rustix::process::test_kill_process(pid).is_err();
        if !gone {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        gone
    })
}

#[cfg(unix)]
#[test]
fn a_command_ends_when_it_exits_even_if_a_child_keeps_its_output() {
    let (outcome, elapsed, pid) = run_shell("sleep 60 & echo $!; echo done", None);
    assert!(elapsed < std::time::Duration::from_secs(20), "{elapsed:?}");
    assert!(outcome.success, "{}", outcome.status);
    assert_eq!(
        outcome.status,
        "exit status 0 (processes it left running were stopped)"
    );
    assert!(outcome.output.contains("done"));
    assert!(ends(pid), "the leftover process is still running");
}

#[cfg(unix)]
#[test]
fn a_timeout_stops_the_whole_process_group() {
    let (outcome, elapsed, pid) = run_shell(
        "sleep 60 & echo $!; sleep 60",
        Some(std::time::Duration::from_secs(1)),
    );
    assert!(elapsed < std::time::Duration::from_secs(20), "{elapsed:?}");
    assert!(!outcome.success);
    assert_eq!(outcome.status, "stopped after the time limit");
    assert!(
        ends(pid),
        "a process the command started survived the timeout"
    );
}

/// Run by [`ctrl_c_reaches_the_command_group`] in a child process, where a
/// SIGINT cannot disturb other tests.
#[cfg(unix)]
#[test]
#[ignore = "run in a child process by ctrl_c_reaches_the_command_group"]
fn ctrl_c_child() {
    if std::env::var_os("RULLST_TEST_CTRL_C_CHILD").is_none() {
        return;
    }
    // The handler is installed first, so the signal cannot end this process.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let _handler = runtime.block_on(async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap()
    });
    let sender = std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        rustix::process::kill_process(rustix::process::getpid(), rustix::process::Signal::INT)
            .unwrap();
    });
    let (outcome, elapsed, _) = run_shell(
        "echo $$; sleep 60",
        Some(std::time::Duration::from_secs(30)),
    );
    sender.join().unwrap();
    assert!(elapsed < std::time::Duration::from_secs(20), "{elapsed:?}");
    assert!(!outcome.success);
    assert!(
        outcome.status.starts_with("terminated by a signal"),
        "{}",
        outcome.status
    );
}

#[cfg(unix)]
#[test]
fn ctrl_c_reaches_the_command_group() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ai::commands::tests::ctrl_c_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("RULLST_TEST_CTRL_C_CHILD", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("1 passed"), "{stdout}");
}
