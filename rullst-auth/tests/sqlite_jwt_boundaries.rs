#![cfg(feature = "sqlite")]
//! Input, pruning, cutoff and database-target boundaries of the SQLite JWT store.

use rullst_auth::{
    ApplicationJwtClaims, AsyncJwtRevocationStore, JwtError, SqliteJwtRevocationStore,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs()
}

fn temporary_database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-auth-jwt-bounds-{label}-{}-{nonce}.sqlite",
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

fn claims(jti: &str, subject: &str, issued_at: u64) -> ApplicationJwtClaims {
    ApplicationJwtClaims {
        sub: subject.to_string(),
        iss: "https://auth.example.test".to_string(),
        aud: "rullst-academy".to_string(),
        iat: issued_at,
        nbf: issued_at,
        exp: now() + 3_600,
        jti: jti.to_string(),
        session_version: 5,
        scopes: Vec::new(),
        token_use: "access".to_string(),
        schema_version: 1,
    }
}

#[tokio::test]
async fn subject_revocations_need_an_identity_and_a_positive_version() {
    let path = temporary_database("subject");
    let store = SqliteJwtRevocationStore::connect(database_url(&path), 16)
        .await
        .expect("store");
    for (subject, version) in [("learner", 0), (" learner", 2), ("", 2)] {
        assert_eq!(
            store.revoke_subject_before(subject, version).await,
            Err(JwtError::InvalidConfiguration("subject revocation"))
        );
    }
    assert_eq!(store.snapshot().await.expect("snapshot").total_entries(), 0);
    // A version-only revocation has no issue-time cutoff, even for `iat` 0.
    store
        .revoke_subject_before("learner", 2)
        .await
        .expect("subject revocation");
    let current = claims("token-a", "learner", 0);
    assert!(!store.is_revoked(&current, now()).await.expect("lookup"));
    let mut older = current.clone();
    older.session_version = 1;
    assert!(store.is_revoked(&older, now()).await.expect("lookup"));
    store.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn snapshots_prune_expired_tokens_and_close_shuts_the_shared_pool() {
    let path = temporary_database("prune");
    let url = database_url(&path);
    let store = SqliteJwtRevocationStore::connect(&url, 16)
        .await
        .expect("store");
    store
        .revoke_token(&claims("token-a", "learner", now()))
        .await
        .expect("token revocation");
    assert_eq!(
        store
            .snapshot()
            .await
            .expect("snapshot")
            .token_revocations(),
        1
    );
    // Age the row past its expiry, as if its token had expired.
    let pool = sqlx::SqlitePool::connect(&url).await.expect("raw pool");
    sqlx::query("UPDATE rullst_auth_jwt_tokens SET expires_at = 1")
        .execute(&pool)
        .await
        .expect("expire row");
    pool.close().await;
    let snapshot = store.snapshot().await.expect("snapshot");
    assert_eq!(snapshot.token_revocations(), 0);
    assert_eq!(snapshot.max_entries(), 16);

    let other = store.clone();
    store.close().await;
    assert!(matches!(
        other.snapshot().await,
        Err(JwtError::RevocationBackend(_))
    ));
    remove_database(&path);
}

#[cfg(unix)]
#[tokio::test]
async fn database_targets_must_be_named_regular_files() {
    let file_backed = "SQLite revocation database must be file-backed";
    for url in ["sqlite:", "sqlite://"] {
        assert_eq!(
            SqliteJwtRevocationStore::connect(url, 16).await.err(),
            Some(JwtError::InvalidConfiguration(file_backed))
        );
    }
    // Only `mode=memory` is volatile; other open modes stay file-backed.
    let path = temporary_database("mode");
    let store = SqliteJwtRevocationStore::connect(format!("{}?mode=rwc", database_url(&path)), 16)
        .await
        .expect("read-write-create mode is file-backed");
    store.close().await;
    remove_database(&path);

    let regular = "SQLite revocation target must be a regular file";
    let target = temporary_database("target");
    std::fs::File::create(&target).expect("create target");
    let link = temporary_database("link");
    std::os::unix::fs::symlink(&target, &link).expect("create symlink");
    let directory = temporary_database("directory");
    std::fs::create_dir(&directory).expect("create directory");
    for unsafe_target in [&link, &directory] {
        assert_eq!(
            SqliteJwtRevocationStore::connect(database_url(unsafe_target), 16)
                .await
                .err(),
            Some(JwtError::InvalidConfiguration(regular))
        );
    }
    // A path below a regular file cannot be inspected (ENOTDIR).
    assert_eq!(
        SqliteJwtRevocationStore::connect(database_url(&target.join("child.sqlite")), 16)
            .await
            .err(),
        Some(JwtError::RevocationBackend(
            "inspect SQLite revocation target".to_string()
        ))
    );
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_dir(directory).expect("remove directory");
    std::fs::remove_file(target).expect("remove target");
}
