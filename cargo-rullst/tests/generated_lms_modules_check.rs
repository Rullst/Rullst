//! Materialized proof that the LMS starter compiles and passes its tests.

#![allow(clippy::expect_used, clippy::panic)]

use cargo_rullst::blueprints::{self, LMS_BLUEPRINT_ID};
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use std::{fs, path::Path, path::PathBuf, process::Command};

fn materialize_and_test(profile: &str, hot_reload: bool, required: &[&str], excluded: &[&str]) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir = std::env::temp_dir().join(format!(
        "rullst-generated-lms-{profile}-{}",
        rand::random::<u64>()
    ));
    fs::create_dir_all(&project_dir).expect("LMS starter project directory");

    let manifest = build_cargo_toml(
        &format!("generated-lms-{profile}"),
        hot_reload,
        true,
        "Sqlite",
        &[],
        false,
        false,
        LMS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .expect("LMS starter Cargo.toml");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write LMS starter Cargo.toml");
    let workspace_lock = workspace.join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(workspace_lock, project_dir.join("Cargo.lock")).expect("copy workspace lockfile");
    }
    blueprints::apply(
        LMS_BLUEPRINT_ID,
        &project_dir,
        &format!("generated-lms-{profile}"),
        &format!("generated_lms_{}", profile.replace('-', "_")),
        false,
        hot_reload,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .expect("apply the LMS starter");

    for path in required {
        assert!(project_dir.join(path).exists(), "missing {path}");
    }
    for path in excluded {
        assert!(!project_dir.join(path).exists(), "unexpected {path}");
    }

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
        // Generated projects are independent workspaces. Use the same compact,
        // single-job profile as the broader blueprint matrix so four Rust test
        // harnesses cannot exhaust a small developer machine in parallel.
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_INCREMENTAL", "false")
        .env("CARGO_BUILD_JOBS", "1")
        .output()
        .expect("run generated LMS starter cargo test");
    if !output.status.success() {
        panic!(
            "generated LMS {profile} profile failed cargo test\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("LMS starter project cleanup");
}

#[test]
fn lms_starter_passes_generated_cargo_tests() {
    materialize_and_test(
        "starter",
        false,
        &[
            "static/media/memory-safety.en.vtt",
            "src/models/course.rs",
            "src/models/enrollment.rs",
            "src/services/learning_service.rs",
        ],
        &[
            "src/models/quiz.rs",
            "src/models/achievement.rs",
            "rullst-lms-modules.json",
            "src/lib.rs",
        ],
    );
}

#[test]
fn lms_starter_with_hot_reload_passes_generated_cargo_tests() {
    materialize_and_test(
        "starter-hot",
        true,
        &["src/lib.rs", "src/main.rs", "src/models/course.rs"],
        &["src/models/quiz.rs", "src/models/achievement.rs"],
    );
}
