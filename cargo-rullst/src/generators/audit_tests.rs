#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[test]
fn advisory_exception_ids_are_strictly_validated() {
    assert!(validate_audit_ignores(&["RUSTSEC-2099-0001".to_string()]).is_ok());
    for invalid in [
        "rustsec-2099-0001",
        "RUSTSEC-99-0001",
        "RUSTSEC-2099-001",
        "RUSTSEC-2099-0001 --quiet",
    ] {
        assert!(validate_audit_ignores(&[invalid.to_string()]).is_err());
    }
}

#[test]
fn advisory_exceptions_are_forwarded_as_distinct_cargo_audit_arguments() {
    assert_eq!(
        cargo_audit_arguments(&[
            "RUSTSEC-2099-0001".to_string(),
            "RUSTSEC-2099-0002".to_string(),
        ]),
        [
            "audit",
            "--ignore",
            "RUSTSEC-2099-0001",
            "--ignore",
            "RUSTSEC-2099-0002",
        ]
    );
}

#[cfg(unix)]
#[test]
fn source_scans_do_not_follow_a_symlinked_directory_loop() {
    let project = tempfile::tempdir().expect("temporary project");
    let src = project.path().join("src");
    fs::create_dir_all(&src).expect("source directory");
    fs::write(
        src.join("lib.rs"),
        "pub unsafe fn unchecked() {}\nfn routes() { get(\"/users/:id\" => show); }\n",
    )
    .expect("source fixture");
    // The old walk followed this link until the kernel's symlink bound and
    // reported one copy of every finding per level.
    std::os::unix::fs::symlink(".", src.join("loop")).expect("loop link");

    let (unsafe_count, _) = scan_unsafe_code(&src);
    assert_eq!(unsafe_count, 1);
    let (idor_count, warnings) = scan_idor_vulnerabilities(&src);
    assert_eq!(idor_count, 1, "{warnings:?}");
}

/// Runs `run_audit` in a child test process inside a fixture project, with
/// no external programs on `PATH`.
/// Miri cannot start processes, so the tests that run it are ignored there.
#[test]
fn audit_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_AUDIT_REPORT") else {
        return;
    };
    let full = std::env::var_os("RULLST_AUDIT_FULL").is_some();
    let modes = std::env::var("RULLST_AUDIT_MODES").unwrap_or_default();
    let has = |mode: &str| modes.split(',').any(|word| word == mode);
    let ignores = if has("ignore") {
        vec!["RUSTSEC-2020-0001".to_string()]
    } else {
        Vec::new()
    };
    let result = if has("wrapper") {
        run_security_audit(false, false, true, false, false, false)
    } else if has("wrapper-exceptions") {
        run_security_audit_with_exceptions(false, false, true, false, false, &[], false)
    } else {
        run_audit(AuditOptions {
            ai: has("ai"),
            idor: !has("no-idor"),
            geiger: has("geiger"),
            sbom: full || has("sbom"),
            network: full || has("network"),
            ignores: &ignores,
            json: true,
            ..AuditOptions::default()
        })
    };
    let text = match result {
        Ok(()) => "ok".to_string(),
        Err(error) => format!("err {error}"),
    };
    fs::write(report, text).unwrap();
}

fn audit_in(project: &Path, full: bool) -> (String, serde_json::Value) {
    let programs = project.join(".no-programs");
    fs::create_dir_all(&programs).unwrap();
    let report = project.join(".audit-result");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "generators::audit::tests::audit_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        .env("PATH", &programs)
        .env("NO_COLOR", "1")
        .env("RULLST_AUDIT_REPORT", &report)
        .env_remove("RULLST_AUDIT_FULL")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if full {
        command.env("RULLST_AUDIT_FULL", "1");
    }
    let output = command.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let start = stdout.find("\n{\n").expect("JSON summary") + 1;
    let end = start + stdout[start..].find("\n}\n").expect("end of summary") + 2;
    (
        fs::read_to_string(report).unwrap(),
        serde_json::from_str(&stdout[start..end]).unwrap(),
    )
}

