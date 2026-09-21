use super::*;
use crate::recovery::RecoverySecrets;

#[cfg(feature = "recovery-postgres")]
#[path = "postgres_tests.rs"]
mod postgres;

fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

async fn account(store: &SqlRecoveryStore, subject: &str) -> AuthenticatedRecoveryAccount {
    let email = format!("{subject}@example.com");
    let password = format!("Fixture_{:032x}", rand::random::<u128>());
    store
        .register_account(subject, &email, &password, 1000)
        .await
        .unwrap();
    store.authenticate(email, password).await.unwrap().unwrap()
}

async fn connect(url: &str) -> SqlRecoveryStore {
    let store = SqlRecoveryStore::connect(url, keys()).await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn details(store: &SqlRecoveryStore) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rullst_recovery_session_details")
        .fetch_one(&store.pool)
        .await
        .unwrap()
}

async fn inventory_contract(url: &str) {
    let store = connect(url).await;
    let a = account(&store, "inventory-alice").await;
    let b = account(&store, "inventory-bob").await;
    let current = store
        .create_session_with_label(&a, 1000, 600, SessionLabel::new("My laptop").unwrap())
        .await
        .unwrap();
    let other = store
        .create_session_with_label(&a, 1001, 120, SessionLabel::new("My phone").unwrap())
        .await
        .unwrap();
    let b_token = store
        .create_session_with_label(&b, 1002, 300, SessionLabel::new("Private label").unwrap())
        .await
        .unwrap();
    let list = store.active_sessions(current.expose(), 1003).await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list.iter().filter(|s| s.is_current()).count(), 1);
    assert_eq!(list[0].created_at(), Some(1001));
    assert_eq!(list[0].expires_at(), 1121);
    assert_eq!(list[0].label().unwrap().as_str(), "My phone");
    assert_eq!(list[1].label().unwrap().as_str(), "My laptop");
    assert!(!format!("{list:?}").contains("My phone"));
    for session in &list {
        let id = session.id();
        assert_eq!(SessionId::new(id.as_str()).unwrap(), *id);
        assert_ne!(id.as_str(), current.expose());
        assert_ne!(id.as_str(), other.expose());
        assert_eq!(store.verify_session(id.as_str(), 1003).await.unwrap(), None);
    }
    let foreign = store.active_sessions(b_token.expose(), 1003).await.unwrap();
    for target in [foreign[0].id(), list[1].id()] {
        assert_eq!(
            store
                .revoke_other_session(current.expose(), target, 1003)
                .await,
            Err(RecoveryError::InvalidAction)
        );
    }
    assert_eq!(
        store
            .revoke_other_session(b_token.expose(), list[0].id(), 1003)
            .await,
        Err(RecoveryError::InvalidAction)
    );
    store
        .revoke_other_session(current.expose(), list[0].id(), 1003)
        .await
        .unwrap();
    assert_eq!(
        store.verify_session(other.expose(), 1003).await.unwrap(),
        None
    );
    assert_eq!(
        store
            .revoke_other_session(current.expose(), list[0].id(), 1003)
            .await,
        Err(RecoveryError::InvalidAction)
    );
    assert!(
        store
            .verify_session(current.expose(), 1003)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .verify_session(b_token.expose(), 1003)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(details(&store).await, 2);

    let sibling = store.create_session(&a, 1003, 300).await.unwrap();
    let expired = store.create_session(&a, 1003, 1).await.unwrap();
    assert_eq!(
        store
            .active_sessions(current.expose(), 1004)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        store
            .revoke_other_sessions(current.expose(), 1004)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .revoke_other_sessions(current.expose(), 1004)
            .await
            .unwrap(),
        0
    );
    assert_eq!(details(&store).await, 2);
    for token in [sibling.expose(), expired.expose()] {
        assert_eq!(store.verify_session(token, 1004).await.unwrap(), None);
        assert_eq!(
            store.active_sessions(token, 1004).await,
            Err(RecoveryError::InvalidAction)
        );
        assert_eq!(
            store.revoke_other_sessions(token, 1004).await,
            Err(RecoveryError::InvalidAction)
        );
    }
    assert!(
        store
            .verify_session(current.expose(), 1004)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .verify_session(b_token.expose(), 1004)
            .await
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        store.create_session(&a, 1004, 60).await,
        Err(RecoveryError::InvalidAction)
    ));
    store.revoke_session(current.expose()).await.unwrap();
    store.revoke_session(current.expose()).await.unwrap();
    assert_eq!(details(&store).await, 1);
    assert_eq!(
        store.active_sessions(current.expose(), 1004).await,
        Err(RecoveryError::InvalidAction)
    );
    assert_eq!(
        store.active_sessions(b_token.expose(), 1302).await,
        Err(RecoveryError::InvalidAction)
    );
    assert_eq!(
        store.active_sessions(b_token.expose(), u64::MAX).await,
        Err(RecoveryError::InvalidInput)
    );
    let closed = store.clone();
    store.close().await;
    assert_eq!(
        closed.active_sessions(b_token.expose(), 1004).await,
        Err(RecoveryError::Storage)
    );
    assert_eq!(
        closed.revoke_other_sessions(b_token.expose(), 1004).await,
        Err(RecoveryError::Storage)
    );
    assert_eq!(
        closed
            .revoke_other_session(b_token.expose(), list[0].id(), 1004)
            .await,
        Err(RecoveryError::Storage)
    );
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn sqlite_inventory_and_revocation_are_account_scoped() {
    inventory_contract("sqlite::memory:").await;
}

