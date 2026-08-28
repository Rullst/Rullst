//! Small, compiling `auth,learning` LMS scaffold profile.

mod auth_only;
mod controller;
mod middleware;
mod migrations;
mod routes;
mod service;

use controller::FOUNDATION_CONTROLLER;
use middleware::FOUNDATION_AUTH_MIDDLEWARE;
use migrations::FOUNDATION_MIGRATIONS_MODULE;
use service::FOUNDATION_SERVICE;

const RETAINED_FILES: &[&str] = &[
    "src/controllers/auth_controller.rs",
    "src/controllers/lms_controller.rs",
    "src/migrations/m20260601000000_create_lms_tables.rs",
    "src/migrations/m20260827000000_add_learning_access.rs",
    "src/models/category.rs",
    "src/models/course.rs",
    "src/models/course_module.rs",
    "src/models/enrollment.rs",
    "src/models/lesson.rs",
    "src/models/lesson_progress.rs",
    "src/models/lesson_progress_event.rs",
    "src/models/user.rs",
    "src/pages/auth.rs",
    "src/pages/lms.rs",
];

pub(super) fn select(
    mut full_manifest: Vec<(&'static str, String)>,
    hot_reload: bool,
    modules: &[super::LmsModule],
) -> Result<Vec<(&'static str, String)>, super::LmsModuleError> {
    if hot_reload {
        return Err(super::LmsModuleError::HotReloadUnsupported);
    }
    if modules.len() == 1 && modules.contains(&super::LmsModule::Auth) {
        return Ok(auth_only::select(full_manifest));
    }
    full_manifest.retain(|(path, _)| RETAINED_FILES.contains(path));
    full_manifest.extend([
        ("src/main.rs", routes::main_source()),
        (
            "src/controllers/learning_controller.rs",
            FOUNDATION_CONTROLLER.to_string(),
        ),
        (
            "src/controllers/mod.rs",
            "pub mod auth_controller;\npub mod learning_controller;\npub mod lms_controller;\n"
                .to_string(),
        ),
        (
            "src/middlewares/auth_middleware.rs",
            FOUNDATION_AUTH_MIDDLEWARE.to_string(),
        ),
        (
            "src/middlewares/mod.rs",
            "pub mod auth_middleware;\n".to_string(),
        ),
        (
            "src/migrations/mod.rs",
            FOUNDATION_MIGRATIONS_MODULE.to_string(),
        ),
        (
            "src/models/mod.rs",
            "pub mod category;\npub mod course;\npub mod course_module;\npub mod enrollment;\npub mod lesson;\npub mod lesson_progress;\npub mod lesson_progress_event;\npub mod user;\n"
                .to_string(),
        ),
        (
            "src/pages/mod.rs",
            "pub mod auth;\npub mod lms;\n".to_string(),
        ),
        (
            "src/services/learning_service.rs",
            FOUNDATION_SERVICE.to_string(),
        ),
        (
            "src/services/mod.rs",
            "pub mod learning_service;\n".to_string(),
        ),
        (
            "rullst-lms-modules.json",
            "{\n  \"schema_version\": 1,\n  \"modules\": [\"auth\", \"learning\"],\n  \"profile\": \"foundation\"\n}\n"
                .to_string(),
        ),
    ]);
    full_manifest.sort_unstable_by_key(|(path, _)| *path);
    Ok(full_manifest)
}

#[cfg(test)]
mod tests {
    use super::super::{LmsModule, LmsModuleError, file_manifest_for_modules};

    #[test]
    fn foundation_manifest_is_small_explicit_and_excludes_vertical_modules() {
        let manifest = file_manifest_for_modules(
            "demo",
            false,
            "Active Record",
            "Zero-Bundle HTMX",
            &[LmsModule::Auth, LmsModule::Learning],
        )
        .expect("detached foundation manifest");
        assert!(
            manifest
                .iter()
                .any(|(path, _)| *path == "rullst-lms-modules.json")
        );
        assert!(
            manifest
                .iter()
                .any(|(path, _)| *path == "src/services/learning_service.rs")
        );
        for excluded in [
            "src/models/quiz.rs",
            "src/models/achievement.rs",
            "src/services/automation_worker_service.rs",
            "src/services/notification_service.rs",
        ] {
            assert!(manifest.iter().all(|(path, _)| *path != excluded));
        }
        assert!(
            manifest.len() < 30,
            "foundation emitted {} files",
            manifest.len()
        );
    }

    #[test]
    fn auth_manifest_contains_only_the_identity_boundary() {
        let manifest = file_manifest_for_modules(
            "demo",
            false,
            "Active Record",
            "Zero-Bundle HTMX",
            &[LmsModule::Auth],
        )
        .expect("detached auth manifest");
        for required in [
            "src/controllers/auth_controller.rs",
            "src/migrations/m20260827000000_add_auth_identity.rs",
            "src/models/user.rs",
            "rullst-lms-modules.json",
        ] {
            assert!(manifest.iter().any(|(path, _)| *path == required));
        }
        for excluded in [
            "src/models/course.rs",
            "src/models/enrollment.rs",
            "src/services/learning_service.rs",
            "src/models/quiz.rs",
        ] {
            assert!(manifest.iter().all(|(path, _)| *path != excluded));
        }
        assert!(
            manifest.len() < 15,
            "auth profile emitted {} files",
            manifest.len()
        );
    }

    #[test]
    fn foundation_hot_reload_fails_explicitly() {
        assert_eq!(
            file_manifest_for_modules(
                "demo",
                true,
                "Active Record",
                "Zero-Bundle HTMX",
                &[LmsModule::Auth, LmsModule::Learning],
            ),
            Err(LmsModuleError::HotReloadUnsupported)
        );
    }
}
