//! Outbox claims, back-off, capacity eviction, snapshots and store inputs.
use super::*;
use crate::recovery::RecoverySecrets;

const T: u64 = 1_800_000_000;

async fn store() -> SqlRecoveryStore {
    let keys = RecoverySecrets::new([3; 32], [9; 32]).unwrap();
    let store = SqlRecoveryStore::connect("sqlite::memory:", keys)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn claims_keep_their_delivery_id_and_back_off_after_a_transient_failure() {
    let store = store().await;
    store
        .register_account("learner-1", "learner@example.com", "Fixture_password_1", T)
        .await
        .unwrap();
    let first = store.claim_notice(T).await.unwrap().unwrap();
    let id = first.delivery_id().to_owned();
    assert_eq!(id.len(), 43);
    assert_eq!(first.notice().kind(), RecoveryNoticeKind::Welcome);
    assert_eq!(first.notice().expires_at(), (T + 86_400) as i64);
    assert_eq!(
        format!("{first:?}"),
        format!(
            "ClaimedRecoveryNotice {{ id: {id:?}, notice: RecoveryNotice {{ kind: Welcome, .. }}, .. }}"
        )
    );
    let snapshot = store.outbox_snapshot().await.unwrap();
    assert_eq!((snapshot.pending, snapshot.leased), (0, 1));
    store
        .fail_notice(&first, RecoveryDeliveryFailure::Transient, T)
        .await
        .unwrap();
    // The first retry waits 15 * 2 seconds.
    assert!(store.claim_notice(T + 29).await.unwrap().is_none());
    let retry = store.claim_notice(T + 30).await.unwrap().unwrap();
    assert_eq!(retry.delivery_id(), id);
    store.close().await;
}

#[tokio::test]
async fn a_full_outbox_evicts_exactly_one_tombstone() {
    let store = store().await;
    sqlx::query(
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 10000) \
         INSERT INTO rullst_recovery_outbox (id, subject, kind, ciphertext, expires_at, status, \
         attempts, due_at, lease, lease_until) SELECT 'tomb-' || i, 'nobody', 'welcome', '', \
         1900000000 + i, 'delivered', 1, 0, '', 0 FROM n",
    )
    .execute(&store.pool)
    .await
    .unwrap();
    store
        .register_account("learner-1", "learner@example.com", "Fixture_password_1", T)
        .await
        .unwrap();
    let snapshot = store.outbox_snapshot().await.unwrap();
    assert_eq!((snapshot.delivered, snapshot.pending), (9_999, 1));
    store.close().await;
}

#[tokio::test]
async fn passwords_need_twelve_characters_and_proofs_debug_redacted() {
    let store = store().await;
    assert_eq!(
        store
            .register_account("short-1", "short@example.com", "Abcdefghij1", T)
            .await,
        Err(RecoveryError::InvalidInput)
    );
    store
        .register_account("learner-1", "learner@example.com", "Abcdefghijk1", T)
        .await
        .unwrap();
    let proof = store
        .authenticate("learner@example.com", "Abcdefghijk1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        format!("{proof:?}"),
        "AuthenticatedRecoveryAccount([REDACTED])"
    );
    store.close().await;
}

/// Both PostgreSQL schemes are classified as PostgreSQL: with nothing
/// listening, the connection fails as storage, never as configuration.
#[cfg(feature = "recovery-postgres")]
#[tokio::test]
async fn both_postgresql_schemes_reach_the_postgresql_driver() {
    let keys = || RecoverySecrets::new([3; 32], [9; 32]).unwrap();
    let (short, long) = tokio::join!(
        SqlRecoveryStore::connect("postgres://127.0.0.1:1/recovery", keys()),
        SqlRecoveryStore::connect("postgresql://127.0.0.1:1/recovery", keys())
    );
    assert_eq!(short.err(), Some(RecoveryError::Storage));
    assert_eq!(long.err(), Some(RecoveryError::Storage));
    assert_eq!(
        SqlRecoveryStore::connect("mysql://127.0.0.1:1/recovery", keys())
            .await
            .err(),
        Some(RecoveryError::Configuration)
    );
}
