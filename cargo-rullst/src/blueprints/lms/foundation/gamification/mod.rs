//! Detached `auth,learning,gamification` profile composition.

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
            "src/controllers/gamification_controller.rs",
            controller::GAMIFICATION_CONTROLLER.to_string(),
        ),
        (
            "src/controllers/mod.rs",
            "pub mod auth_controller;\npub mod gamification_controller;\npub mod learning_controller;\npub mod lms_controller;\n"
                .to_string(),
        ),
        (
            "src/migrations/m20260828000000_add_gamification.rs",
            migration::GAMIFICATION_MIGRATION.to_string(),
        ),
        (
            "src/migrations/mod.rs",
            migration::GAMIFICATION_MIGRATIONS_MODULE.to_string(),
        ),
        (
            "src/models/mod.rs",
            "pub mod activity;\npub mod category;\npub mod course;\npub mod course_module;\npub mod enrollment;\npub mod leaderboard_entry;\npub mod lesson;\npub mod lesson_progress;\npub mod lesson_progress_event;\npub mod score_event;\npub mod user;\n"
                .to_string(),
        ),
        (
            "src/services/gamification_service.rs",
            service::GAMIFICATION_SERVICE.to_string(),
        ),
        (
            "src/services/mod.rs",
            "pub mod gamification_service;\npub mod learning_service;\n".to_string(),
        ),
        (
            "rullst-lms-modules.json",
            "{\n  \"schema_version\": 1,\n  \"modules\": [\"auth\", \"learning\", \"gamification\"],\n  \"profile\": \"gamification-foundation\"\n}\n"
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
    fn detached_scoring_is_server_owned_and_excludes_event_verticals() {
        for required in [
            "TrustedActivityResult",
            "evidence digest",
            "IdempotencyConflict",
            "AttemptLimit",
            "FOR UPDATE",
            "INSERT INTO leaderboard_entries",
        ] {
            assert!(service::GAMIFICATION_SERVICE.contains(required));
        }
        assert!(
            !service::GAMIFICATION_SERVICE
                .contains("derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)")
        );
        for excluded in [
            "academy_outbox",
            "automation",
            "notification",
            "achievement",
        ] {
            assert!(!service::GAMIFICATION_SERVICE.contains(excluded));
            assert!(!migration::GAMIFICATION_MIGRATION.contains(excluded));
        }
    }
}
