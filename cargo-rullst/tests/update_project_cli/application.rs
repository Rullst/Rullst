use super::{
    Fixture,
    review::{review, verified},
    text,
    verification::fixture,
};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn digest(fixture: &Fixture, stage: &Path) -> String {
    let output = review(fixture, stage);
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    report["review_sha256"].as_str().unwrap().into()
}

fn invoke(fixture: &Fixture, operation: &str, stage: &Path, digest: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "project", operation, "--verified"])
        .arg(stage)
        .args(["--approved-review", digest, "--json"])
        .env("XDG_CACHE_HOME", &fixture.base)
        .env("LOCALAPPDATA", &fixture.base)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .output()
        .unwrap()
}

#[test]
fn explicit_application_and_recovery_preserve_unrelated_work_and_file_permissions() {
    let fixture = fixture("// original user work\nfn main() {}\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            fixture.app.join("Cargo.toml"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
    }
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let index = fs::read(fixture.app.join(".git/index")).unwrap();
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    fs::remove_file(fixture.base.join("build-executed")).unwrap();
    let applied = invoke(&fixture, "apply", &stage, &digest);
    assert!(applied.status.success(), "{}", text(&applied));
    assert!(
        fs::read_to_string(fixture.app.join("Cargo.toml"))
            .unwrap()
            .contains(&format!("version = \"={}\"", env!("CARGO_PKG_VERSION")))
    );
    assert_eq!(fs::read(fixture.app.join(".git/index")).unwrap(), index);
    assert!(!fixture.base.join("build-executed").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(fixture.app.join("Cargo.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
    fs::write(
        fixture.app.join("src/main.rs"),
        "// later unrelated edit\nfn main() {}\n",
    )
    .unwrap();
    fs::write(fixture.app.join("new-user-note.txt"), "retain me").unwrap();
    let restored = invoke(&fixture, "recover", &stage, &digest);
    assert!(restored.status.success(), "{}", text(&restored));
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(fixture.app.join("Cargo.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
    assert!(
        fs::read_to_string(fixture.app.join("src/main.rs"))
            .unwrap()
            .contains("later unrelated edit")
    );
    assert_eq!(
        fs::read_to_string(fixture.app.join("new-user-note.txt")).unwrap(),
        "retain me"
    );
    assert_eq!(fs::read(fixture.app.join(".git/index")).unwrap(), index);
    let again = invoke(&fixture, "recover", &stage, &digest);
    assert!(again.status.success(), "{}", text(&again));
    assert_eq!(
        serde_json::from_slice::<Value>(&again.stdout).unwrap()["file_operations"],
        0
    );
    assert!(!fs::read_dir(&fixture.app).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".rullst-update-stage-")
    }));
}

#[test]
fn wrong_approval_stale_sources_and_changed_verified_bytes_never_apply() {
    let fixture = fixture("fn main() {}\n");
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    let denied = invoke(&fixture, "apply", &stage, &"0".repeat(64));
    assert!(!denied.status.success());
    assert!(text(&denied).contains("approved review differs"));
    fs::write(
        fixture.app.join("src/main.rs"),
        "fn main() { /* user change */ }\n",
    )
    .unwrap();
    assert!(!invoke(&fixture, "apply", &stage, &digest).status.success());
    fs::write(fixture.app.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(stage.join("candidate/Cargo.toml"), "changed\n").unwrap();
    assert!(!invoke(&fixture, "apply", &stage, &digest).status.success());
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
    assert!(!stage.join("application.json").exists());
}

#[test]
fn recovery_handles_a_partial_application_and_removes_only_its_new_lockfile() {
    let fixture = fixture("fn main() {}\n");
    fs::remove_file(fixture.app.join("Cargo.lock")).unwrap();
    let original = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    let applied = invoke(&fixture, "apply", &stage, &digest);
    assert!(applied.status.success(), "{}", text(&applied));
    assert!(fixture.app.join("Cargo.lock").is_file());
    // Reproduce the persisted state after the first of two file replacements:
    // the new Cargo.lock exists, but Cargo.toml still has its pre-update bytes.
    fs::write(fixture.app.join("Cargo.toml"), &original).unwrap();
    let intent_path = stage.join("application.json");
    let mut intent: Value = serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
    intent["phase"] = "applying".into();
    fs::write(intent_path, serde_json::to_vec(&intent).unwrap()).unwrap();
    let restored = invoke(&fixture, "recover", &stage, &digest);
    assert!(restored.status.success(), "{}", text(&restored));
    assert_eq!(
        serde_json::from_slice::<Value>(&restored.stdout).unwrap()["file_operations"],
        1
    );
    assert!(!fixture.app.join("Cargo.lock").exists());
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), original);
}

#[test]
fn recovery_preflights_all_targets_and_refuses_divergent_user_edits() {
    let fixture = fixture("fn main() {}\n");
    fs::remove_file(fixture.app.join("Cargo.lock")).unwrap();
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    let applied = invoke(&fixture, "apply", &stage, &digest);
    assert!(applied.status.success(), "{}", text(&applied));
    let after = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    fs::write(fixture.app.join("Cargo.lock"), "later user change\n").unwrap();
    let output = invoke(&fixture, "recover", &stage, &digest);
    assert!(!output.status.success());
    assert!(
        text(&output).contains("refuses a divergent user edit"),
        "{}",
        text(&output)
    );
    assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), after);
    assert_eq!(
        fs::read_to_string(fixture.app.join("Cargo.lock")).unwrap(),
        "later user change\n"
    );
}

