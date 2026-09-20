use super::{Fixture, text};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) fn fixture(source: &str) -> Fixture {
    let fixture = Fixture::current();
    fs::write(
        fixture.app.join("build.rs"),
        r#"fn main() {
        std::fs::write(std::env::var("RULLST_PROJECT_BUILD_MARKER").unwrap(), "executed").unwrap();
    }"#,
    )
    .unwrap();
    fs::write(fixture.app.join("src/main.rs"), source).unwrap();
    fixture
}

pub(super) fn prepare(fixture: &Fixture) -> PathBuf {
    let output = fixture.prepare();
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    PathBuf::from(report["prepared_directory"].as_str().unwrap())
}

pub(super) fn verify(fixture: &Fixture, stage: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "project", "verify", "--prepared"])
        .arg(stage)
        .args(arguments)
        .arg("--json")
        .env("XDG_CACHE_HOME", &fixture.base)
        .env("LOCALAPPDATA", &fixture.base)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .env(
            "RULLST_PROJECT_BUILD_MARKER",
            fixture.base.join("build-executed"),
        )
        .output()
        .unwrap()
}

#[test]
fn verification_requires_explicit_execution_consent_before_reading_preparation() {
    let fixture = fixture("fn main() {}\n");
    let output = verify(&fixture, &fixture.base.join("does-not-exist"), &[]);
    assert!(!output.status.success());
    assert!(text(&output).contains("--allow-project-code is required"));
    assert!(!fixture.base.join("build-executed").exists());
}

#[test]
fn verification_runs_real_locked_checks_and_tests_only_in_a_fresh_candidate() {
    let fixture =
        fixture("fn main() {}\n#[test] fn real_application_test() { assert_eq!(2 + 2, 4); }\n");
    let stage = prepare(&fixture);
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let lock = fs::read(fixture.app.join("Cargo.lock")).unwrap();
    assert!(!fixture.base.join("build-executed").exists());
    let output = verify(
        &fixture,
        &stage,
        &["--allow-project-code", "--all-features"],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(fixture.base.join("build-executed").exists());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], "rullst.project-verification.v1");
    assert_eq!(report["phase"], "verified");
    assert_eq!(report["application_authorized"], false);
    assert_eq!(report["production_ready"], false);
    assert_eq!(report["features"], serde_json::json!(["--all-features"]));
    let candidate = Path::new(report["verified_candidate_directory"].as_str().unwrap());
    assert_ne!(candidate, stage.join("candidate"));
    assert!(!candidate.starts_with(&fixture.app));
    for observation in report["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| matches!(entry["args"][0].as_str(), Some("check" | "test")))
    {
        assert!(
            observation["args"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "--locked")
        );
    }
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    assert_eq!(fs::read(fixture.app.join("Cargo.lock")).unwrap(), lock);
    assert!(!fixture.app.join("target").exists());
    assert!(!stage.join("candidate/target").exists());
    assert_eq!(
        serde_json::from_slice::<Value>(
            &fs::read(candidate.parent().unwrap().join("verification.json")).unwrap()
        )
        .unwrap(),
        report
    );
}

#[test]
fn stale_source_candidate_and_baseline_are_rejected_before_project_code() {
    let fixture = fixture("fn main() {}\n");
    let stage = prepare(&fixture);
    for path in [
        fixture.app.join("src/main.rs"),
        stage.join("candidate/src/main.rs"),
        stage.join("before/src/main.rs"),
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, "fn main() { /* later edit */ }\n").unwrap();
        let output = verify(&fixture, &stage, &["--allow-project-code"]);
        assert!(!output.status.success(), "{}", text(&output));
        assert!(!fixture.base.join("build-executed").exists());
        fs::write(path, original).unwrap();
    }
    let record = stage.join("preparation.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    let mut previous_catalog = value.clone();
    previous_catalog["plan"]["rule_catalog"] = "rullst-upgrade-rules-v1".into();
    fs::write(&record, serde_json::to_vec(&previous_catalog).unwrap()).unwrap();
    let rejected = verify(&fixture, &stage, &["--allow-project-code"]);
    assert!(!rejected.status.success(), "{}", text(&rejected));
    assert!(text(&rejected).contains("current migration catalog"));
    assert!(!fixture.base.join("build-executed").exists());
    value["execution_authorized"] = true.into();
    fs::write(record, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        !verify(&fixture, &stage, &["--allow-project-code"])
            .status
            .success()
    );
    assert!(!fixture.base.join("build-executed").exists());
}

