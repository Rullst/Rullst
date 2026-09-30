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
    let file_backed = database_url(&temporary_database("volatile"));
    for url in [
        "sqlite::memory:".to_string(),
        "sqlite::memory:?cache=private".to_string(),
        "sqlite://:memory:?cache=shared".to_string(),
        "sqlite:file:revocations%3Fmode%3Dmemory".to_string(),
        format!("{file_backed}?vfs=memdb"),
        format!("{file_backed}?immutable=1"),
        format!("{file_backed}?mode=memory"),
    ] {
        assert!(
            matches!(
                SqliteJwtRevocationStore::connect(url.as_str(), 16).await,
                Err(JwtError::InvalidConfiguration(
                    "SQLite revocation database must be file-backed"
                ))
            ),
            "accepted volatile URL {url}"
        );
    }

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

fn claims_issued_at(jti: String, subject: &str, issued_at: u64) -> ApplicationJwtClaims {
    let mut claims = claims(jti, subject.to_string());
    claims.iat = issued_at;
    claims.nbf = issued_at;
    claims
}

async fn revoked(store: &SqliteJwtRevocationStore, claims: &ApplicationJwtClaims) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs();
    rullst_auth::AsyncJwtRevocationStore::is_revoked(store, claims, now)
        .await
        .expect("read revocation")
}

#[tokio::test]
// TM-AUTH-06: one subject's repeated logout cannot exhaust shared capacity.
async fn one_subject_cannot_exhaust_shared_revocation_capacity() {
    let path = temporary_database("per-subject");
    let url = database_url(&path);
    let store = SqliteJwtRevocationStore::connect(&url, 128)
        .await
        .expect("store");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs();
    let mut abusive = Vec::new();
    for index in 0..200 {
        let claims = claims_issued_at(format!("abusive-{index}"), "learner-a", now - 10);
        store
            .revoke_token(&claims)
            .await
            .expect("revocation never fails for the capped subject");
        abusive.push(claims);
    }
    for claims in &abusive {
        assert!(revoked(&store, claims).await);
    }
    let later = claims_issued_at("abusive-later".to_string(), "learner-a", now);
    assert!(!revoked(&store, &later).await);

    let bystander = claims_issued_at("bystander".to_string(), "learner-b", now - 10);
    let sibling = claims_issued_at("bystander-sibling".to_string(), "learner-b", now - 20);
    store
        .revoke_token(&bystander)
        .await
        .expect("another subject can still log out");
    assert!(revoked(&store, &bystander).await);
    assert!(!revoked(&store, &sibling).await);
    store
        .revoke_subject_before("learner-c", 2)
        .await
        .expect("subject revocation still has capacity");
    let snapshot = store.snapshot().await.expect("snapshot");
    assert_eq!(snapshot.token_revocations(), 65);
    assert_eq!(snapshot.subject_revocations(), 2);
    store.close().await;

    // The cutoff survives restart.
    let reopened = SqliteJwtRevocationStore::connect(&url, 128)
        .await
        .expect("reopen");
    assert!(revoked(&reopened, &abusive[199]).await);
    assert!(!revoked(&reopened, &later).await);
    reopened.close().await;
    remove_database(&path);
}

#[tokio::test]
async fn legacy_revocation_files_gain_additive_columns_and_stay_compatible() {
    let path = temporary_database("legacy");
    let url = database_url(&path);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_secs();
    let expires_at = i64::try_from(now + 3_600).unwrap();
    let pool = sqlx::SqlitePool::connect(&format!("{url}?mode=rwc"))
        .await
        .expect("legacy fixture");
    for statement in [
        "CREATE TABLE rullst_auth_jwt_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_entries INTEGER NOT NULL CHECK (max_entries > 0))",
        "CREATE TABLE rullst_auth_jwt_tokens (jti TEXT PRIMARY KEY, expires_at INTEGER NOT NULL CHECK (expires_at > 0))",
        "CREATE TABLE rullst_auth_jwt_subjects (subject TEXT PRIMARY KEY, minimum_session_version INTEGER NOT NULL CHECK (minimum_session_version > 0))",
        "INSERT INTO rullst_auth_jwt_meta VALUES (1, 1, 16)",
        "INSERT INTO rullst_auth_jwt_subjects VALUES ('instructor-2', 5)",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .expect("legacy schema");
    }
    sqlx::query("INSERT INTO rullst_auth_jwt_tokens VALUES ('legacy-jti', ?)")
        .bind(expires_at)
        .execute(&pool)
        .await
        .expect("legacy token row");
    pool.close().await;

    let store = SqliteJwtRevocationStore::connect(&url, 16)
        .await
        .expect("additive migration");
    let legacy = claims_issued_at("legacy-jti".to_string(), "learner-7", now);
    assert!(revoked(&store, &legacy).await);
    let mut old_session = claims_issued_at("old-session".to_string(), "instructor-2", now);
    old_session.session_version = 4;
    assert!(revoked(&store, &old_session).await);
    store
        .revoke_token(&claims_issued_at("new-jti".to_string(), "learner-7", now))
        .await
        .expect("new revocation");
    store.close().await;

    // An older release keeps writing with its explicit two-column statements.
    let pool = sqlx::SqlitePool::connect(&url).await.expect("older writer");
    sqlx::query("INSERT INTO rullst_auth_jwt_subjects (subject, minimum_session_version) VALUES ('learner-9', 2)")
        .execute(&pool)
        .await
        .expect("older subject write");
    sqlx::query("INSERT INTO rullst_auth_jwt_tokens (jti, expires_at) VALUES ('older-jti', ?)")
        .bind(expires_at)
        .execute(&pool)
        .await
        .expect("older token write");
    pool.close().await;
    let reopened = SqliteJwtRevocationStore::connect(&url, 16)
        .await
        .expect("idempotent migration");
    assert_eq!(
        reopened.snapshot().await.expect("snapshot").total_entries(),
        5
    );
    reopened.close().await;
    remove_database(&path);
}