#[test]
fn source_lock_serializes_distinct_preparations_of_the_same_project() {
    use sha2::{Digest, Sha256};
    let fixture = fixture("fn main() {}\n");
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    let root = fixture.app.canonicalize().unwrap();
    let name = format!(
        "source-{}.lock",
        hex::encode(Sha256::digest(root.as_os_str().as_encoded_bytes()))
    );
    let lock = fs::File::create(fixture.base.join("rullst-update-v1").join(name)).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        lock.set_permissions(fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    lock.lock().unwrap();
    let output = invoke(&fixture, "apply", &stage, &digest);
    lock.unlock().unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("busy"), "{}", text(&output));
    assert!(!stage.join("application.json").exists());
}

#[test]
fn recovery_rejects_changed_intent_scope_and_permissions_before_writing() {
    let fixture = fixture("fn main() {}\n");
    let stage = verified(&fixture);
    let digest = digest(&fixture, &stage);
    let output = invoke(&fixture, "apply", &stage, &digest);
    assert!(output.status.success(), "{}", text(&output));
    let after = fs::read(fixture.app.join("Cargo.toml")).unwrap();
    let path = stage.join("application.json");
    let intent: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for field in ["source", "permissions"] {
        let mut changed = intent.clone();
        if field == "source" {
            changed["source"] = fixture
                .base
                .join("another-project")
                .to_string_lossy()
                .as_ref()
                .into();
        } else {
            changed["permissions"][0]["readonly"] =
                (!changed["permissions"][0]["readonly"].as_bool().unwrap()).into();
        }
        fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        let output = invoke(&fixture, "recover", &stage, &digest);
        assert!(!output.status.success(), "{}", text(&output));
        assert_eq!(fs::read(fixture.app.join("Cargo.toml")).unwrap(), after);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn enforced_file_size_limit_terminates_staging_without_changing_originals() {
    use std::os::unix::process::ExitStatusExt;
    let fixture = fixture("fn main() {}\n");
    let manifest = fixture.app.join("Cargo.toml");
    let mut original = fs::read_to_string(&manifest).unwrap();
    original.push_str(&format!("\n# {}\n", "retained comment ".repeat(1024)));
    fs::write(&manifest, &original).unwrap();
    let original_lock = fs::read(fixture.app.join("Cargo.lock")).ok();
    let stage = verified(&fixture);
    let approval = digest(&fixture, &stage);
    let output = Command::new("bash")
        .args([
            "-c",
            "ulimit -c 0; ulimit -f 1; exec \"$@\"",
            "rullst-file-limit",
        ])
        .arg(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "project", "apply", "--verified"])
        .arg(&stage)
        .args(["--approved-review", &approval, "--json"])
        .env("XDG_CACHE_HOME", &fixture.base)
        .env("LOCALAPPDATA", &fixture.base)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.signal(), Some(25), "{}", text(&output)); // SIGXFSZ on Linux
    assert_eq!(fs::read_to_string(manifest).unwrap(), original);
    assert_eq!(fs::read(fixture.app.join("Cargo.lock")).ok(), original_lock);
    assert!(!stage.join("application.json").exists());
}
