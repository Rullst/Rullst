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
#[test]
fn audit_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_AUDIT_REPORT") else {
        return;
    };
    let full = std::env::var_os("RULLST_AUDIT_FULL").is_some();
    let result = run_audit(AuditOptions {
        idor: true,
        sbom: full,
        network: full,
        json: true,
        ..AuditOptions::default()
    });
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
