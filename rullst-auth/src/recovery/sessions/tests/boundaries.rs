//! Inventory size, stored creation times, lifetimes and atomic logout.
use super::*;

#[test]
fn identifiers_and_labels_debug_as_fixed_redactions() {
    let id = SessionId::new("A".repeat(43)).unwrap();
    assert_eq!(format!("{id:?}"), "SessionId([REDACTED])");
    let label = SessionLabel::new("My personal tablet").unwrap();
    assert_eq!(format!("{label:?}"), "SessionLabel([REDACTED])");
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn twenty_sessions_and_thirty_day_lifetimes_are_accepted() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "limits-alice").await;
    let current = store.create_session(&a, 1000, 2_592_000).await.unwrap();
    for _ in 1..20 {
        store.create_session(&a, 1000, 600).await.unwrap();
    }
    let sessions = store.active_sessions(current.expose(), 1001).await.unwrap();
    assert_eq!(sessions.len(), 20);
    assert_eq!(
        store.create_session(&a, 1000, 2_592_001).await.err(),
        Some(RecoveryError::InvalidInput)
    );
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn stored_creation_times_must_lie_between_unknown_and_expiry() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "times-alice").await;
    let current = store.create_session(&a, 1000, 600).await.unwrap();
    for (created, expected) in [(-1, Ok(None)), (1_600, Ok(Some(1_600)))] {
        sqlx::query("UPDATE rullst_recovery_session_details SET created_at = $1")
            .bind(created)
            .execute(&store.pool)
            .await
            .unwrap();
        let listed = store.active_sessions(current.expose(), 1001).await;
        assert_eq!(
            listed.map(|sessions| sessions[0].created_at()),
            expected,
            "{created}"
        );
    }
    for created in [-2_i64, 1_601] {
        sqlx::query("UPDATE rullst_recovery_session_details SET created_at = $1")
            .bind(created)
            .execute(&store.pool)
            .await
            .unwrap();
        assert_eq!(
            store.active_sessions(current.expose(), 1001).await,
            Err(RecoveryError::Storage),
            "{created}"
        );
    }
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn revoking_other_sessions_needs_both_version_updates() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "logout-alice").await;
    let current = store.create_session(&a, 1000, 600).await.unwrap();
    let sibling = store.create_session(&a, 1000, 600).await.unwrap();
    for (create, drop) in [
        (
            "CREATE TRIGGER skip_update BEFORE UPDATE ON rullst_recovery_accounts BEGIN SELECT RAISE(IGNORE); END",
            "DROP TRIGGER skip_update",
        ),
        (
            "CREATE TRIGGER skip_update BEFORE UPDATE ON rullst_recovery_sessions BEGIN SELECT RAISE(IGNORE); END",
            "DROP TRIGGER skip_update",
        ),
    ] {
        sqlx::query(create).execute(&store.pool).await.unwrap();
        assert_eq!(
            store.revoke_other_sessions(current.expose(), 1001).await,
            Err(RecoveryError::InvalidAction),
            "{create}"
        );
        sqlx::query(drop).execute(&store.pool).await.unwrap();
        assert!(
            store
                .verify_session(sibling.expose(), 1001)
                .await
                .unwrap()
                .is_some(),
            "the failed logout rolled back"
        );
    }
    assert_eq!(
        store.revoke_other_sessions(current.expose(), 1001).await,
        Ok(1)
    );
    store.close().await;
}
