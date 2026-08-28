//! Materialized proof for detached LMS module profiles.

#![allow(clippy::expect_used, clippy::panic)]

use cargo_rullst::blueprints::{self, LMS_BLUEPRINT_ID, lms::LmsModule};
use cargo_rullst::generators::project::cargo_toml::build_cargo_toml;
use std::{fs, path::Path, path::PathBuf, process::Command};

fn materialize_and_test(
    profile: &str,
    modules: &[LmsModule],
    required: &[&str],
    excluded: &[&str],
) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = crate_dir.parent().expect("workspace root");
    let project_dir = std::env::temp_dir().join(format!(
        "rullst-generated-lms-{profile}-{}",
        rand::random::<u64>()
    ));
    fs::create_dir_all(&project_dir).expect("selected profile project directory");

    let manifest = build_cargo_toml(
        &format!("generated-lms-{profile}"),
        false,
        true,
        "Sqlite",
        false,
        false,
        LMS_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .expect("selected profile Cargo.toml");
    fs::write(project_dir.join("Cargo.toml"), manifest).expect("write selected profile Cargo.toml");
    let workspace_lock = workspace.join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(workspace_lock, project_dir.join("Cargo.lock")).expect("copy workspace lockfile");
    }
    blueprints::apply_with_lms_modules(
        LMS_BLUEPRINT_ID,
        &project_dir,
        &format!("generated-lms-{profile}"),
        &format!("generated_lms_{}", profile.replace('-', "_")),
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
        Some(modules),
    )
    .expect("apply selected LMS modules");

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
        .output()
        .expect("run generated foundation cargo test");
    if !output.status.success() {
        panic!(
            "generated LMS {profile} profile failed cargo test\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::remove_dir_all(project_dir).expect("selected profile project cleanup");
}

#[test]
fn selected_auth_profile_passes_generated_cargo_tests() {
    materialize_and_test(
        "auth",
        &[LmsModule::Auth],
        &[
            "rullst-lms-modules.json",
            "src/models/user.rs",
            "src/controllers/auth_controller.rs",
        ],
        &[
            "src/models/course.rs",
            "src/models/enrollment.rs",
            "src/models/quiz.rs",
        ],
    );
}

#[test]
fn selected_auth_learning_profile_passes_generated_cargo_tests() {
    materialize_and_test(
        "foundation",
        &[LmsModule::Auth, LmsModule::Learning],
        &[
            "rullst-lms-modules.json",
            "src/models/course.rs",
            "src/models/enrollment.rs",
        ],
        &["src/models/quiz.rs", "src/models/achievement.rs"],
    );
}
