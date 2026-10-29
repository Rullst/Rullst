//! Unit tests of the upgrade planner.

use super::*;

#[test]
fn target_must_match_the_installed_cli_major() {
    let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let incompatible = format!("{}.0.0", cli.major + 1);
    assert!(matches!(
        target_version(Some(&incompatible)),
        Err(UpgradeError::IncompatibleCli { .. })
    ));
}

#[test]
fn requirements_above_the_target_are_downgrades() {
    let target = Version::parse("13.0.0").unwrap();
    for newer in [
        "=13.2.0", "13.1", "^13.0.1", "~13.0.5", ">=14", ">13.0.0", "13.1.*",
    ] {
        assert!(requirement_exceeds(newer, &target), "{newer}");
    }
    for not_newer in [
        "=13.0.0",
        "13",
        "12",
        ">=12, <14",
        "<13.5",
        "*",
        "13.0.0-rc.1",
    ] {
        assert!(!requirement_exceeds(not_newer, &target), "{not_newer}");
    }
    // A stable requirement is above a prerelease CLI of the same version.
    let prerelease = Version::parse("13.0.0-alpha.1").unwrap();
    assert!(requirement_exceeds("13", &prerelease));
    assert!(!requirement_exceeds("13.0.0-alpha.1", &prerelease));
}

/// A removed provider is a review finding, so a failed gate used to give no
/// hint; it now names the finding and repeats its guidance.
#[test]
fn a_failed_gate_repeats_the_guidance_of_removed_api_findings() {
    let root = Path::new("/project");
    let finding = |rule: &'static rules::Rule, path: &str, line| rules::SourceFinding {
        rule,
        path: root.join(path),
        line,
    };
    let capital = rules::REMOVED_API_RULES
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-CAPITAL-REMOVED")
        .unwrap();
    let other = rules::REMOVED_API_RULES
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-MAIL-REMOVED")
        .unwrap();
    let scan = SourceScan {
        findings: vec![
            finding(capital, ".env.example", 21),
            finding(capital, "src/controllers/billing_controller.rs", 8),
            finding(other, "src/mail.rs", 3),
        ],
        ..SourceScan::default()
    };
    let hint = findings_hint(root, &scan);
    assert!(!hint.contains("must-change"), "{hint}");
    assert!(
        hint.contains("V13-CAPITAL-REMOVED at src/controllers/billing_controller.rs:8"),
        "{hint}"
    );
    assert!(!hint.contains(".env.example"), "{hint}");
    assert!(hint.contains("`StripeProvider`"), "{hint}");
    assert!(
        hint.contains("`InfinitePayProvider` is experimental"),
        "{hint}"
    );
    assert!(hint.contains("V13-MAIL-REMOVED at src/mail.rs:3"), "{hint}");
    assert!(findings_hint(root, &SourceScan::default()).is_empty());
}

#[test]
fn target_accepts_an_exact_prerelease_in_the_cli_train() {
    let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let prerelease = format!("{}.1.0-rc.2", cli.major);
    assert_eq!(
        target_version(Some(&prerelease)).unwrap().to_string(),
        prerelease
    );
}

fn selected_scan(current: Option<&str>, all_rules: bool) -> SourceScan {
    let finding = |rule: &'static rules::Rule, line| rules::SourceFinding {
        rule,
        path: Path::new("/project/src/main.rs").to_path_buf(),
        line,
    };
    let page = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-RENDER-PAGE-LANGUAGE")
        .unwrap();
    let transaction = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-ORM-TRANSACTION-ERRORS")
        .unwrap();
    let mut scan = SourceScan {
        findings: vec![finding(page, 2), finding(transaction, 5)],
        ..SourceScan::default()
    };
    scan.select(
        current.map(|version| Version::parse(version).unwrap()),
        all_rules,
    );
    scan
}

/// An alpha.2 application's plan names its version, the skipped rules and
/// the flag that shows them; every other plan says nothing about it.
#[test]
fn the_plan_says_which_earlier_rules_it_skipped() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0-alpha.3").unwrap();
    let scan = selected_scan(Some("13.0.0-alpha.2"), false);
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(
        text.contains("  Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 1); --all-rules applies every rule\n"),
        "{text}"
    );
    assert!(text.contains("[V13-ORM-TRANSACTION-ERRORS]"), "{text}");
    assert!(!text.contains("[V13-RENDER-PAGE-LANGUAGE]"), "{text}");
    let markdown = report::render_report(root, &target, &[], &scan);
    assert!(
        markdown.contains("Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 1); --all-rules applies every rule.\n\n"),
        "{markdown}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &target, &[], &scan).unwrap())
            .unwrap();
    assert_eq!(
        json["rule_selection"],
        serde_json::json!({
            "current_version": "13.0.0-alpha.2",
            "all_rules": false,
            "skipped_rules": 100,
            "hidden_findings": 1,
        })
    );
    assert_eq!(
        json["source_findings"][0]["introduced_in"],
        "13.0.0-alpha.3"
    );

    for (current, all_rules) in [
        (Some("12.3.0"), false),
        (None, false),
        (Some("13.0.0-alpha.2"), true),
    ] {
        let scan = selected_scan(current, all_rules);
        let text = report::plan_text(root, &target, &[], &scan);
        assert!(!text.contains("Rules:"), "{text}");
        assert!(text.contains("[V13-RENDER-PAGE-LANGUAGE]"), "{text}");
        let markdown = report::render_report(root, &target, &[], &scan);
        assert!(!markdown.contains("Rules:"), "{markdown}");
    }
}

