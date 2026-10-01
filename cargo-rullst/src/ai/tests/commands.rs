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
    ] {
        assert!(matches!(
            validate_rullst(args(refused)),
            Err(CommandError::Argument(_))
        ));
    }
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
