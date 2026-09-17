use super::{
    Fixture, text,
    verification::{fixture, prepare, verify},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub(super) fn verified(fixture: &Fixture) -> PathBuf {
    let stage = prepare(fixture);
    let output = verify(fixture, &stage, &["--allow-project-code"]);
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    PathBuf::from(report["verified_directory"].as_str().unwrap())
}

pub(super) fn review(fixture: &Fixture, stage: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "project", "review", "--verified"])
        .arg(stage)
        .arg("--json")
        .env("XDG_CACHE_HOME", &fixture.base)
        .env("LOCALAPPDATA", &fixture.base)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .output()
        .unwrap()
}

#[test]
fn review_binds_complete_dependency_diff_without_reexecuting_or_applying() {
    let fixture = fixture("fn main() {}\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(fixture.app.join("tool.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            fixture.app.join("tool.sh"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let stage = verified(&fixture);
    fs::remove_file(fixture.base.join("build-executed")).unwrap();
    let first = review(&fixture, &stage);
    assert!(first.status.success(), "{}", text(&first));
    let report: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(
        report["review"]["schema_version"],
        "rullst.project-review.v1"
    );
    assert_eq!(report["review"]["application_authorized"], false);
    assert_eq!(report["review"]["changes"].as_array().unwrap().len(), 1);
    let diff = report["diff"].as_str().unwrap();
    assert!(diff.contains(&format!("version = \"={}\"", env!("CARGO_PKG_VERSION"))));
    assert!(diff.contains("version = \"12\""));
    assert!(
        !diff.contains("tool.sh"),
        "permission normalization must not invent a source edit"
    );
    assert_eq!(report["review_sha256"].as_str().unwrap().len(), 64);
    let again = review(&fixture, &stage);
    assert!(again.status.success(), "{}", text(&again));
    assert_eq!(
        serde_json::from_slice::<Value>(&again.stdout).unwrap(),
        report
    );
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    assert!(!fixture.base.join("build-executed").exists());
}

#[test]
fn review_rejects_changed_logs_candidate_and_verification_authority() {
    let fixture = fixture("fn main() {}\n");
    let stage = verified(&fixture);
    for path in [
        stage.join("command-0.stdout"),
        stage.join("candidate/Cargo.toml"),
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, "changed\n").unwrap();
        let output = review(&fixture, &stage);
        assert!(!output.status.success(), "{}", text(&output));
        assert!(output.stdout.is_empty());
        fs::write(path, original).unwrap();
    }
    let path = stage.join("verification.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["application_authorized"] = true.into();
    fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
    assert!(!review(&fixture, &stage).status.success());
}