fn checks(summary: &serde_json::Value) -> Vec<String> {
    summary["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| format!("{} {} {}", check["id"], check["status"], check["count"]))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_full_audit_counts_every_finding_once() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(
        root.join(".env"),
        "# COMMENTED_KEY=x\nSECRET_TOKEN=short\nAPI_KEY=\nPASSWORD=\"\"\nDB_PASSWORD=''\nLONG_SECRET=0123456789abcdef\nPLAIN=short\nOTHER_KEY=abc\nHOST=0.0.0.0\n",
    )
    .unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        "fn main() { let _ = TcpListener::bind(\"0.0.0.0:5555\"); }\npub unsafe fn unchecked() {}\nfn routes() { get(\"/users/{id}\" => show); }\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"demo\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let (result, summary) = audit_in(root, true);
    assert_eq!(
        result,
        "err IDOR/BOLA audit found 1 unclassified or unguarded parameterized route(s)"
    );
    assert_eq!(summary["issues_found"], 7);
    assert_eq!(
        checks(&summary),
        [
            r#""secret_scan" "findings" 2"#,
            r#""dependency_audit" "not_checked" null"#,
            r#""unsafe_scan" "findings" 1"#,
            r#""idor_scan" "findings" 1"#,
            r#""sbom" "generated" 1"#,
            r#""network_scan" "error" null"#,
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_clean_project_passes_the_audit() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(
        root.join(".env"),
        "APP_KEY=0123456789abcdef0123
",
    )
    .unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    let (result, summary) = audit_in(root, false);
    assert_eq!(result, "ok");
    assert_eq!(summary["issues_found"], 0);
    assert_eq!(
        checks(&summary),
        [
            r#""secret_scan" "no_findings" null"#,
            r#""dependency_audit" "not_checked" null"#,
            r#""unsafe_scan" "no_findings" null"#,
            r#""idor_scan" "no_findings" null"#,
            r#""sbom" "not_checked" null"#,
            r#""network_scan" "not_checked" null"#,
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn unreadable_secrets_and_missing_sources_are_incomplete_checks() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::create_dir(root.join(".env")).unwrap();
    let (result, summary) = audit_in(root, false);
    assert_eq!(
        result,
        "err IDOR/BOLA audit could not find any scannable Rust source"
    );
    assert_eq!(summary["issues_found"], 2);
    let checks = checks(&summary);
    assert_eq!(checks[0], r#""secret_scan" "error" null"#);
    assert_eq!(checks[2], r#""unsafe_scan" "not_checked" null"#);
    assert_eq!(checks[3], r#""idor_scan" "not_checked" null"#);
}

#[test]
fn an_incomplete_walk_names_its_root_reason_and_subject() {
    assert_eq!(
        incomplete_walk_warning(Path::new("src"), "too many files", "files"),
        "Source walk under 'src' is incomplete (too many files); files beyond it were not scanned"
    );
}

/// The outcome and output of the fixture with `modes` and `variables`.
struct Audit {
    result: String,
    stdout: String,
    stderr: String,
}

impl Audit {
    fn summary(&self) -> serde_json::Value {
        let start = self.stdout.find("\n{\n").expect("JSON summary") + 1;
        let end = start + self.stdout[start..].find("\n}\n").expect("end of summary") + 2;
        serde_json::from_str(&self.stdout[start..end]).unwrap()
    }
}

fn audit_with(project: &Path, modes: &str, variables: &[(&str, &str)]) -> Audit {
    let programs = project.join(".no-programs");
    fs::create_dir_all(&programs).unwrap();
    let report = project.join(".audit-result");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "generators::audit::tests::audit_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        .env("PATH", &programs)
        .env("NO_COLOR", "1")
        .env("RULLST_AUDIT_REPORT", &report)
        .env("RULLST_AUDIT_MODES", modes)
        .env_remove("RULLST_AUDIT_FULL")
        .envs(variables.iter().copied())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    Audit {
        result: fs::read_to_string(report).unwrap(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// A project with a strong key, clean source and the given extra source.
fn project_with(source: &str) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::write(root.join(".env"), "APP_KEY=0123456789abcdef0123\n").unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        format!("fn main() {{}}\n{source}"),
    )
    .unwrap();
    project
}

/// A fake program on the fixture's `PATH`, run once so that no other
/// test's fork still holds it open for writing.
#[cfg(unix)]
fn program(project: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let programs = project.join(".no-programs");
    fs::create_dir_all(&programs).unwrap();
    let path = programs.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..100 {
        match std::process::Command::new(&path).arg("--warm-up").output() {
            Err(error) if error.raw_os_error() == Some(26) => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            _ => return,
        }
    }
    panic!("{name} stayed busy");
}

/// `cargo audit` and `cargo geiger` answer with the exit statuses in
/// `FAKE_AUDIT`, `FAKE_GEIGER` and `FAKE_GEIGER_VERSION`; `FAKE_VANISH`
/// removes the program after the geiger version check.
#[cfg(unix)]
const CARGO: &str = r#"case "$1 $2" in
  "audit --version") exit 0 ;;
  "geiger --version") [ -n "$FAKE_VANISH" ] && /bin/rm -f "$0"; exit "${FAKE_GEIGER_VERSION:-0}" ;;
esac
case "$1" in
  audit) exit "${FAKE_AUDIT:-0}" ;;
  geiger) exit "${FAKE_GEIGER:-0}" ;;
esac
exit 1"#;

const ONE_FINDING: &str =
    "err security audit reported 1 finding(s) or incomplete requested check(s)";

#[cfg(unix)]
#[test]
#[cfg_attr(miri, ignore)]
fn cargo_audit_outcomes_are_reported_and_counted() {
    for (status, modes, expected, outcome) in [
        ("0", "", "no_findings", "ok"),
        ("0", "ignore", "no_findings_outside_exceptions", "ok"),
        ("1", "", "error", ONE_FINDING),
    ] {
        let project = project_with("");
        program(project.path(), "cargo", CARGO);
        let audit = audit_with(project.path(), modes, &[("FAKE_AUDIT", status)]);
        assert_eq!(audit.result, outcome, "{modes} {status}");
        assert_eq!(audit.summary()["checks"][1]["status"], expected);
    }
}

#[cfg(unix)]
#[test]
#[cfg_attr(miri, ignore)]
fn requested_geiger_runs_count_failures_and_absence() {
    for (variable, value, issues) in [
        ("FAKE_GEIGER", "0", 0),
        ("FAKE_GEIGER", "3", 1),
        ("FAKE_GEIGER_VERSION", "1", 1),
        ("FAKE_VANISH", "1", 1),
    ] {
        let project = project_with("");
        program(project.path(), "cargo", CARGO);
        let audit = audit_with(project.path(), "geiger", &[(variable, value)]);
        assert_eq!(audit.summary()["issues_found"], issues, "{variable}");
        let outcome = if issues == 0 { "ok" } else { ONE_FINDING };
        assert_eq!(audit.result, outcome, "{variable}");
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn without_idor_mode_routes_still_count_and_absent_source_does_not() {
    let project = project_with("fn routes() { get(\"/users/{id}\" => show); }\n");
    let audit = audit_with(project.path(), "no-idor", &[]);
    assert_eq!(audit.result, ONE_FINDING);
    assert_eq!(audit.summary()["issues_found"], 1);

    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join(".env"),
        "APP_KEY=0123456789abcdef0123\n",
    )
    .unwrap();
    let audit = audit_with(project.path(), "no-idor", &[]);
    assert_eq!(audit.result, "ok");
    assert_eq!(audit.summary()["issues_found"], 0);
}

#[test]
#[cfg_attr(miri, ignore)]
fn recommendations_and_failures_follow_the_issue_count() {
    let clean = project_with("");
    let audit = audit_with(clean.path(), "ai", &[]);
    assert_eq!(audit.result, "ok");
    assert!(
        audit
            .stderr
            .contains("Completed bounded checks reported no findings"),
        "{}",
        audit.stderr
    );

    let weak = project_with("");
    fs::write(weak.path().join(".env"), "SECRET_TOKEN=short\n").unwrap();
    let audit = audit_with(weak.path(), "ai", &[]);
    assert_eq!(audit.result, ONE_FINDING);
    assert!(
        audit.stderr.contains("Found 1 potential security items"),
        "{}",
        audit.stderr
    );
}

#[test]
#[cfg_attr(miri, ignore)]
fn a_failed_sbom_is_an_incomplete_check() {
    let project = project_with("");
    let audit = audit_with(project.path(), "sbom", &[]);
    assert_eq!(audit.result, ONE_FINDING);
    assert_eq!(audit.summary()["checks"][4]["status"], "error");
}

#[cfg(unix)]
#[test]
#[cfg_attr(miri, ignore)]
fn no_open_port_is_reported_only_for_a_complete_empty_scan() {
    const NONE: &str = "No open local listening ports detected.";
    let project = project_with("");
    program(project.path(), "ss", "exit 0");
    let audit = audit_with(project.path(), "network", &[]);
    let network = &audit.summary()["checks"][5];
    assert_eq!(network["status"], "observed", "{network}");
    assert_eq!(
        audit.stderr.contains(NONE),
        network["count"] == 0,
        "{network} {}",
        audit.stderr
    );

    let project = project_with("");
    let audit = audit_with(project.path(), "network", &[]);
    assert_eq!(audit.summary()["checks"][5]["status"], "error");
    assert!(!audit.stderr.contains(NONE), "{}", audit.stderr);
}

#[test]
#[cfg_attr(miri, ignore)]
fn the_public_entry_points_run_the_audit() {
    for modes in ["wrapper", "wrapper-exceptions"] {
        let project = project_with("fn routes() { get(\"/users/{id}\" => show); }\n");
        let audit = audit_with(project.path(), modes, &[]);
        assert_eq!(
            audit.result,
            "err IDOR/BOLA audit found 1 unclassified or unguarded parameterized route(s)",
            "{modes}"
        );
        assert!(
            audit.stdout.contains("Audit finished. Issues found: 1"),
            "{modes}"
        );
    }
}