#[cfg(feature = "recovery-postgres")]
#[tokio::test]
#[ignore = "requires the owned PostgreSQL fixture"]
async fn postgres_inventory_and_revocation_are_account_scoped() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    inventory_contract(&url).await;
}

#[test]
fn identifiers_and_labels_are_bounded_and_redacted() {
    for label in ["", " a", "a ", "a\nb", "\u{2028}", "\u{0085}"] {
        assert_eq!(SessionLabel::new(label), Err(RecoveryError::InvalidInput));
    }
    assert!(SessionLabel::new("a".repeat(80)).is_ok());
    assert!(SessionLabel::new("a".repeat(81)).is_err());
    assert!(SessionLabel::new("é".repeat(41)).is_err());
    let label = SessionLabel::new("My personal tablet").unwrap();
    assert!(!format!("{label:?}").contains("personal"));
    for id in [
        "",
        "x",
        &"x".repeat(44),
        &"!".repeat(43),
        &"/".repeat(43),
        &"x".repeat(43),
    ] {
        assert!(SessionId::new(id).is_err());
    }
    let id = SessionId::new("A".repeat(43)).unwrap();
    assert!(!format!("{id:?}").contains(id.as_str()));
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn legacy_metadata_expiry_and_recovery_keep_their_atomic_boundaries() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "metadata-alice").await;
    let legacy = store.create_session(&a, 1000, 10).await.unwrap();
    // Simulate a pre-v13 session row: additive migration must not fabricate history.
    sqlx::query("DROP TABLE rullst_recovery_session_details")
        .execute(&store.pool)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let list = store.active_sessions(legacy.expose(), 1001).await.unwrap();
    assert_eq!(list[0].created_at(), None);
    assert_eq!(list[0].label(), None);
    assert!(
        store
            .verify_session(legacy.expose(), 1001)
            .await
            .unwrap()
            .is_some()
    );
    let newer = store
        .create_session_with_label(&a, 1001, 2, SessionLabel::new("temporary").unwrap())
        .await
        .unwrap();
    assert_eq!(details(&store).await, 1);
    let next = store.create_session(&a, 1003, 60).await.unwrap();
    assert_eq!(details(&store).await, 1);
    assert_eq!(
        store.verify_session(newer.expose(), 1003).await.unwrap(),
        None
    );
    // Consume welcome before retrieving the reset notice.
    let welcome = store.claim_notice(1003).await.unwrap().unwrap();
    store.complete_notice(&welcome, 1003).await.unwrap();
    store
        .request_password_reset("metadata-alice@example.com", 1004)
        .await
        .unwrap();
    let notice = store.claim_notice(1004).await.unwrap().unwrap();
    let replacement = format!("Replacement_{:032x}", rand::random::<u128>());
    store
        .complete_password_reset(
            notice.notice().token().unwrap().expose(),
            &replacement,
            1005,
        )
        .await
        .unwrap();
    assert_eq!(details(&store).await, 0);
    assert_eq!(
        store.verify_session(next.expose(), 1005).await.unwrap(),
        None
    );
    assert_eq!(
        store.active_sessions(legacy.expose(), 1005).await,
        Err(RecoveryError::InvalidAction)
    );
    let fresh = store
        .authenticate("metadata-alice@example.com", replacement)
        .await
        .unwrap()
        .unwrap();
    store.create_session(&fresh, 1005, 60).await.unwrap();
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn rollback_preserves_sessions_and_invalid_metadata_fails_closed() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "rollback-alice").await;
    let current = store.create_session(&a, 1000, 600).await.unwrap();
    let sibling = store.create_session(&a, 1000, 600).await.unwrap();
    let target = store
        .active_sessions(current.expose(), 1001)
        .await
        .unwrap()
        .into_iter()
        .find(|s| !s.is_current())
        .unwrap();
    sqlx::query("CREATE TRIGGER refuse_session_delete BEFORE DELETE ON rullst_recovery_sessions BEGIN SELECT RAISE(ABORT, 'fixture'); END").execute(&store.pool).await.unwrap();
    assert_eq!(
        store
            .revoke_other_session(current.expose(), target.id(), 1001)
            .await,
        Err(RecoveryError::Storage)
    );
    assert_eq!(
        store.revoke_other_sessions(current.expose(), 1001).await,
        Err(RecoveryError::Storage)
    );
    assert_eq!(details(&store).await, 2);
    assert!(
        store
            .verify_session(sibling.expose(), 1001)
            .await
            .unwrap()
            .is_some()
    );
    store.create_session(&a, 1001, 60).await.unwrap();
    sqlx::query("DROP TRIGGER refuse_session_delete")
        .execute(&store.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rullst_recovery_session_details SET label = $1")
        .bind("x".repeat(81))
        .execute(&store.pool)
        .await
        .unwrap();
    assert_eq!(
        store.active_sessions(current.expose(), 1001).await,
        Err(RecoveryError::Storage)
    );
    // Corrupt inventory cannot disable the existing bearer-token logout path.
    store.revoke_session(current.expose()).await.unwrap();
    assert_eq!(details(&store).await, 2);
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn simultaneous_sibling_logouts_have_only_one_survivor() {
    let store = connect("sqlite::memory:").await;
    let a = account(&store, "race-alice").await;
    let one = store.create_session(&a, 1000, 600).await.unwrap();
    let two = store.create_session(&a, 1000, 600).await.unwrap();
    let (first, second) = tokio::join!(
        store.revoke_other_sessions(one.expose(), 1001),
        store.revoke_other_sessions(two.expose(), 1001)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(matches!(first, Ok(1) | Err(RecoveryError::InvalidAction)));
    assert!(matches!(second, Ok(1) | Err(RecoveryError::InvalidAction)));
    assert_eq!(details(&store).await, 1);
    assert_eq!(
        usize::from(
            store
                .verify_session(one.expose(), 1001)
                .await
                .unwrap()
                .is_some()
        ) + usize::from(
            store
                .verify_session(two.expose(), 1001)
                .await
                .unwrap()
                .is_some()
        ),
        1
    );
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn retention_is_bounded_and_foreign_store_tokens_never_authenticate() {
    let store = connect("sqlite::memory:").await;
    let account = account(&store, "retention").await;
    let a = store.create_session(&account, 1000, 1).await.unwrap();
    let b = store.create_session(&account, 1000, 2).await.unwrap();
    let current = store.create_session(&account, 1000, 60).await.unwrap();
    for limit in [0, 101, u32::MAX] {
        assert_eq!(
            store.purge_expired_sessions(1002, limit).await,
            Err(RecoveryError::InvalidInput)
        );
    }
    assert_eq!(
        store.purge_expired_sessions(u64::MAX, 1).await,
        Err(RecoveryError::InvalidInput)
    );
    assert_eq!(store.purge_expired_sessions(1002, 1).await.unwrap(), 1);
    assert_eq!(details(&store).await, 2);
    assert_eq!(store.purge_expired_sessions(1002, 100).await.unwrap(), 1);
    assert_eq!(store.purge_expired_sessions(1002, 100).await.unwrap(), 0);
    assert_eq!(details(&store).await, 1);
    for token in [a.expose(), b.expose()] {
        assert_eq!(store.verify_session(token, 1000).await.unwrap(), None);
    }
    let other = connect("sqlite::memory:").await;
    assert_eq!(
        other.active_sessions(current.expose(), 1002).await,
        Err(RecoveryError::InvalidAction)
    );
    assert_eq!(
        other.revoke_other_sessions(current.expose(), 1002).await,
        Err(RecoveryError::InvalidAction)
    );
    assert!(
        store
            .verify_session(current.expose(), 1002)
            .await
            .unwrap()
            .is_some()
    );
    other.close().await;
    store.close().await;
}
