use super::*;
use crate::auth::passkey::PasskeyConfig;
use crate::auth::passkey::test_support::{
    RegistrationOptions, assertion_fixture, registration_fixture,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn auth() -> PasskeyAuth {
    PasskeyAuth::new(&PasskeyConfig::new(
        "Test App",
        "localhost",
        "http://localhost",
    ))
    .expect("localhost WebAuthn configuration")
}

fn temporary_database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-passkey-store-{label}-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn database_url(path: &Path) -> String {
    format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
}

fn remove_database(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{suffix}", path.display()));
        match std::fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite fixture: {error}"),
        }
    }
}

#[tokio::test]
// TM-AUTH-05: durable device state preserves ceremony and revocation invariants.
async fn complete_device_lifecycle_survives_restart_and_rejects_replay() {
    let path = temporary_database("lifecycle");
    let url = database_url(&path);
    let auth = auth();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .expect("finish registration");
    let store = SqlitePasskeyStore::connect(&url, 32, 8)
        .await
        .expect("connect store");
    let registered = store
        .register("user-7", "Laptop", passkey.clone())
        .await
        .expect("persist credential");
    assert_eq!(registered.label(), "Laptop");
    assert_eq!(registered.sign_count(), 1);

    let (assertion, challenge) =
        assertion_fixture(&auth, &passkey, &fixture.key_pair, "localhost", 0x05, 2);
    let updated = store
        .finish_authenticate(&auth, "user-7", &assertion, &challenge)
        .await
        .expect("verify and persist assertion");
    assert_eq!(updated.sign_count, 2);
    assert!(matches!(
        store
            .finish_authenticate(&auth, "user-7", &assertion, &challenge)
            .await,
        Err(PasskeyStoreError::CeremonyRejected)
    ));
    store
        .rename("user-7", &passkey.credential_id, "Primary laptop")
        .await
        .expect("rename device");
    let summary = store.devices("user-7").await.expect("list devices");
    assert_eq!(summary[0].label(), "Primary laptop");
    assert_eq!(summary[0].sign_count(), 2);
    assert!(summary[0].last_used_at().is_some());
    store.close().await;

    let reopened = SqlitePasskeyStore::connect(&url, 32, 8)
        .await
        .expect("reopen store");
    assert_eq!(
        reopened
            .active_passkeys("user-7")
            .await
            .expect("load active credentials")[0]
            .sign_count,
        2
    );
    reopened
        .revoke("user-7", &passkey.credential_id)
        .await
        .expect("revoke credential");
    reopened
        .revoke("user-7", &passkey.credential_id)
        .await
        .expect("revoke remains idempotent");
    assert!(
        reopened
            .active_passkeys("user-7")
            .await
            .expect("load active credentials")
            .is_empty()
    );
    assert!(reopened.devices("user-7").await.expect("list devices")[0].is_revoked());
    reopened.close().await;
    remove_database(&path);
}

#[tokio::test]
// TM-AUTH-05: signature counters use compare-and-set across local processes.
async fn competing_counter_updates_use_compare_and_set() {
    let path = temporary_database("counter-cas");
    let url = database_url(&path);
    let auth = auth();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let passkey = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .expect("finish registration");
    let first = SqlitePasskeyStore::connect(&url, 32, 8)
        .await
        .expect("first store");
    let second = SqlitePasskeyStore::connect(&url, 32, 8)
        .await
        .expect("second store");
    first
        .register("user-7", "Security key", passkey.clone())
        .await
        .expect("register credential");
    let mut updated_two = passkey.clone();
    updated_two.sign_count = 2;
    let mut updated_three = passkey.clone();
    updated_three.sign_count = 3;
    let (left, right) = tokio::join!(
        first.advance_counter("user-7", &passkey, &updated_two),
        second.advance_counter("user-7", &passkey, &updated_three)
    );
    let results = [left, right];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err(PasskeyStoreError::CounterConflict))
            .count(),
        1
    );
    let stored = first.devices("user-7").await.expect("list device");
    assert!(matches!(stored[0].sign_count(), 2 | 3));

    first.close().await;
    second.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn quotas_duplicates_and_configuration_drift_fail_closed() {
    let path = temporary_database("quota");
    let url = database_url(&path);
    let auth = auth();
    let fixture = registration_fixture(&auth, "localhost", RegistrationOptions::default());
    let first = auth
        .finish_register(&fixture.credential, &fixture.challenge)
        .expect("finish registration");
    let mut second = first.clone();
    second.credential_id = vec![20, 30, 40, 50];
    let mut third = first.clone();
    third.credential_id = vec![30, 40, 50, 60];
    let store = SqlitePasskeyStore::connect(&url, 2, 1)
        .await
        .expect("connect store");
    store
        .register("user-a", "First", first.clone())
        .await
        .expect("first credential");
    assert_eq!(
        store.register("user-a", "Duplicate", first).await,
        Err(PasskeyStoreError::AlreadyExists)
    );
    assert_eq!(
        store
            .register("user-a", "Per subject", second.clone())
            .await,
        Err(PasskeyStoreError::CapacityExceeded)
    );
    store
        .register("user-b", "Second", second)
        .await
        .expect("second subject");
    assert_eq!(
        store.register("user-c", "Total", third).await,
        Err(PasskeyStoreError::CapacityExceeded)
    );
    store.close().await;
    assert!(matches!(
        SqlitePasskeyStore::connect(&url, 3, 1).await,
        Err(PasskeyStoreError::InvalidConfiguration(
            "credential limits conflict with stored configuration"
        ))
    ));
    remove_database(&path);
}

#[cfg(unix)]
#[tokio::test]
async fn memory_and_existing_symlink_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    assert!(matches!(
        SqlitePasskeyStore::connect("sqlite::memory:", 4, 2).await,
        Err(PasskeyStoreError::InvalidConfiguration(
            "database must be file-backed"
        ))
    ));
    let target = temporary_database("target");
    let link = temporary_database("link");
    std::fs::File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        SqlitePasskeyStore::connect(database_url(&link), 4, 2).await,
        Err(PasskeyStoreError::InvalidConfiguration(
            "target must be a regular file"
        ))
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");
}
