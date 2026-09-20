use super::*;

#[tokio::test]
async fn a_fresh_process_observes_withdrawal_and_clock_high_water() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1200))
        .await
        .unwrap();
    store.close().await;
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sqlite::consent_child",
            "--ignored",
            "--nocapture",
        ])
        .env("RULLST_CONSENT_TEST_DATABASE", &path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
#[ignore = "fresh child process invoked by the owned consent lifecycle"]
async fn consent_child() {
    let path = std::env::var_os("RULLST_CONSENT_TEST_DATABASE").unwrap();
    let store = SqliteConsentStore::open(std::path::Path::new(&path), 8)
        .await
        .unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1100))
            .await,
        Err(ConsentError::ClockRollback)
    );
    let state = gate
        .current_with_clock(&subject(), &purpose(), &Clock::at(1200))
        .await
        .unwrap();
    assert_eq!(state.choice(), ConsentChoice::Withdrawn);
    assert_eq!(state.revision(), 1);
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &Clock::at(1200))
            .await
            .unwrap()
    );
    assert_eq!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            1300,
            &Clock::at(1200)
        )
        .await,
        Err(ConsentError::RevisionConflict)
    );
    store.close().await;
}

#[tokio::test]
async fn initialization_is_explicit_and_normal_open_never_repairs_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    assert!(SqliteConsentStore::open(&path, 8).await.is_err());
    assert!(!path.exists());
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    assert!(SqliteConsentStore::initialize(&path, 8).await.is_err());
    assert!(SqliteConsentStore::open(&path, 7).await.is_err());
    store.close().await;
    let raw =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    sqlx::query("DROP TABLE rullst_consent_meta")
        .execute(&raw)
        .await
        .unwrap();
    assert!(SqliteConsentStore::open(&path, 8).await.is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema WHERE name='rullst_consent_meta'")
            .fetch_one(&raw)
            .await
            .unwrap();
    assert_eq!(count, 0);
    raw.close().await;
}

#[tokio::test]
async fn independent_pools_serialize_revisions_and_restart_preserves_withdrawal() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let first = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    let second = SqliteConsentStore::open(&path, 8).await.unwrap();
    let one = ConsentGate::new(first.clone()).unwrap();
    let two = ConsentGate::new(second.clone()).unwrap();
    let clock = Clock::at(1000);
    let response = submission(0, ConsentChoice::Granted);
    let who = subject();
    let why = purpose();
    let (left, right) = tokio::join!(
        one.choose_with_clock(&who, &why, &response, 1100, &clock),
        two.choose_with_clock(&who, &why, &response, 1100, &clock)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    assert!(matches!(
        left.err().or(right.err()),
        Some(ConsentError::RevisionConflict)
    ));
    one.withdraw_with_clock(&who, &why, &clock).await.unwrap();
    assert!(matches!(
        two.choose_with_clock(
            &who,
            &why,
            &submission(1, ConsentChoice::Granted),
            1100,
            &clock
        )
        .await,
        Err(ConsentError::RevisionConflict)
    ));
    first.close().await;
    second.close().await;
    let reopened = SqliteConsentStore::open(&path, 8).await.unwrap();
    let gate = ConsentGate::new(reopened.clone()).unwrap();
    let current = gate.current_with_clock(&who, &why, &clock).await.unwrap();
    assert_eq!(current.choice(), ConsentChoice::Withdrawn);
    assert_eq!(current.revision(), 2);
    assert!(!gate.allows_with_clock(&who, &why, &clock).await.unwrap());
    reopened.close().await;
}

