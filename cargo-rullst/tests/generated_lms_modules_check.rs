//! Materialized proof for the detached `auth,learning` LMS profile.

#![allow(clippy::expect_used, clippy::panic)]

use cargo_rullst::blueprints::{self, LMS_BLUEPRINT_ID, lms::LmsModule};
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use std::{fs, path::Path, path::PathBuf, process::Command};

#[test]
fn selected_auth_learning_profile_passes_generated_cargo_tests() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir = std::env::temp_dir().join(format!(
        "rullst-generated-lms-foundation-{}",
        rand::random::<u64>()
    ));
    fs::create_dir_all(&project_dir).expect("foundation project directory");

    let manifest = build_cargo_toml(
        "generated-lms-foundation",
        false,
        true,
        "Sqlite",
        false,
        false,
        LMS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .expect("foundation Cargo.toml");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write foundation Cargo.toml");
    let workspace_lock = workspace.join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(workspace_lock, project_dir.join("Cargo.lock")).expect("copy workspace lockfile");
    }
    blueprints::apply_with_lms_modules(
        LMS_BLUEPRINT_ID,
        &project_dir,
        "generated-lms-foundation",
        "generated_lms_foundation",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
        Some(&[LmsModule::Auth, LmsModule::Learning]),
    )
    .expect("apply foundation modules");

    assert!(!project_dir.join("src/models/quiz.rs").exists());
    assert!(!project_dir.join("src/models/achievement.rs").exists());
    assert!(project_dir.join("rullst-lms-modules.json").exists());

    let target_root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let output = Command::new(env!("CARGO"))
        .arg("test")
        .arg("--offline")
        .arg("--all-targets")
        .arg("--manifest-path")
        .arg(project_dir.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            target_root.join("generated-scaffold-check"),
        )
        .output()
        .expect("run generated foundation cargo test");
    if !output.status.success() {
        panic!(
            "generated LMS foundation failed cargo test\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("foundation project cleanup");
}
