use super::*;
use secrecy::ExposeSecret as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-connect-token-store-{label}-{}-{nonce}.sqlite",
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

fn binding(account: &str) -> TokenSnapshotBinding {
    TokenSnapshotBinding::try_new("github", account).expect("valid binding")
}

fn key(id: &str, byte: u8) -> TokenSnapshotKey {
    TokenSnapshotKey::try_new(id, [byte; 32]).expect("valid key")
}

fn state(access: &str, refresh: &str, generation: u64) -> RefreshableTokenState {
    RefreshableTokenState::try_restore(
        "provider-user".to_string(),
        access.to_string(),
        refresh.to_string(),
        1_000 + generation,
        4_600 + generation,
        generation,
    )
    .expect("valid state")
}

#[tokio::test]
async fn competing_instances_enforce_generation_cas_and_recovery() {
    // TM-CONNECT-24
    let path = temporary_database("cas");
    let url = database_url(&path);
    let first = SqliteTokenSnapshotStore::connect(&url, 8)
        .await
        .expect("first store");
    let second = SqliteTokenSnapshotStore::connect(&url, 8)
        .await
        .expect("second store");
    let owner = binding("application-user");
    let encryption_key = key("token-2026-a", 23);
    let initial = state("access-zero", "refresh-zero", 0);
    let inserted = first
        .insert_initial(&owner, &initial, &encryption_key)
        .await
        .expect("insert initial generation");
    assert_eq!(inserted.generation(), 0);

    let observed = second
        .load(&owner, &encryption_key)
        .await
        .expect("load across instance")
        .expect("stored state");
    assert_eq!(observed.access_token().expose_secret(), "access-zero");
    let candidate_a = state("access-one-a", "refresh-one-a", 1);
    let candidate_b = state("access-one-b", "refresh-one-b", 1);
    let (first_result, second_result) = tokio::join!(
        first.compare_and_swap(&owner, 0, &candidate_a, &encryption_key),
        second.compare_and_swap(&owner, 0, &candidate_b, &encryption_key),
    );
    let results = [first_result, second_result];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(TokenStoreError::GenerationConflict)))
            .count(),
        1
    );
    let winner = first
        .load(&owner, &encryption_key)
        .await
        .expect("load winner")
        .expect("winner exists");
    assert_eq!(winner.generation(), 1);
    assert!(matches!(
        winner.access_token().expose_secret(),
        "access-one-a" | "access-one-b"
    ));
    assert_eq!(
        second.delete_if_generation(&owner, 0).await,
        Err(TokenStoreError::GenerationConflict)
    );
    first.close().await;
    second.close().await;

    let reopened = SqliteTokenSnapshotStore::connect(&url, 8)
        .await
        .expect("reopen store");
    assert_eq!(
        reopened
            .metadata(&owner)
            .await
            .expect("metadata")
            .expect("metadata exists")
            .generation(),
        1
    );
    assert_eq!(
        reopened
            .load(&owner, &encryption_key)
            .await
            .expect("recover generation")
            .expect("state exists")
            .generation(),
        1
    );
    reopened
        .delete_if_generation(&owner, 1)
        .await
        .expect("delete observed generation");
    assert!(
        reopened
            .load(&owner, &encryption_key)
            .await
            .expect("load after delete")
            .is_none()
    );
    reopened.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn quota_configuration_and_disk_content_are_bounded() {
    let path = temporary_database("quota");
    let url = database_url(&path);
    let store = SqliteTokenSnapshotStore::connect(&url, 1)
        .await
        .expect("bounded store");
    let owner = binding("private-account-marker");
    let encryption_key = key("primary", 31);
    let initial = state("access-plaintext-marker", "refresh-plaintext-marker", 0);
    store
        .insert_initial(&owner, &initial, &encryption_key)
        .await
        .expect("insert first row");
    assert_eq!(
        store
            .insert_initial(&owner, &initial, &encryption_key)
            .await,
        Err(TokenStoreError::AlreadyExists)
    );
    assert_eq!(
        store
            .insert_initial(&binding("other-account"), &initial, &encryption_key)
            .await,
        Err(TokenStoreError::CapacityExceeded)
    );
    assert_eq!(
        store.snapshot().await.expect("snapshot"),
        TokenStoreSnapshot {
            entries: 1,
            max_entries: 1,
        }
    );
    let debug = format!("{store:?}");
    assert!(!debug.contains(&url));
    assert!(!debug.contains("private-account-marker"));
    store.close().await;

    let bytes = std::fs::read(&path).expect("read database bytes");
    for marker in [
        b"private-account-marker".as_slice(),
        b"access-plaintext-marker".as_slice(),
        b"refresh-plaintext-marker".as_slice(),
    ] {
        assert!(!bytes.windows(marker.len()).any(|window| window == marker));
    }
    assert!(matches!(
        SqliteTokenSnapshotStore::connect(&url, 2).await,
        Err(TokenStoreError::InvalidConfiguration(
            "entry limit conflicts with stored configuration"
        ))
    ));
    assert!(matches!(
        SqliteTokenSnapshotStore::connect("sqlite::memory:", 1).await,
        Err(TokenStoreError::InvalidConfiguration(
            "database must be file-backed"
        ))
    ));
    remove_database(&path);
}

