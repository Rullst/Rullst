// src/blueprints/lms.rs — LMS starter: catalog, courses, modules, lessons, an
// accessible player, enrollment, progress, login and a Nexus admin.
// The templates under `lms/src/` and `lms/static/` mirror the generated
// project's layout.
use super::common;

/// Files every LMS variant emits unchanged.
const COMMON_FILES: [(&str, &str); 23] = [
    (
        "src/controllers/learning_controller.rs",
        include_str!("lms/src/controllers/learning_controller.rs.template"),
    ),
    // The bounded server-rendered catalog and course pages.
    (
        "src/controllers/lms_controller.rs",
        include_str!("lms/src/controllers/lms_controller.rs.template"),
    ),
    (
        "src/controllers/mod.rs",
        include_str!("lms/src/controllers/mod.rs.template"),
    ),
    (
        "src/middlewares/auth_middleware.rs",
        include_str!("lms/src/middlewares/auth_middleware.rs.template"),
    ),
    (
        "src/middlewares/mod.rs",
        include_str!("lms/src/middlewares/mod.rs.template"),
    ),
    (
        "src/migrations/m20260601000000_create_lms_tables.rs",
        include_str!("lms/src/migrations/m20260601000000_create_lms_tables.rs.template"),
    ),
    (
        "src/migrations/m20260827000000_add_learning_access.rs",
        include_str!("lms/src/migrations/m20260827000000_add_learning_access.rs.template"),
    ),
    (
        "src/migrations/mod.rs",
        include_str!("lms/src/migrations/mod.rs.template"),
    ),
    (
        "src/models/category.rs",
        include_str!("lms/src/models/category.rs.template"),
    ),
    (
        "src/models/course.rs",
        include_str!("lms/src/models/course.rs.template"),
    ),
    (
        "src/models/course_module.rs",
        include_str!("lms/src/models/course_module.rs.template"),
    ),
    (
        "src/models/enrollment.rs",
        include_str!("lms/src/models/enrollment.rs.template"),
    ),
    (
        "src/models/lesson.rs",
        include_str!("lms/src/models/lesson.rs.template"),
    ),
    (
        "src/models/lesson_progress.rs",
        include_str!("lms/src/models/lesson_progress.rs.template"),
    ),
    (
        "src/models/lesson_progress_event.rs",
        include_str!("lms/src/models/lesson_progress_event.rs.template"),
    ),
    (
        "src/models/mod.rs",
        include_str!("lms/src/models/mod.rs.template"),
    ),
    (
        "src/models/user.rs",
        include_str!("lms/src/models/user.rs.template"),
    ),
    (
        "src/pages/auth.rs",
        include_str!("lms/src/pages/auth.rs.template"),
    ),
    (
        "src/pages/mod.rs",
        include_str!("lms/src/pages/mod.rs.template"),
    ),
    (
        "src/services/learning_service.rs",
        include_str!("lms/src/services/learning_service.rs.template"),
    ),
    (
        "src/services/mod.rs",
        include_str!("lms/src/services/mod.rs.template"),
    ),
    (
        "static/media/first-project.en.vtt",
        include_str!("lms/static/media/first-project.en.vtt"),
    ),
    (
        "static/media/memory-safety.en.vtt",
        include_str!("lms/static/media/memory-safety.en.vtt"),
    ),
];

/// Generates the LMS starter. Its Active Record models do not vary with the ORM
/// pattern, and hot reload exports the router from `src/lib.rs`. Files are
/// emitted in path order.
pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    _orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    if hot_reload {
        manifest.push((
            "src/lib.rs",
            include_str!("lms/src/lib.rs.template").to_string(),
        ));
        // `HOT_RELOAD` loads the rebuilt library; otherwise the binary links it.
        manifest.push((
            "src/main.rs",
            include_str!("lms/src/main.hot.rs.template")
                .replace("__PROJECT_NAME_SAFE__", project_name_safe),
        ));
    } else {
        manifest.push((
            "src/main.rs",
            include_str!("lms/src/main.rs.template").to_string(),
        ));
        manifest.push((
            super::security_tests::PATH,
            super::security_tests::source(super::security_tests::Starter::Lms),
        ));
    }
    manifest.extend(
        COMMON_FILES
            .iter()
            .map(|(path, template)| (*path, (*template).to_string())),
    );
    // Registration creates only the account: the starter has no schools or
    // memberships, and learners enroll in courses from the catalog.
    manifest.push((
        "src/controllers/auth_controller.rs",
        crate::generators::auth::controllers::render_auth_controller(None),
    ));
    // The catalog and course pages followed by the CSP-compatible lesson player.
    manifest.push((
        "src/pages/lms.rs",
        include_str!("lms/src/pages/lms.rs.template").replace(
            "__FE_IMPORTS__",
            &common::frontend_page_imports(frontend_engine),
        ),
    ));
    manifest.sort_unstable_by_key(|(path, _)| *path);
    manifest
}

#[cfg(test)]
#[path = "lms_tests.rs"]
mod tests;
