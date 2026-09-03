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
        &[],
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
        // Generated projects are independent workspaces. Use the same compact,
        // single-job profile as the broader blueprint matrix so four Rust test
        // harnesses cannot exhaust a small developer machine in parallel.
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_INCREMENTAL", "false")
        .env("CARGO_BUILD_JOBS", "1")
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
            "static/media/memory-safety.en.vtt",
            "src/models/course.rs",
            "src/models/enrollment.rs",
        ],
        &["src/models/quiz.rs", "src/models/achievement.rs"],
    );
}

#[test]
fn selected_auth_learning_assessment_profile_passes_generated_cargo_tests() {
    materialize_and_test(
        "assessment-foundation",
        &[LmsModule::Auth, LmsModule::Learning, LmsModule::Assessment],
        &[
            "rullst-lms-modules.json",
            "src/models/quiz.rs",
            "src/controllers/assessment_controller.rs",
            "src/services/assessment_service.rs",
            "src/migrations/m20260828000000_add_assessment.rs",
        ],
        &[
            "src/models/achievement.rs",
            "src/models/leaderboard_entry.rs",
            "src/services/automation_worker_service.rs",
            "src/services/notification_service.rs",
            "src/services/outbox_service.rs",
        ],
    );
}

#[test]
fn selected_auth_learning_gamification_profile_passes_generated_cargo_tests() {
    materialize_and_test(
        "gamification-foundation",
        &[
            LmsModule::Auth,
            LmsModule::Learning,
            LmsModule::Gamification,
        ],
        &[
            "rullst-lms-modules.json",
            "src/models/activity.rs",
            "src/models/score_event.rs",
            "src/models/leaderboard_entry.rs",
            "src/controllers/gamification_controller.rs",
            "src/services/gamification_service.rs",
            "src/migrations/m20260828000000_add_gamification.rs",
        ],
        &[
            "src/models/quiz.rs",
            "src/models/achievement.rs",
            "src/services/automation_worker_service.rs",
            "src/services/notification_service.rs",
            "src/services/outbox_service.rs",
        ],
    );
}
