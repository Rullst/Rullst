//! The LMS starter's file set and entrypoint overlays.

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
    "static/media/first-project.en.vtt",
    "static/media/memory-safety.en.vtt",
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

/// Keeps the starter's files from the shared templates and adds its entrypoints,
/// controllers, middleware, migrations index and learning service.
pub(super) fn starter(
    mut full_manifest: Vec<(&'static str, String)>,
    project_name_safe: &str,
    hot_reload: bool,
) -> Vec<(&'static str, String)> {
    full_manifest.retain(|(path, _)| RETAINED_FILES.contains(path));
    if let Some((_, source)) = full_manifest
        .iter_mut()
        .find(|(path, _)| *path == "src/controllers/auth_controller.rs")
    {
        *source = super::auth::identity_controller();
    }
    if hot_reload {
        full_manifest.extend([
            ("src/lib.rs", routes::hot_lib_source()),
            ("src/main.rs", routes::hot_main_source(project_name_safe)),
        ]);
    } else {
        full_manifest.push(("src/main.rs", routes::main_source()));
    }
    full_manifest.extend([
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
    ]);
    full_manifest.sort_unstable_by_key(|(path, _)| *path);
    full_manifest
}

#[cfg(test)]
mod tests {
    use super::super::file_manifest;

    fn manifest(hot_reload: bool) -> Vec<(&'static str, String)> {
        file_manifest("demo", hot_reload, "Active Record", "Zero-Bundle HTMX")
    }

    fn source<'a>(manifest: &'a [(&'static str, String)], name: &str) -> &'a str {
        manifest
            .iter()
            .find(|(path, _)| *path == name)
            .map(|(_, source)| source.as_str())
            .unwrap_or_default()
    }

    #[test]
    fn starter_is_small_explicit_and_excludes_academy_verticals() {
        let manifest = manifest(false);
        for required in [
            "src/services/learning_service.rs",
            "src/controllers/learning_controller.rs",
            "src/models/course.rs",
            "src/models/enrollment.rs",
            "src/pages/lms.rs",
            "static/media/memory-safety.en.vtt",
        ] {
            assert!(
                manifest.iter().any(|(path, _)| *path == required),
                "{required}"
            );
        }
        for excluded in [
            "src/models/quiz.rs",
            "src/models/achievement.rs",
            "src/services/automation_worker_service.rs",
            "src/services/notification_service.rs",
            "rullst-lms-modules.json",
            "src/lib.rs",
        ] {
            assert!(
                manifest.iter().all(|(path, _)| *path != excluded),
                "{excluded}"
            );
        }
        assert!(
            manifest.len() < 30,
            "starter emitted {} files",
            manifest.len()
        );
        assert_eq!(
            manifest
                .iter()
                .filter(|(path, _)| *path == "src/main.rs")
                .count(),
            1
        );
        assert!(
            !source(&manifest, "src/controllers/auth_controller.rs")
                .contains("provision_self_registration_with_tx")
        );
    }

    #[test]
    fn hot_reload_exports_the_router_library() {
        let manifest = manifest(true);
        assert!(
            source(&manifest, "src/lib.rs").contains("pub extern \"C\" fn rullst_router_init()")
        );
        assert!(source(&manifest, "src/lib.rs").contains("pub fn router()"));
        assert!(source(&manifest, "src/main.rs").contains("rullst::Server::new_hot(lib_path)"));
        assert!(source(&manifest, "src/main.rs").contains("demo::router()?"));
        assert_eq!(
            manifest
                .iter()
                .filter(|(path, _)| *path == "src/main.rs")
                .count(),
            1
        );
    }
}