#[test]
fn failed_application_tests_retain_diagnostics_without_acceptance_or_original_changes() {
    let fixture = fixture(
        "fn main() {}\n#[test] fn failing_app_test() { panic!(\"acceptance must fail\"); }\n",
    );
    let stage = prepare(&fixture);
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let output = verify(&fixture, &stage, &["--allow-project-code"]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("Cargo verification failed"),
        "{}",
        text(&output)
    );
    assert!(fixture.base.join("build-executed").exists());
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    for entry in fs::read_dir(fixture.base.join("rullst-update-v1")).unwrap() {
        assert!(!entry.unwrap().path().join("verification.json").exists());
    }
}

#[test]
fn application_test_writes_cannot_become_accepted_source_edits() {
    let fixture = fixture(
        "fn main() {}\n#[test] fn changes_source() { std::fs::write(\"src/main.rs\", \"fn main() {}\").unwrap(); }\n",
    );
    let stage = prepare(&fixture);
    let output = verify(&fixture, &stage, &["--allow-project-code"]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("execution changed a prepared source input"),
        "{}",
        text(&output)
    );
    assert!(
        fs::read_to_string(fixture.app.join("src/main.rs"))
            .unwrap()
            .contains("changes_source")
    );
}

#[test]
fn dry_run_shows_selected_commands_without_execution_and_conflicting_policy_is_rejected() {
    let fixture = fixture("fn main() {}\n");
    let stage = prepare(&fixture);
    let output = verify(&fixture, &stage, &["--dry-run", "--all-features"]);
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["schema_version"],
        "rullst.project-verification-plan.v1"
    );
    assert_eq!(report["execution_authorized"], false);
    assert_eq!(report["application_authorized"], false);
    assert!(
        report["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["args"][0] == "test")
    );
    assert!(!fixture.base.join("build-executed").exists());
    let conflicting = verify(
        &fixture,
        &stage,
        &["--allow-project-code", "--allow-network"],
    );
    assert!(!conflicting.status.success());
    assert!(text(&conflicting).contains("conflicts with CARGO_NET_OFFLINE"));
    assert!(!fixture.base.join("build-executed").exists());
}

#[test]
fn operation_lock_prevents_concurrent_verification_before_execution() {
    let fixture = fixture("fn main() {}\n");
    let stage = prepare(&fixture);
    let lock = fs::File::create(stage.join("operation.lock")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        lock.set_permissions(fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    lock.lock().unwrap();
    let output = verify(&fixture, &stage, &["--allow-project-code"]);
    lock.unlock().unwrap();
    assert!(!output.status.success());
    assert!(
        text(&output).contains("prepared project is busy"),
        "{}",
        text(&output)
    );
    assert!(!fixture.base.join("build-executed").exists());
}

#[test]
fn command_deadline_rejects_a_hanging_project_without_acceptance() {
    let fixture = fixture(
        "fn main() {}\n#[test] fn hangs() { std::thread::sleep(std::time::Duration::from_secs(60)); }\n",
    );
    let stage = prepare(&fixture);
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let start = std::time::Instant::now();
    let output = verify(
        &fixture,
        &stage,
        &["--allow-project-code", "--timeout-seconds", "2"],
    );
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("timed out"), "{}", text(&output));
    assert!(start.elapsed() < std::time::Duration::from_secs(30));
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    for entry in fs::read_dir(fixture.base.join("rullst-update-v1")).unwrap() {
        assert!(!entry.unwrap().path().join("verification.json").exists());
    }
}
