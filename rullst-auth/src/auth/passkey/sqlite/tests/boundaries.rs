//! Field limits, device metadata, counter rules and database-target checks.
use super::*;
use PasskeyStoreError::{CorruptStorage, InvalidConfiguration, InvalidCredential};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs()
}

fn registered_fixture(
    auth: &PasskeyAuth,
) -> (
    crate::auth::passkey::test_support::RegistrationFixture,
    Passkey,
) {
    let fixture = registration_fixture(auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .expect("finish registration");
    (fixture, passkey)
}

#[tokio::test]
async fn device_summaries_report_identity_times_and_revocation() {
    let path = temporary_database("summary");
    let auth = auth();
    let (fixture, passkey) = registered_fixture(&auth);
    let store = SqlitePasskeyStore::connect(database_url(&path), 32, 8)
        .await
        .expect("connect store");
    let before = now_secs();
    let registered = store
        .register("user-7", "Laptop", passkey.clone())
        .await
        .expect("register credential");
    assert_eq!(registered.credential_id(), &[10, 20, 30, 40]);
    assert!((before..=now_secs()).contains(&registered.created_at()));
    assert_eq!(registered.last_used_at(), None);
    assert_eq!(registered.revoked_at(), None);
    assert!(!registered.is_revoked());
    let listed = store.devices("user-7").await.expect("list devices");
    assert_eq!(listed[0].credential_id(), &[10, 20, 30, 40]);
    assert!((before..=now_secs()).contains(&listed[0].created_at()));
    assert!(!listed[0].is_revoked());

    let (assertion, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x05, 2);
    store
        .finish_authenticate(&auth, "user-7", &assertion, &challenge)
        .await
        .expect("verify assertion");
    let used = &store.devices("user-7").await.expect("list devices")[0];
    let used_at = used.last_used_at().expect("usage time is recorded");
    assert!((before..=now_secs()).contains(&used_at));
    assert_eq!(used.revoked_at(), None);

    store
        .revoke("user-7", &passkey.credential_id)
        .await
        .expect("revoke device");
    let revoked = &store.devices("user-7").await.expect("list devices")[0];
    let revoked_at = revoked.revoked_at().expect("revocation time is recorded");
    assert!((before..=now_secs()).contains(&revoked_at));
    assert!(revoked.is_revoked());

    // Closing one handle closes the pool its clones share.
    let other = store.clone();
    store.close().await;
    assert_eq!(
        other.devices("user-7").await,
        Err(PasskeyStoreError::StorageUnavailable("list devices"))
    );
    remove_database(&path);
}

#[tokio::test]
async fn counter_updates_keep_the_credential_and_advance_unless_both_are_zero() {
    let path = temporary_database("counter-rules");
    let auth = auth();
    let (_, mut passkey) = registered_fixture(&auth);
    passkey.sign_count = 2;
    let (_, rekeyed) = registered_fixture(&auth);
    let mut zero = passkey.clone();
    zero.credential_id = vec![50, 60, 70, 80];
    zero.sign_count = 0;
    let store = SqlitePasskeyStore::connect(database_url(&path), 32, 8)
        .await
        .expect("connect store");
    for (label, device) in [("Key", &passkey), ("Zero", &zero)] {
        store
            .register("user-7", label, device.clone())
            .await
            .expect("register credential");
    }
    let mut other_id = passkey.clone();
    other_id.credential_id = vec![1, 2, 3];
    other_id.sign_count = 3;
    let mut other_key = passkey.clone();
    other_key.public_key = rekeyed.public_key;
    other_key.sign_count = 3;
    let mut reset = passkey.clone();
    reset.sign_count = 0;
    for updated in [other_id, other_key, passkey.clone(), reset] {
        assert_eq!(
            store.advance_counter("user-7", &passkey, &updated).await,
            Err(InvalidCredential)
        );
    }
    let devices = store.devices("user-7").await.expect("list devices");
    assert_eq!(devices[0].sign_count(), 2);
    assert_eq!(devices[0].last_used_at(), None);
    // Authenticators without a counter keep reporting zero.
    store
        .advance_counter("user-7", &zero, &zero)
        .await
        .expect("zero counters may repeat");
    let devices = store.devices("user-7").await.expect("list devices");
    assert_eq!(devices[1].sign_count(), 0);
    assert!(devices[1].last_used_at().is_some());
    store.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn malformed_credentials_are_neither_stored_nor_loaded() {
    let path = temporary_database("malformed");
    let auth = auth();
    let (_, passkey) = registered_fixture(&auth);
    let store = SqlitePasskeyStore::connect(database_url(&path), 32, 8)
        .await
        .expect("connect store");
    let mut off_curve = vec![4];
    off_curve.extend_from_slice(&[1; 64]);
    let mut wrong_tag = passkey.public_key.clone();
    wrong_tag[0] = 3;
    let malformed = [
        Passkey {
            credential_id: Vec::new(),
            ..passkey.clone()
        },
        Passkey {
            credential_id: vec![1; 1_024],
            ..passkey.clone()
        },
        Passkey {
            public_key: passkey.public_key[..33].to_vec(),
            ..passkey.clone()
        },
        Passkey {
            public_key: wrong_tag,
            ..passkey.clone()
        },
        Passkey {
            public_key: off_curve.clone(),
            ..passkey.clone()
        },
    ];
    for candidate in malformed {
        assert_eq!(
            store.register("user-7", "Laptop", candidate).await,
            Err(InvalidCredential)
        );
    }
    assert!(store.devices("user-7").await.expect("list").is_empty());
    // A stored key corrupted outside the store is not handed to WebAuthn.
    store
        .register("user-7", "Laptop", passkey)
        .await
        .expect("register credential");
    sqlx::query("UPDATE rullst_auth_passkey_devices SET public_key = ?")
        .bind(off_curve)
        .execute(&store.pool)
        .await
        .expect("corrupt stored key");
    assert_eq!(
        store.active_passkeys("user-7").await.err(),
        Some(InvalidCredential)
    );
    store.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn rename_and_revoke_classify_missing_revoked_and_malformed_ids() {
    let path = temporary_database("classify");
    let auth = auth();
    let (_, passkey) = registered_fixture(&auth);
    let store = SqlitePasskeyStore::connect(database_url(&path), 32, 8)
        .await
        .expect("connect store");
    store
        .register("user-7", "Laptop", passkey.clone())
        .await
        .expect("register credential");
    assert_eq!(
        store.rename("user-7", &[9, 9, 9], "Phone").await,
        Err(PasskeyStoreError::NotFound)
    );
    assert_eq!(
        store.revoke("user-7", &[1; 1_023]).await,
        Err(PasskeyStoreError::NotFound)
    );
    for malformed in [Vec::new(), vec![1; 1_024]] {
        assert_eq!(
            store.rename("user-7", &malformed, "Phone").await,
            Err(InvalidCredential)
        );
        assert_eq!(
            store.revoke("user-7", &malformed).await,
            Err(InvalidCredential)
        );
    }
    store
        .revoke("user-7", &passkey.credential_id)
        .await
        .expect("revoke device");
    assert_eq!(
        store
            .rename("user-7", &passkey.credential_id, "Phone")
            .await,
        Err(PasskeyStoreError::Revoked)
    );
    store.close().await;
    remove_database(&path);
}

#[test]
fn field_validators_accept_their_limits_and_reject_the_next_byte() {
    for (total, per_subject) in [(1, 1), (3, 3), (1_000_000, 128)] {
        assert_eq!(validate_limits(total, per_subject), Ok(()));
    }
    for (total, per_subject) in [(0, 1), (1_000_001, 1), (4, 0), (200, 129), (2, 3)] {
        assert_eq!(
            validate_limits(total, per_subject),
            Err(InvalidConfiguration("credential limits"))
        );
    }
    let subject = "s".repeat(256);
    let label = "l".repeat(128);
    assert_eq!(validate_subject(&subject), Ok(()));
    assert_eq!(validate_subject("user 7"), Ok(()));
    assert_eq!(validate_label(&label), Ok(()));
    assert_eq!(validate_label("Work laptop"), Ok(()));
    let long_subject = "s".repeat(257);
    let long_label = "l".repeat(129);
    for bad in ["", " user", "user ", "us\ner", long_subject.as_str()] {
        assert_eq!(validate_subject(bad), Err(InvalidConfiguration("subject")));
    }
    for bad in ["", " Laptop", "Laptop ", "Lap\ttop", long_label.as_str()] {
        assert_eq!(
            validate_label(bad),
            Err(InvalidConfiguration("device label"))
        );
    }
    assert_eq!(validate_credential_id(&[1]), Ok(()));
    assert_eq!(validate_credential_id(&[1; 1_023]), Ok(()));
    assert_eq!(validate_credential_id(&[]), Err(InvalidCredential));
    assert_eq!(validate_credential_id(&[1; 1_024]), Err(InvalidCredential));
}

#[test]
fn stored_device_times_may_not_precede_creation() {
    let row = |last: Option<i64>, revoked: Option<i64>| -> DeviceRow {
        (vec![1, 2], "Laptop".to_owned(), 5, 100, last, revoked)
    };
    for (last, revoked) in [(None, None), (Some(100), Some(100)), (Some(150), Some(200))] {
        let summary = decode_summary(row(last, revoked)).expect("consistent row");
        assert_eq!(summary.created_at(), 100);
        assert_eq!(summary.last_used_at(), last.map(|value| value as u64));
        assert_eq!(summary.revoked_at(), revoked.map(|value| value as u64));
    }
    for (last, revoked) in [(Some(99), None), (None, Some(99)), (Some(99), Some(150))] {
        assert_eq!(
            decode_summary(row(last, revoked)),
            Err(CorruptStorage("device timestamps"))
        );
    }
    let mut unborn = row(None, None);
    unborn.3 = 0;
    assert_eq!(
        decode_summary(unborn),
        Err(CorruptStorage("device timestamps"))
    );
}

#[cfg(unix)]
#[tokio::test]
async fn database_targets_must_be_named_regular_files_with_a_known_schema() {
    for url in ["sqlite:", "sqlite://"] {
        assert!(matches!(
            SqlitePasskeyStore::connect(url, 4, 2).await,
            Err(InvalidConfiguration("database must be file-backed"))
        ));
    }
    // Only `mode=memory` is volatile; other open modes stay file-backed.
    let path = temporary_database("mode");
    let store = SqlitePasskeyStore::connect(format!("{}?mode=rwc", database_url(&path)), 4, 2)
        .await
        .expect("read-write-create mode is file-backed");
    store.close().await;
    // A stored schema version from another release is refused.
    let mut raw = <sqlx::SqliteConnection as sqlx::Connection>::connect(&database_url(&path))
        .await
        .expect("raw connection");
    sqlx::query("UPDATE rullst_auth_passkey_meta SET schema_version = 2")
        .execute(&mut raw)
        .await
        .expect("change schema version");
    assert!(matches!(
        SqlitePasskeyStore::connect(database_url(&path), 4, 2).await,
        Err(CorruptStorage("schema configuration"))
    ));
    // So is a non-positive stored limit written around the CHECK constraints.
    for statement in [
        "UPDATE rullst_auth_passkey_meta SET schema_version = 1, max_total_credentials = 0",
        "UPDATE rullst_auth_passkey_meta SET max_total_credentials = 4, max_credentials_per_subject = 0",
    ] {
        sqlx::query("PRAGMA ignore_check_constraints = ON")
            .execute(&mut raw)
            .await
            .expect("ignore CHECK constraints");
        sqlx::query(statement)
            .execute(&mut raw)
            .await
            .expect("corrupt stored limit");
        assert!(matches!(
            SqlitePasskeyStore::connect(database_url(&path), 4, 2).await,
            Err(CorruptStorage("schema configuration"))
        ));
    }
    sqlx::Connection::close(raw).await.expect("close raw");
    remove_database(&path);

    let directory = temporary_database("directory");
    std::fs::create_dir(&directory).expect("create directory");
    assert!(matches!(
        SqlitePasskeyStore::connect(database_url(&directory), 4, 2).await,
        Err(InvalidConfiguration("target must be a regular file"))
    ));
    std::fs::remove_dir(&directory).expect("remove directory");

    // A path below a regular file cannot be inspected (ENOTDIR).
    let file = temporary_database("parent-file");
    std::fs::File::create(&file).expect("create file");
    assert!(matches!(
        SqlitePasskeyStore::connect(database_url(&file.join("child.sqlite")), 4, 2).await,
        Err(PasskeyStoreError::StorageUnavailable(
            "inspect database target"
        ))
    ));
    std::fs::remove_file(file).expect("remove file");
}
