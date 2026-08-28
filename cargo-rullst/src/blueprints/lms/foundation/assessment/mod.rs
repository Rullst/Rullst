//! Detached `auth,learning,assessment` profile composition.

mod controller;
mod migration;
mod models;
mod routes;
mod service;

fn upsert(manifest: &mut Vec<(&'static str, String)>, path: &'static str, contents: String) {
    manifest.retain(|(existing, _)| *existing != path);
    manifest.push((path, contents));
}

pub(super) fn extend(manifest: &mut Vec<(&'static str, String)>) {
    for (path, contents) in models::files() {
        upsert(manifest, path, contents);
    }
    for (path, contents) in [
        ("src/main.rs", routes::main_source()),
        (
            "src/controllers/assessment_controller.rs",
            controller::ASSESSMENT_CONTROLLER.to_string(),
        ),
        (
            "src/controllers/mod.rs",
            "pub mod assessment_controller;\npub mod auth_controller;\npub mod learning_controller;\npub mod lms_controller;\n"
                .to_string(),
        ),
        (
            "src/migrations/m20260828000000_add_assessment.rs",
            migration::ASSESSMENT_MIGRATION.to_string(),
        ),
        (
            "src/migrations/mod.rs",
            migration::ASSESSMENT_MIGRATIONS_MODULE.to_string(),
        ),
        (
            "src/models/mod.rs",
            "pub mod category;\npub mod course;\npub mod course_module;\npub mod enrollment;\npub mod lesson;\npub mod lesson_progress;\npub mod lesson_progress_event;\npub mod quiz;\npub mod quiz_answer;\npub mod quiz_attempt;\npub mod quiz_option;\npub mod quiz_question;\npub mod user;\n"
                .to_string(),
        ),
        (
            "src/services/assessment_service.rs",
            service::ASSESSMENT_SERVICE.to_string(),
        ),
        (
            "src/services/mod.rs",
            "pub mod assessment_service;\npub mod learning_service;\n".to_string(),
        ),
        (
            "rullst-lms-modules.json",
            "{\n  \"schema_version\": 1,\n  \"modules\": [\"auth\", \"learning\", \"assessment\"],\n  \"profile\": \"assessment-foundation\"\n}\n"
                .to_string(),
        ),
    ] {
        upsert(manifest, path, contents);
    }
}

#[cfg(test)]
mod tests {
    use super::{migration, service};

    #[test]
    fn detached_service_keeps_grading_authoritative_without_vertical_coupling() {
        for required in [
            "correct option invariant",
            "option ownership",
            "AttemptLimit",
            "IdempotencyConflict",
            "FOR UPDATE",
        ] {
            assert!(service::ASSESSMENT_SERVICE.contains(required));
        }
        for excluded in [
            "leaderboard",
            "score_events",
            "academy_outbox",
            "notification",
            "automation",
        ] {
            assert!(!service::ASSESSMENT_SERVICE.contains(excluded));
            assert!(!migration::ASSESSMENT_MIGRATION.contains(excluded));
        }
    }
}