/// Must-change and review findings as the assistant receives them, and the
/// guide links that only a plan with findings carries.
#[test]
fn findings_reach_the_plan_text_and_the_assistant() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0").unwrap();
    let clean = report::plan_text(root, &target, &[], &SourceScan::default());
    assert!(!clean.contains("Migration guide:"), "{clean}");
    let scan = selected_scan(None, false);
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(text.contains("Migration guide: "), "{text}");
    assert!(
        text.contains("Reviewed fixes for these findings: cargo rullst ai upgrade"),
        "{text}"
    );

    let handler = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-HTML-DYNAMIC-EVENT-HANDLER")
        .unwrap();
    let mut mixed = selected_scan(None, false);
    mixed.findings.push(rules::SourceFinding {
        rule: handler,
        path: root.join("src/view.rs"),
        line: 7,
    });
    let findings = report::assist_findings(root, &mixed);
    let summary: Vec<_> = findings
        .iter()
        .map(|finding| {
            (
                finding.code,
                finding.must_change,
                finding.path.as_str(),
                finding.line,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("V13-RENDER-PAGE-LANGUAGE", false, "src/main.rs", 2),
            ("V13-ORM-TRANSACTION-ERRORS", false, "src/main.rs", 5),
            ("V13-HTML-DYNAMIC-EVENT-HANDLER", true, "src/view.rs", 7),
        ]
    );
}

/// Runs an upgrade in the current directory when a parent test names a
/// report file; otherwise does nothing. `RULLST_UPGRADE_MODE` selects `json`
/// (a JSON dry run), `apply`, or `keep` (apply, keeping edited files when a
/// Cargo gate fails); any other value runs a plain dry run.
#[test]
fn upgrade_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_UPGRADE_REPORT") else {
        return;
    };
    let mode = std::env::var("RULLST_UPGRADE_MODE").unwrap_or_default();
    let outcome = match run_upgrade(UpgradeOptions {
        target: Some("13.0.0".into()),
        dry_run: !matches!(mode.as_str(), "apply" | "keep"),
        json: mode == "json",
        keep_on_failure: mode == "keep",
        restore: None,
        all_rules: false,
    }) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    std::fs::write(report, outcome).unwrap();
}

/// A package that depends on `rullst = "<requirement>"`.
fn demo_project(requirement: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(project.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        project.path().join("Cargo.toml"),
        demo_manifest(requirement),
    )
    .unwrap();
    project
}

