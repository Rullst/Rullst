#![cfg(feature = "recovery-sqlite")]

use rullst_auth::recovery::{RecoveryError, RecoveryNoticeKind, RecoverySecrets, SqlRecoveryStore};
use std::collections::HashSet;

const EMAIL: &str = "sessions@example.com";
fn fixture_password() -> String {
    format!("Fixture_{:032x}", rand::random::<u128>())
}

async fn store() -> SqlRecoveryStore {
    let store = SqlRecoveryStore::connect(
        "sqlite::memory:",
        RecoverySecrets::new([3; 32], [9; 32]).unwrap(),
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    store
}

#[tokio::test]
async fn logout_capacity_expiry_and_instance_binding_are_authoritative() {
    let password = fixture_password();
    let store = store().await;
    store
        .register_account("member", EMAIL, password.as_str(), 1000)
        .await
        .unwrap();
    let account = store
        .authenticate(EMAIL, password.as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(account.subject(), "member");
    assert!(!format!("{account:?}").contains("member"));

    let foreign = SqlRecoveryStore::connect(
        "sqlite::memory:",
        RecoverySecrets::new([3; 32], [9; 32]).unwrap(),
    )
    .await
    .unwrap();
    foreign.migrate().await.unwrap();
    assert!(matches!(
        foreign.create_session(&account, 1000, 60).await,
        Err(RecoveryError::InvalidAction)
    ));
    foreign.close().await;
    for lifetime in [0, 30 * 86400 + 1] {
        assert!(matches!(
            store.create_session(&account, 1000, lifetime).await,
            Err(RecoveryError::InvalidInput)
        ));
    }
    assert_eq!(store.verify_session("malformed", 1000).await.unwrap(), None);

    let mut sessions = Vec::new();
    let mut unique = HashSet::new();
    for _ in 0..20 {
        let session = store.create_session(&account, 1000, 60).await.unwrap();
        assert!(unique.insert(session.expose().to_owned()));
        assert!(!format!("{session:?}").contains(session.expose()));
        sessions.push(session);
    }
    assert!(matches!(
        store.create_session(&account, 1000, 60).await,
        Err(RecoveryError::Limited)
    ));
    store.revoke_session(sessions[0].expose()).await.unwrap();
    store.revoke_session(sessions[0].expose()).await.unwrap();
    assert_eq!(
        store
            .verify_session(sessions[0].expose(), 1001)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .verify_session(sessions[1].expose(), 1059)
            .await
            .unwrap()
            .as_deref(),
        Some("member")
    );
    let replacement = store.create_session(&account, 1001, 60).await.unwrap();
    assert!(matches!(
        store.create_session(&account, 1001, 60).await,
        Err(RecoveryError::Limited)
    ));
    assert_eq!(
        store
            .verify_session(sessions[1].expose(), 1060)
            .await
            .unwrap(),
        None
    );
    // Expiry releases capacity without revoking a newer, still-live session.
    let next = store.create_session(&account, 1060, 60).await.unwrap();
    assert!(
        store
            .verify_session(replacement.expose(), 1060)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .verify_session(next.expose(), 1060)
            .await
            .unwrap()
            .is_some()
    );

    let clone = store.clone();
    store.close().await;
    assert_eq!(
        clone.verify_session(next.expose(), 1060).await,
        Err(RecoveryError::Storage)
    );
}

#[tokio::test]
async fn invalid_registration_cannot_create_accounts_or_welcome_deliveries() {
    let password = fixture_password();
    let store = store().await;
    for (subject, email, password) in [
        ("../member", EMAIL, password.as_str()),
        ("member", "invalid", password.as_str()),
        ("member", "a@@example.com", password.as_str()),
        ("member", "a@localhost", password.as_str()),
        ("member", "a\r\n@example.com", password.as_str()),
        ("member", "á@example.com", password.as_str()),
        ("member", EMAIL, &password[..5]),
    ] {
        assert_eq!(
            store.register_account(subject, email, password, 1000).await,
            Err(RecoveryError::InvalidInput)
        );
    }
    assert_eq!(
        store
            .register_account("member", EMAIL, password.repeat(40), 1000)
            .await,
        Err(RecoveryError::InvalidInput)
    );
    assert_eq!(
        store
            .register_account("member", EMAIL, password.as_str(), u64::MAX)
            .await,
        Err(RecoveryError::InvalidInput)
    );
    assert_eq!(store.outbox_snapshot().await.unwrap().pending, 0);
    assert!(
        store
            .authenticate(EMAIL, password.as_str())
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        store.authenticate(EMAIL, password.repeat(40)).await,
        Err(RecoveryError::InvalidInput)
    ));

    store
        .register_account("member", " Sessions@Example.com ", password.as_str(), 1000)
        .await
        .unwrap();
    assert!(
        store
            .authenticate(EMAIL, password.as_str())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .authenticate(EMAIL, format!("wrong-{password}"))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(store.outbox_snapshot().await.unwrap().pending, 1);
    assert!(
        store
            .register_account("duplicate", EMAIL, password.as_str(), 1001)
            .await
            .is_err()
    );
    assert_eq!(store.outbox_snapshot().await.unwrap().pending, 1);
    store.close().await;
}

#[tokio::test]
async fn invalid_storage_configuration_is_rejected_without_connecting() {
    for url in [
        "",
        "mysql://localhost/accounts",
        "https://example.com/accounts",
    ] {
        assert!(matches!(
            SqlRecoveryStore::connect(url, RecoverySecrets::new([3; 32], [9; 32]).unwrap()).await,
            Err(RecoveryError::Configuration)
        ));
    }
    for (digest, encryption) in [([0; 32], [9; 32]), ([3; 32], [0; 32]), ([3; 32], [3; 32])] {
        assert!(matches!(
            RecoverySecrets::new(digest, encryption),
            Err(RecoveryError::Configuration)
        ));
    }
}

#[tokio::test]
async fn password_reset_revokes_only_the_selected_accounts_sessions_and_handles() {
    let store = store().await;
    let old = fixture_password();
    let new = fixture_password();
    for (subject, email) in [("alice", "alice@example.com"), ("bob", "bob@example.com")] {
        store
            .register_account(subject, email, old.as_str(), 1000)
            .await
            .unwrap();
        let welcome = store.claim_notice(1001).await.unwrap().unwrap();
        assert_eq!(welcome.notice().kind(), RecoveryNoticeKind::Welcome);
        store.complete_notice(&welcome, 1002).await.unwrap();
    }
    let alice = store
        .authenticate("alice@example.com", old.as_str())
        .await
        .unwrap()
        .unwrap();
    let bob = store
        .authenticate("bob@example.com", old.as_str())
        .await
        .unwrap()
        .unwrap();
    let alice_session = store.create_session(&alice, 1003, 3600).await.unwrap();
    let bob_session = store.create_session(&bob, 1003, 3600).await.unwrap();
    store
        .request_password_reset("alice@example.com", 1004)
        .await
        .unwrap();
    let reset = store.claim_notice(1005).await.unwrap().unwrap();
    let token = reset.notice().token().unwrap().expose().to_owned();
    store.complete_notice(&reset, 1006).await.unwrap();
    store
        .complete_password_reset(&token, new.as_str(), 1007)
        .await
        .unwrap();

    assert_eq!(
        store
            .verify_session(alice_session.expose(), 1008)
            .await
            .unwrap(),
        None
    );
    assert!(matches!(
        store.create_session(&alice, 1008, 3600).await,
        Err(RecoveryError::InvalidAction)
    ));
    assert_eq!(
        store
            .verify_session(bob_session.expose(), 1008)
            .await
            .unwrap()
            .as_deref(),
        Some("bob")
    );
    let fresh_bob = store.create_session(&bob, 1008, 3600).await.unwrap();
    assert_eq!(
        store
            .verify_session(fresh_bob.expose(), 1009)
            .await
            .unwrap()
            .as_deref(),
        Some("bob")
    );
    assert!(
        store
            .authenticate("bob@example.com", old.as_str())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .authenticate("bob@example.com", new.as_str())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .authenticate("alice@example.com", new.as_str())
            .await
            .unwrap()
            .is_some()
    );
    let changed = store.claim_notice(1010).await.unwrap().unwrap();
    assert_eq!(changed.notice().kind(), RecoveryNoticeKind::PasswordChanged);
    store.complete_notice(&changed, 1011).await.unwrap();
    assert!(store.claim_notice(1012).await.unwrap().is_none());
    store.close().await;
}
