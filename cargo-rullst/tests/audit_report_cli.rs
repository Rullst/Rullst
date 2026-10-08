//! Process-level contract for `cargo rullst audit --report`.

#![cfg(unix)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const APP: &str = r#"
use rullst::Server;

fn routes() -> Router {
    routes![
        get("/" => index),
        post("/login" => login),
    ]
}

#[tokio::main]
async fn main() {
    Server::new(routes()).run(3000).await;
}

fn page() -> Html {
    html! { <html lang="en"><img src="/logo.png" /><input id="q" aria-label="Search" /></html> }
}
"#;

/// A Stripe-shaped live key built at run time, so no literal key is committed.
fn secret() -> String {
    ["sk", "live", "Fx7Kq2Lm9Pz4Rt8Wv3Yb"].join("_")
}

fn executable(path: &Path, body: &str) {
    fs::write(path, body).expect("fake tool");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable");
}

/// The system `git`, when one is installed.
fn system_git() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
}

fn fixture() -> (tempfile::TempDir, PathBuf, bool) {
    let root = tempfile::tempdir().expect("fixture project");
    let project = root.path().join("app");
    fs::create_dir_all(project.join("src")).expect("source directory");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"report-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest");
    fs::write(project.join("src/main.rs"), APP).expect("source");
    fs::write(
        project.join("Rullst.toml"),
        "[security]\ncsp = \"default-src 'self'; script-src 'self' 'unsafe-inline'\"\n",
    )
    .expect("configuration");
    fs::write(
        project.join("settings.toml"),
        format!("stripe = \"{}\"\n", secret()),
    )
    .expect("tracked secret fixture");

    let tools = root.path().join("tools");
    fs::create_dir_all(&tools).expect("tool directory");
    // `cargo audit --version` and `cargo audit` succeed; `cargo metadata` prints nothing.
    executable(&tools.join("cargo"), "#!/bin/sh\nexit 0\n");
    let git = system_git();
    if let Some(git) = &git {
        std::os::unix::fs::symlink(git, tools.join("git")).expect("git link");
        for arguments in [&["init", "-q"][..], &["add", "."]] {
            let status = Command::new(git)
                .args(arguments)
                .current_dir(&project)
                .status()
                .expect("git");
            assert!(status.success());
        }
    }
    (root, tools, git.is_some())
}

fn run(project: &Path, tools: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(project)
        .arg("audit")
        .args(arguments)
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .env("PATH", tools)
        .output()
        .expect("audit --report process")
}

#[test]
fn audit_report_json_writes_a_parseable_report_and_fails_on_findings() {
    let (root, tools, has_git) = fixture();
    let project = root.path().join("app");

    let output = run(&project, &tools, &["--report", "json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("Security checks\n"), "{stdout}");
    assert!(
        stdout.contains("✗ Security headers and CSP — 1 finding(s)"),
        "{stdout}"
    );
    assert!(stdout.contains("Report written to SECURITY_REPORT.json"));
    assert!(!stdout.contains('\u{1b}'));

    let text = fs::read_to_string(project.join("SECURITY_REPORT.json")).expect("JSON report");
    assert!(!text.contains(&secret()));
    let report: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(report["schema_version"], "rullst.cli-audit-report.v1");
    assert_eq!(report["standard"]["version"], "5.0.0");
    assert_eq!(report["exit_non_zero"], true);
    let check = |id: &str| {
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("check {id}"))
    };
    assert_eq!(check("security_headers")["status"], "findings");
    assert_eq!(check("csrf")["status"], "no_findings");
    assert_eq!(check("dependency_audit")["status"], "no_findings");
    assert_eq!(check("auth_rate_limit")["status"], "findings");
    assert_eq!(check("a11y_img_alt")["status"], "findings");
    assert_eq!(check("a11y_form_labels")["status"], "no_findings");
    if has_git {
        let secrets = check("committed_secrets");
        assert_eq!(secrets["status"], "findings");
        assert_eq!(secrets["findings"][0]["file"], "settings.toml");
        assert_eq!(secrets["findings"][0]["preview"], "sk_l…");
    }
    assert!(!report["not_evaluated"].as_array().unwrap().is_empty());

    // The default format is Markdown; `--output` chooses the path.
    fs::create_dir_all(project.join("evidence")).expect("output directory");
    let markdown = run(
        &project,
        &tools,
        &["--report", "--output", "evidence/report.md"],
    );
    assert_eq!(markdown.status.code(), Some(1));
    let text = fs::read_to_string(project.join("evidence/report.md")).expect("Markdown report");
    assert!(text.starts_with("# Rullst Security Report"));
    assert!(text.contains("NOT EVALUATED"));
    assert!(!text.contains(&secret()));

    let html = run(&project, &tools, &["--report", "html"]);
    assert_eq!(html.status.code(), Some(1));
    let text = fs::read_to_string(project.join("SECURITY_REPORT.html")).expect("HTML report");
    assert!(text.starts_with("<!DOCTYPE html>") && !text.contains("<script"));

    let conflict = run(&project, &tools, &["--report", "json", "--json"]);
    assert_eq!(conflict.status.code(), Some(2));
}