fn demo_manifest(requirement: &str) -> String {
    format!(
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = {{ version = \"{requirement}\", default-features = false }}\n"
    )
}

/// Runs `upgrade_child_fixture` in `project` in `mode` with extra
/// environment; returns the outcome it reported and its standard output.
fn run_child(
    project: &Path,
    mode: &str,
    environment: &[(&str, &std::ffi::OsStr)],
) -> (String, String) {
    let report = project.join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::build::upgrade::tests::upgrade_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        .env("NO_COLOR", "1")
        .env("RULLST_UPGRADE_REPORT", &report)
        .env("RULLST_UPGRADE_MODE", mode)
        .stdin(std::process::Stdio::null());
    for (name, value) in environment {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success(), "{stdout}");
    (std::fs::read_to_string(&report).unwrap(), stdout)
}

#[test]
fn a_dry_run_prints_the_plan() {
    let project = demo_project("12.3");
    let (outcome, stdout) = run_child(project.path(), "dry-run", &[]);
    assert_eq!(outcome, "ok", "{stdout}");
    assert!(
        stdout.contains("Rullst assisted upgrade — DRY RUN — target 13.0.0"),
        "{stdout}"
    );
    assert!(stdout.contains("rullst (rullst) 12.3 -> "), "{stdout}");
    assert!(
        stdout.contains("Dry run complete: no files or lockfile were changed."),
        "{stdout}"
    );
}

#[test]
fn a_json_dry_run_prints_only_the_report() {
    let project = demo_project("12.3");
    let (outcome, stdout) = run_child(project.path(), "json", &[]);
    assert_eq!(outcome, "ok", "{stdout}");
    assert!(stdout.contains("\"target\": \"13.0.0\""), "{stdout}");
    assert!(!stdout.contains("DRY RUN"), "{stdout}");
    assert!(!stdout.contains("Dry run complete"), "{stdout}");
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("12.3")
    );
}

/// A stand-in `cargo` that answers `metadata` for `project` and exits with
/// `RULLST_STUB_FIX` and `RULLST_STUB_CHECK` for `fix` and `check`; returns
/// a `PATH` that finds it first. Unix-only: the stand-in is a shell script.
#[cfg(unix)]
fn stub_cargo(project: &Path, directory: &Path) -> std::ffi::OsString {
    use std::os::unix::fs::PermissionsExt;
    let metadata = serde_json::json!({
        "packages": [{
            "id": "demo",
            "manifest_path": project.canonicalize().unwrap().join("Cargo.toml"),
        }],
        "workspace_members": ["demo"],
    });
    let answer = directory.join("metadata.json");
    std::fs::write(&answer, metadata.to_string()).unwrap();
    let script = directory.join("cargo");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncase \"$1\" in\n  metadata) cat '{}' ;;\n  fix) exit \"$RULLST_STUB_FIX\" ;;\n  check) exit \"$RULLST_STUB_CHECK\" ;;\n  *) exit 97 ;;\nesac\n",
            answer.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Another test thread forking while the script was open for writing
    // makes executing it fail briefly with ETXTBSY; wait until it runs.
    for _ in 0..40 {
        if std::process::Command::new(&script)
            .arg("probe")
            .status()
            .is_ok()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let mut path = directory.as_os_str().to_os_string();
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap_or_default());
    path
}

/// Applies an upgrade of `rullst = "<requirement>"` with the stand-in
/// `cargo` exiting `fix` and `check` as given.
#[cfg(unix)]
fn apply(
    requirement: &str,
    mode: &str,
    fix: &str,
    check: &str,
) -> (tempfile::TempDir, String, String) {
    let project = demo_project(requirement);
    let tools = tempfile::tempdir().unwrap();
    let path = stub_cargo(project.path(), tools.path());
    let (outcome, stdout) = run_child(
        project.path(),
        mode,
        &[
            ("PATH", path.as_os_str()),
            ("RULLST_STUB_FIX", fix.as_ref()),
            ("RULLST_STUB_CHECK", check.as_ref()),
        ],
    );
    (project, outcome, stdout)
}

#[cfg(unix)]
#[test]
fn a_passing_upgrade_edits_the_manifest_and_names_the_report() {
    let (project, outcome, stdout) = apply("12.3", "apply", "0", "0");
    assert_eq!(outcome, "ok", "{stdout}");
    assert!(
        stdout.contains(
            "Upgrade transaction completed and cargo check passed.\nBackup and review report: "
        ),
        "{stdout}"
    );
    assert!(
        !stdout.contains("No version edits are required"),
        "{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("=13.0.0")
    );

    let (_, outcome, stdout) = apply("=13.0.0", "apply", "0", "0");
    assert_eq!(outcome, "ok", "{stdout}");
    assert!(
        stdout.contains("No version edits are required."),
        "{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn a_failed_cargo_fix_restores_the_project() {
    let (project, outcome, stdout) = apply("12.3", "apply", "1", "0");
    assert!(
        outcome.starts_with("cargo fix --workspace --all-targets failed; the original manifests, lockfile, and Rust sources were restored; diagnostic report: "),
        "{outcome}\n{stdout}"
    );
    assert!(
        outcome.contains(
            "; the persisted snapshot can be restored again with `cargo rullst upgrade --restore "
        ),
        "{outcome}"
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("12.3")
    );
}

#[cfg(unix)]
#[test]
fn a_failed_cargo_check_can_keep_the_edited_project() {
    let (project, outcome, stdout) = apply("12.3", "keep", "0", "1");
    assert!(
        outcome.starts_with("cargo check --workspace --all-targets --locked failed; edited files were kept by request; recover with `cargo rullst upgrade --restore "),
        "{outcome}\n{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("=13.0.0")
    );
}

/// The planner accepts v13 sources and refuses a requirement above the target.
#[test]
fn the_plan_accepts_v13_sources_and_refuses_downgrades() {
    let plan = |requirement: &str| {
        let project = demo_project(requirement);
        let root = project.path().canonicalize().unwrap();
        plan_project(&root, Some("13.0.0"), false).map(|planned| planned.changed)
    };
    assert_eq!(plan("13.0").unwrap(), 1);
    assert_eq!(
        plan("=13.1.0").unwrap_err().to_string(),
        "upgrading to 13.0.0 would downgrade `rullst` (requirement `=13.1.0`); install a cargo-rullst release that is not older than the project, or pass `--to` with such a version"
    );
    assert!(
        plan("11.2")
            .unwrap_err()
            .to_string()
            .starts_with("this project depends on Rullst v11; "),
    );
}

/// Only removed-API findings in Rust sources are named after a failed gate.
#[test]
fn a_failed_gate_names_no_other_review_finding() {
    let page = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-RENDER-PAGE-LANGUAGE")
        .unwrap();
    let scan = SourceScan {
        findings: vec![rules::SourceFinding {
            rule: page,
            path: Path::new("/project/src/main.rs").to_path_buf(),
            line: 2,
        }],
        ..SourceScan::default()
    };
    assert_eq!(findings_hint(Path::new("/project"), &scan), "");
}