#[tokio::test]
async fn expired_records_and_tombstones_keep_quota_and_clock_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 1).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &Clock::at(1000),
    )
    .await
    .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &Clock::at(1100))
            .await
            .unwrap()
    );
    store.close().await;
    let reopened = SqliteConsentStore::open(&path, 1).await.unwrap();
    let gate = ConsentGate::new(reopened.clone()).unwrap();
    assert!(matches!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1099))
            .await,
        Err(ConsentError::ClockRollback)
    ));
    let other = ConsentSubject::new("user-2", "school-1").unwrap();
    assert!(matches!(
        gate.withdraw_with_clock(&other, &purpose(), &Clock::at(1200))
            .await,
        Err(ConsentError::StoreCapacity)
    ));
    let current = gate
        .withdraw_with_clock(&subject(), &purpose(), &Clock::at(1200))
        .await
        .unwrap();
    assert_eq!(current.revision(), 2);
    reopened.close().await;
    assert!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1200))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn scope_digests_isolate_accounts_tenants_and_purposes_without_raw_ids() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    let clock = Clock::at(1000);
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &clock,
    )
    .await
    .unwrap();
    for other in [
        ConsentSubject::new("user-2", "school-1").unwrap(),
        ConsentSubject::new("user-1", "school-2").unwrap(),
    ] {
        assert!(
            !gate
                .allows_with_clock(&other, &purpose(), &clock)
                .await
                .unwrap()
        );
    }
    assert!(
        !gate
            .allows_with_clock(
                &subject(),
                &ConsentPurpose::new("other", "notice-v1").unwrap(),
                &clock
            )
            .await
            .unwrap()
    );
    assert!(
        !gate
            .allows_with_clock(
                &subject(),
                &ConsentPurpose::new("optional-digest", "notice-v2").unwrap(),
                &clock
            )
            .await
            .unwrap()
    );
    store.close().await;
    let raw =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    let rows: Vec<(Vec<u8>, String)> =
        sqlx::query_as("SELECT scope_digest,policy_version FROM rullst_consent_choices")
            .fetch_all(&raw)
            .await
            .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.len(), 32);
    assert_eq!(rows[0].1, "notice-v1");
    raw.close().await;
    let bytes = std::fs::read(&path).unwrap();
    for forbidden in [b"user-1".as_slice(), b"school-1", b"optional-digest"] {
        assert!(
            !bytes
                .windows(forbidden.len())
                .any(|window| window == forbidden)
        );
    }
}

#[tokio::test]
async fn cancellation_while_waiting_for_storage_never_acknowledges_a_choice() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    let raw =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    let lock = raw.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            1100,
            &Clock::at(1000),
        ),
    )
    .await;
    assert!(result.is_err());
    lock.rollback().await.unwrap();
    let current = gate
        .current_with_clock(&subject(), &purpose(), &Clock::at(1000))
        .await
        .unwrap();
    assert_eq!(current.choice(), ConsentChoice::Unset);
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await
            .unwrap()
    );
    raw.close().await;
    store.close().await;
}

#[tokio::test]
async fn corrupted_records_and_exhausted_revisions_cannot_allow_or_regrant() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    let gate = ConsentGate::new(store.clone()).unwrap();
    let clock = Clock::at(1000);
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &clock,
    )
    .await
    .unwrap();
    let raw =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    sqlx::query("UPDATE rullst_consent_choices SET policy_version = ?")
        .bind("invalid/version")
        .execute(&raw)
        .await
        .unwrap();
    assert!(
        gate.allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .is_err()
    );
    sqlx::query("UPDATE rullst_consent_choices SET policy_version = ?, revision = ?")
        .bind("notice-v1")
        .bind(i64::MAX)
        .execute(&raw)
        .await
        .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    assert!(matches!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(i64::MAX as u64, ConsentChoice::Granted),
            1100,
            &clock
        )
        .await,
        Err(ConsentError::RevisionExhausted)
    ));
    raw.close().await;
    store.close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn bootstrap_sets_private_permissions_and_refuses_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("choices.db");
    let store = SqliteConsentStore::initialize(&path, 8).await.unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
        0
    );
    let link = temp.path().join("link.db");
    symlink(&path, &link).unwrap();
    assert!(SqliteConsentStore::open(&link, 8).await.is_err());
    assert!(SqliteConsentStore::initialize(&link, 8).await.is_err());
    assert!(SqliteConsentStore::open(temp.path(), 8).await.is_err());
    assert!(SqliteConsentStore::initialize(":memory:", 8).await.is_err());
    store.close().await;
}
