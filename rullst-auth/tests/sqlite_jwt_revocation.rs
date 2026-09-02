#![cfg(feature = "sqlite")]

use rullst_auth::{
    ApplicationJwtClaims, ApplicationJwtPolicy, JwtError, JwtRevocationMode, JwtSigningKey,
    SqliteJwtRevocationStore,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SIGNING_SECRET: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn temporary_database(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-auth-revocation-{label}-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn database_url(path: &Path) -> String {
    format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
}

fn policy() -> ApplicationJwtPolicy {
    ApplicationJwtPolicy::production(
        "https://auth.example.test",
        "rullst-academy",
        Duration::from_secs(3_600),
        JwtSigningKey::new("2026-09-a", SIGNING_SECRET).expect("strong signing key"),
    )
    .expect("production policy")
}

fn claims(jti: String, subject: String) -> ApplicationJwtClaims {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs();
    ApplicationJwtClaims {
        sub: subject,
        iss: "https://auth.example.test".to_string(),
        aud: "rullst-academy".to_string(),
        iat: now,
        nbf: now,
        exp: now + 3_600,
        jti,
        session_version: 1,
        scopes: Vec::new(),
        token_use: "access".to_string(),
        schema_version: 1,
    }
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
// TM-AUTH-06: production verification observes durable JTI revocation after restart.
async fn production_verification_and_token_revocation_survive_restart() {
    let path = temporary_database("restart");
    let url = database_url(&path);
    let policy = policy();
    let store = SqliteJwtRevocationStore::connect(&url, 128)
        .await
        .expect("connect durable revocations");
    assert_eq!(
        rullst_auth::AsyncJwtRevocationStore::mode(&store),
        JwtRevocationMode::Shared
    );
    let token = policy
        .issue("learner-7", ["course:read"], 3, Duration::from_secs(600))
        .expect("issue token");
    let verified = policy
        .verify_async(&token, &store)
        .await
        .expect("verify against SQLite");
    store
        .revoke_token(&verified)
        .await
        .expect("persist token revocation");
    assert_eq!(
        policy.verify_async(&token, &store).await,
        Err(JwtError::Revoked)
    );
    assert_eq!(
        store
            .snapshot()
            .await
            .expect("snapshot")
            .token_revocations(),
        1
    );
    store.close().await;

    let reopened = SqliteJwtRevocationStore::connect(&url, 128)
        .await
        .expect("reopen durable revocations");
    assert_eq!(
        policy.verify_async(&token, &reopened).await,
        Err(JwtError::Revoked)
    );
    reopened.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn two_instances_share_monotonic_subject_versions() {
    let path = temporary_database("subject");
    let url = database_url(&path);
    let first = SqliteJwtRevocationStore::connect(&url, 32)
        .await
        .expect("first store");
    let second = SqliteJwtRevocationStore::connect(&url, 32)
        .await
        .expect("second store");
    first
        .revoke_subject_before("instructor-2", 5)
        .await
        .expect("advance session version");
    second
        .revoke_subject_before("instructor-2", 3)
        .await
        .expect("lower update remains idempotent");

    let policy = policy();
    let old = policy
        .issue(
            "instructor-2",
            Vec::<String>::new(),
            4,
            Duration::from_secs(300),
        )
        .expect("old token");
    let current = policy
        .issue(
            "instructor-2",
            Vec::<String>::new(),
            5,
            Duration::from_secs(300),
        )
        .expect("current token");
    assert_eq!(
        policy.verify_async(&old, &second).await,
        Err(JwtError::Revoked)
    );
    assert!(policy.verify_async(&current, &second).await.is_ok());
    let snapshot = first.snapshot().await.expect("snapshot");
    assert_eq!(snapshot.subject_revocations(), 1);
    assert_eq!(snapshot.total_entries(), 1);
    assert_eq!(snapshot.max_entries(), 32);

    first.close().await;
    second.close().await;
    remove_database(&path);
}

#[tokio::test]
// TM-AUTH-06: shared revocation capacity remains exact under concurrent writers.
async fn transactional_quota_is_exact_across_competing_instances() {
    let path = temporary_database("quota");
    let url = database_url(&path);
    let first = SqliteJwtRevocationStore::connect(&url, 4)
        .await
        .expect("first store");
    let second = SqliteJwtRevocationStore::connect(&url, 4)
        .await
        .expect("second store");
    let handles = (0..8)
        .map(|index| {
            let store = if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            };
            tokio::spawn(async move {
                store
                    .revoke_token(&claims(
                        format!("token-{index}"),
                        format!("subject-{index}"),
                    ))
                    .await
            })
        })
        .collect::<Vec<_>>();
    let mut accepted = 0;
    let mut exhausted = 0;
    for handle in handles {
        match handle.await.expect("revocation task") {
            Ok(()) => accepted += 1,
            Err(JwtError::RevocationStoreCapacity) => exhausted += 1,
            Err(error) => panic!("unexpected revocation error: {error}"),
        }
    }
    assert_eq!((accepted, exhausted), (4, 4));
    assert_eq!(first.snapshot().await.expect("snapshot").total_entries(), 4);

    first.close().await;
    second.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn updates_do_not_consume_capacity_and_configuration_drift_fails_closed() {
    let path = temporary_database("configuration");
    let url = database_url(&path);
    let store = SqliteJwtRevocationStore::connect(&url, 2)
        .await
        .expect("store");
    store
        .revoke_subject_before("learner-a", 2)
        .await
        .expect("first subject");
    store
        .revoke_subject_before("learner-b", 2)
        .await
        .expect("second subject");
    store
        .revoke_subject_before("learner-a", 9)
        .await
        .expect("existing subject update");
    assert_eq!(
        store.revoke_subject_before("learner-c", 2).await,
        Err(JwtError::RevocationStoreCapacity)
    );
    store.close().await;

    assert!(matches!(
        SqliteJwtRevocationStore::connect(&url, 3).await,
        Err(JwtError::InvalidConfiguration(
            "SQLite revocation max_entries conflicts with stored configuration"
        ))
    ));
    remove_database(&path);
}

#[tokio::test]
async fn volatile_and_corrupt_revocation_databases_are_rejected() {
    assert!(matches!(
        SqliteJwtRevocationStore::connect("sqlite::memory:", 16).await,
        Err(JwtError::InvalidConfiguration(
            "SQLite revocation database must be file-backed"
        ))
    ));

    let path = temporary_database("corrupt");
    let url = database_url(&path);
    let store = SqliteJwtRevocationStore::connect(&url, 16)
        .await
        .expect("store");
    store.close().await;
    let pool = sqlx::SqlitePool::connect(&url)
        .await
        .expect("open fixture database");
    sqlx::query("UPDATE rullst_auth_jwt_meta SET schema_version = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .expect("corrupt schema version");
    pool.close().await;
    assert!(matches!(
        SqliteJwtRevocationStore::connect(&url, 16).await,
        Err(JwtError::RevocationBackend(message))
            if message == "validate SQLite revocation schema"
    ));
    remove_database(&path);
}
