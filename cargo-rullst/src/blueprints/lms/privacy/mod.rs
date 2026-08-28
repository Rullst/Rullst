mod executor;
mod migration;
mod retention;
mod service;
mod worker;

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/migrations/m20260906000000_add_privacy_lifecycle.rs",
            migration::PRIVACY_MIGRATION.to_string(),
        ),
        (
            "src/services/privacy_service.rs",
            service::PRIVACY_SERVICE.to_string(),
        ),
        (
            "src/services/privacy_retention_service.rs",
            retention::PRIVACY_RETENTION_SERVICE.to_string(),
        ),
        (
            "src/services/privacy_request_worker_service.rs",
            worker::PRIVACY_REQUEST_WORKER_SERVICE.to_string(),
        ),
        (
            "src/services/privacy_request_executor_service.rs",
            executor::PRIVACY_REQUEST_EXECUTOR_SERVICE.to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        executor::PRIVACY_REQUEST_EXECUTOR_SERVICE, migration::PRIVACY_MIGRATION,
        retention::PRIVACY_RETENTION_SERVICE, service::PRIVACY_SERVICE,
        worker::PRIVACY_REQUEST_WORKER_SERVICE,
    };

    #[test]
    fn generated_privacy_foundation_is_minimized_scoped_and_lifecycle_bounded() {
        for required in [
            "privacy_subject_policies",
            "guardian_consents",
            "privacy_requests",
            "school_id",
            "retention_until_epoch",
            "policy_version",
        ] {
            assert!(PRIVACY_MIGRATION.contains(required));
        }
        assert!(!PRIVACY_MIGRATION.contains("birth_date"));
        assert!(!PRIVACY_MIGRATION.contains("date_of_birth"));
        for required in [
            "configure_subject_policy_at",
            "record_guardian_consent_at",
            "authorize_subject_at",
            "request_privacy_action_at",
            "revoke_guardian_consent_at",
        ] {
            assert!(PRIVACY_SERVICE.contains(required));
        }
        assert!(PRIVACY_RETENTION_SERVICE.contains("pub async fn schedule_expired_at"));
        assert!(PRIVACY_RETENTION_SERVICE.contains("retention_due"));
        assert!(PRIVACY_RETENTION_SERVICE.contains("ON CONFLICT DO NOTHING"));
        assert!(PRIVACY_REQUEST_WORKER_SERVICE.contains("pub async fn claim_next_at"));
        assert!(PRIVACY_REQUEST_WORKER_SERVICE.contains("pub async fn complete_at"));
        assert!(PRIVACY_REQUEST_EXECUTOR_SERVICE.contains("pub fn start<A:"));
    }
}
