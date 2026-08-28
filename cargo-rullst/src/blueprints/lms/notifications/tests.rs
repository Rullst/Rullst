use super::{NOTIFICATION_MIGRATION, NOTIFICATION_SERVICE};

#[test]
fn notification_contract_is_claim_bound_idempotent_and_owner_readable() {
    assert!(NOTIFICATION_MIGRATION.contains("notifications_key_unique"));
    assert!(
        NOTIFICATION_SERVICE
            .contains("ao.event_kind = $2 AND ao.status = $3 AND ao.claim_key = $4")
    );
    assert!(NOTIFICATION_SERVICE.contains("academy.achievement.awarded"));
    assert!(NOTIFICATION_SERVICE.contains("authorize_owner_or_role"));
    assert!(NOTIFICATION_SERVICE.contains("pub async fn list_notifications"));
    assert!(NOTIFICATION_SERVICE.contains("pub async fn set_preference"));
    assert!(NOTIFICATION_SERVICE.contains("pub async fn subscribe_in_app"));
    assert!(NOTIFICATION_SERVICE.contains("TenantRealtime::from_context"));
    assert!(NOTIFICATION_SERVICE.contains("realtime_enabled == 1"));
}