#[tokio::test]
async fn wrong_keys_and_corrupt_database_rows_fail_closed() {
    let path = temporary_database("corrupt");
    let url = database_url(&path);
    let store = SqliteTokenSnapshotStore::connect(&url, 4)
        .await
        .expect("store");
    let owner = binding("owner");
    let encryption_key = key("primary", 17);
    store
        .insert_initial(
            &owner,
            &state("access-zero", "refresh-zero", 0),
            &encryption_key,
        )
        .await
        .expect("insert row");
    assert!(matches!(
        store.load(&owner, &key("other", 17)).await,
        Err(TokenStoreError::Snapshot(TokenSnapshotError::KeyIdMismatch))
    ));
    assert!(matches!(
        store.load(&owner, &key("primary", 18)).await,
        Err(TokenStoreError::Snapshot(
            TokenSnapshotError::AuthenticationFailed
        ))
    ));

    sqlx::query("UPDATE rullst_connect_token_snapshots SET generation = 1")
        .execute(&store.pool)
        .await
        .expect("tamper generation");
    assert!(matches!(
        store.load(&owner, &encryption_key).await,
        Err(TokenStoreError::CorruptStorage(
            "row and encrypted generations disagree"
        ))
    ));
    sqlx::query("UPDATE rullst_connect_token_snapshots SET envelope = 'invalid'")
        .execute(&store.pool)
        .await
        .expect("tamper envelope");
    assert!(matches!(
        store.metadata(&owner).await,
        Err(TokenStoreError::Snapshot(
            TokenSnapshotError::InvalidEnvelope
        ))
    ));
    store.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn successor_and_missing_row_rules_are_explicit() {
    let path = temporary_database("rules");
    let url = database_url(&path);
    let store = SqliteTokenSnapshotStore::connect(&url, 2)
        .await
        .expect("store");
    let owner = binding("owner");
    let encryption_key = key("primary", 41);
    assert_eq!(
        store
            .insert_initial(&owner, &state("access", "refresh", 1), &encryption_key)
            .await,
        Err(TokenStoreError::InvalidConfiguration(
            "initial generation must be zero"
        ))
    );
    assert_eq!(
        store
            .compare_and_swap(
                &owner,
                0,
                &state("access-two", "refresh-two", 2),
                &encryption_key,
            )
            .await,
        Err(TokenStoreError::InvalidConfiguration(
            "replacement must be the next generation"
        ))
    );
    assert_eq!(
        store
            .compare_and_swap(
                &owner,
                0,
                &state("access-one", "refresh-one", 1),
                &encryption_key,
            )
            .await,
        Err(TokenStoreError::NotFound)
    );
    assert_eq!(
        store.delete_if_generation(&owner, 0).await,
        Err(TokenStoreError::NotFound)
    );
    store.close().await;
    remove_database(&path);
}

#[cfg(unix)]
#[tokio::test]
async fn existing_symbolic_link_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let target = temporary_database("target");
    let link = temporary_database("link");
    std::fs::File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        SqliteTokenSnapshotStore::connect(database_url(&link), 4).await,
        Err(TokenStoreError::InvalidConfiguration(
            "target must be a regular file"
        ))
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");
}
