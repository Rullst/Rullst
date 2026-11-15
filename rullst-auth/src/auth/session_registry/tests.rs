//! The registry on an in-memory SQLite pool: storage, limits, last-seen
//! hours and corrupt rows. The database-independent contract is in
//! `rullst-auth/tests/session_registry_contract.rs`.

use super::*;
use crate::{DEFAULT_SESSION_MAX_AGE, decrypt_session_claims};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Duration;

const KEY: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";
/// 2026-11-15T10:00:00Z, the start of an hour.
const HOUR: u64 = 1_794_736_800;
const NOW: u64 = HOUR + 1_234;

async fn pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap()
}

async fn registry() -> (SqliteSessionRegistry, SqlitePool) {
    let pool = pool().await;
    (SessionRegistry::open(pool.clone()).await.unwrap(), pool)
}

async fn login(registry: &SqliteSessionRegistry, user_id: i32, now: u64) -> IssuedSession {
    registry
        .create_session(&NewSession::new(user_id, 3), KEY, now)
        .await
        .unwrap()
}

type Row = (String, String, i32, i64, i64, i64, String, String);

async fn rows(pool: &SqlitePool) -> Vec<Row> {
    sqlx::query_as("SELECT token_hash, public_id, user_id, created_at, last_seen_at, expires_at, browser, platform FROM rullst_auth_sessions ORDER BY created_at, user_id, token_hash")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn execute(pool: &SqlitePool, statement: &'static str) {
    sqlx::query(statement).execute(pool).await.unwrap();
}

#[tokio::test]
async fn the_table_stores_digests_and_minimal_metadata_only() {
    let (registry, pool) = registry().await;
    let firefox = DeviceSummary::new(Browser::Firefox, Platform::Linux);
    let issued = registry
        .create_session(
            &NewSession::new(7, 2)
                .with_device(firefox)
                .with_max_age(Duration::from_secs(600)),
            KEY,
            NOW,
        )
        .await
        .unwrap();
    assert_eq!(issued.expires_at(), NOW + 600);
    let stored = rows(&pool).await;
    assert_eq!(stored.len(), 1);
    let (hash, public_id, user_id, created, seen, expires, browser, platform) = &stored[0];
    assert_eq!(hash.len(), 64);
    assert_eq!(public_id, issued.reference().as_str());
    assert_eq!(
        (*user_id, *created, *seen, *expires),
        (7, NOW as i64, HOUR as i64, (NOW + 600) as i64)
    );
    assert_eq!((browser.as_str(), platform.as_str()), ("firefox", "linux"));
    // The cookie's secret is in no column: the stored values cannot be
    // turned back into the cookie.
    let token = issued.token();
    let secret = token::open(
        &crate::auth::session::derive_cipher(KEY).unwrap(),
        token,
        None,
    )
    .unwrap()
    .1;
    assert_eq!(&secret.lookup_hash(), hash);
    let encoded = secret.encode();
    for column in [hash, public_id, browser, platform] {
        assert!(!column.contains(encoded.as_str()));
        assert!(!token.contains(column.as_str()));
    }
    // A digest copied from the table is not a cookie.
    assert_eq!(
        registry.authenticate(hash, KEY, NOW).await.err(),
        Some(SessionRegistryError::InvalidToken)
    );
    // `open` again keeps the table and its rows.
    let reopened = SessionRegistry::open(pool.clone()).await.unwrap();
    assert_eq!(
        reopened
            .authenticate(token, KEY, NOW)
            .await
            .unwrap()
            .user_id(),
        7
    );
}

#[tokio::test]
async fn the_last_seen_hour_moves_forward_once_an_hour() {
    let (registry, pool) = registry().await;
    let issued = login(&registry, 1, NOW).await;
    let seen = |rows: Vec<Row>| rows[0].4;
    assert_eq!(seen(rows(&pool).await), HOUR as i64);
    // The same hour writes nothing.
    registry
        .authenticate(issued.token(), KEY, HOUR + 3_599)
        .await
        .unwrap();
    assert_eq!(seen(rows(&pool).await), HOUR as i64);
    // The next hour records its start.
    registry
        .authenticate(issued.token(), KEY, HOUR + 3_600)
        .await
        .unwrap();
    assert_eq!(seen(rows(&pool).await), (HOUR + 3_600) as i64);
    registry
        .authenticate(issued.token(), KEY, HOUR + 7_201)
        .await
        .unwrap();
    assert_eq!(seen(rows(&pool).await), (HOUR + 7_200) as i64);
    // An earlier clock never moves it back.
    registry
        .authenticate(issued.token(), KEY, HOUR + 10)
        .await
        .unwrap();
    assert_eq!(seen(rows(&pool).await), (HOUR + 7_200) as i64);
    let listed = registry.list_sessions(1, HOUR + 7_300).await.unwrap();
    assert_eq!(listed[0].last_seen_at(), HOUR + 7_200);
    assert_eq!(listed[0].created_at(), NOW);
}

#[tokio::test]
async fn a_request_in_the_same_hour_writes_nothing() {
    let (registry, pool) = registry().await;
    let issued = login(&registry, 1, NOW).await;
    // The pool's one connection now refuses every write statement, even
    // an UPDATE that would match no row.
    execute(&pool, "PRAGMA query_only = ON").await;
    // The last-seen hour is current: authentication only reads.
    for now in [HOUR, NOW, HOUR + 3_599] {
        assert!(
            registry
                .authenticate(issued.token(), KEY, now)
                .await
                .is_ok()
        );
    }
    // A new hour needs the write, and a failed write never authenticates.
    assert_eq!(
        registry
            .authenticate(issued.token(), KEY, HOUR + 3_600)
            .await
            .err(),
        Some(SessionRegistryError::Unavailable)
    );
}

#[tokio::test]
async fn a_sign_in_beyond_the_limit_signs_out_the_least_recently_used_session() {
    let (registry, pool) = registry().await;
    let mut sessions = Vec::new();
    for index in 0..MAX_SESSIONS_PER_USER {
        sessions.push(login(&registry, 5, NOW + u64::from(index)).await);
    }
    let other = login(&registry, 6, NOW).await;
    assert_eq!(
        registry.list_sessions(5, NOW + 100).await.unwrap().len(),
        50
    );
    // The first session is used an hour later, so the second is the least
    // recently used one.
    registry
        .authenticate(sessions[0].token(), KEY, HOUR + 3_600)
        .await
        .unwrap();
    let newest = login(&registry, 5, HOUR + 3_700).await;
    assert_eq!(
        registry.list_sessions(5, HOUR + 3_700).await.unwrap().len(),
        50
    );
    assert_eq!(
        registry
            .authenticate(sessions[1].token(), KEY, HOUR + 3_700)
            .await
            .err(),
        Some(SessionRegistryError::Revoked)
    );
    for session in [&sessions[0], &sessions[2], &newest, &other] {
        assert!(
            registry
                .authenticate(session.token(), KEY, HOUR + 3_700)
                .await
                .is_ok()
        );
    }
    // Another account's sessions are not counted.
    assert_eq!(rows(&pool).await.len(), 51);
}

#[tokio::test]
async fn a_sign_in_deletes_its_accounts_expired_sessions_only() {
    let (registry, pool) = registry().await;
    let short = NewSession::new(9, 0).with_max_age(Duration::from_secs(10));
    registry.create_session(&short, KEY, NOW).await.unwrap();
    let other = NewSession::new(10, 0).with_max_age(Duration::from_secs(10));
    registry.create_session(&other, KEY, NOW).await.unwrap();
    // Valid through its last second: not deleted yet.
    login(&registry, 9, NOW + 10).await;
    assert_eq!(rows(&pool).await.len(), 3);
    login(&registry, 9, NOW + 11).await;
    let users: Vec<i32> = rows(&pool).await.iter().map(|row| row.2).collect();
    assert_eq!(users, vec![10, 9, 9]);
}

#[tokio::test]
async fn purging_deletes_expired_sessions_in_bounded_batches() {
    let (registry, pool) = registry().await;
    for (user_id, lifetime) in [(1, 10), (2, 20), (3, 30), (4, 600)] {
        let session = NewSession::new(user_id, 0).with_max_age(Duration::from_secs(lifetime));
        registry.create_session(&session, KEY, NOW).await.unwrap();
    }
    for limit in [0, MAX_PURGE_BATCH + 1] {
        assert_eq!(
            registry.purge_expired(NOW + 100, limit).await.err(),
            Some(SessionRegistryError::InvalidInput("limit"))
        );
    }
    // Valid through NOW + 20: only the first has expired at NOW + 20.
    assert_eq!(
        registry
            .purge_expired(NOW + 20, MAX_PURGE_BATCH)
            .await
            .unwrap(),
        1
    );
    // The oldest expiry goes first.
    assert_eq!(registry.purge_expired(NOW + 100, 1).await.unwrap(), 1);
    let users: Vec<i32> = rows(&pool).await.iter().map(|row| row.2).collect();
    assert_eq!(users, vec![3, 4]);
    assert_eq!(registry.purge_expired(NOW + 100, 5).await.unwrap(), 1);
    assert_eq!(registry.purge_expired(NOW + 100, 5).await.unwrap(), 0);
    assert_eq!(rows(&pool).await.len(), 1);
}

#[tokio::test]
async fn corrupt_rows_never_authenticate() {
    let (registry, pool) = registry().await;
    for statement in [
        "UPDATE rullst_auth_sessions SET public_id = '!!!!!!!!!!!!!!!!!!!!!!'",
        "UPDATE rullst_auth_sessions SET created_at = -1",
        "UPDATE rullst_auth_sessions SET last_seen_at = -1",
        "UPDATE rullst_auth_sessions SET expires_at = -1",
        "UPDATE rullst_auth_sessions SET expires_at = created_at - 1",
    ] {
        execute(&pool, "DELETE FROM rullst_auth_sessions").await;
        let issued = login(&registry, 1, NOW).await;
        execute(&pool, statement).await;
        assert_eq!(
            registry.authenticate(issued.token(), KEY, NOW).await.err(),
            Some(SessionRegistryError::Corrupt),
            "{statement}"
        );
        // The list leaves out a negative expiry like any expired session.
        let listed = registry.list_sessions(1, 0).await;
        if statement.ends_with("expires_at = -1") {
            assert_eq!(listed, Ok(Vec::new()));
        } else {
            assert_eq!(
                listed.err(),
                Some(SessionRegistryError::Corrupt),
                "{statement}"
            );
        }
    }
    // A session that expires as it is created is valid for that second.
    execute(&pool, "DELETE FROM rullst_auth_sessions").await;
    let issued = login(&registry, 1, NOW).await;
    execute(
        &pool,
        "UPDATE rullst_auth_sessions SET expires_at = created_at",
    )
    .await;
    assert!(
        registry
            .authenticate(issued.token(), KEY, NOW)
            .await
            .is_ok()
    );
    // A browser or platform code of a newer version reads as unknown.
    execute(
        &pool,
        "UPDATE rullst_auth_sessions SET browser = 'netscape', platform = 'beos'",
    )
    .await;
    let listed = registry.list_sessions(1, NOW).await.unwrap();
    assert_eq!(listed[0].device(), DeviceSummary::unknown());
}

#[tokio::test]
async fn a_record_of_another_account_or_an_earlier_expiry_wins() {
    let (registry, pool) = registry().await;
    let issued = login(&registry, 1, NOW).await;
    execute(&pool, "UPDATE rullst_auth_sessions SET user_id = 2").await;
    assert_eq!(
        registry.authenticate(issued.token(), KEY, NOW).await.err(),
        Some(SessionRegistryError::Revoked)
    );
    execute(&pool, "UPDATE rullst_auth_sessions SET user_id = 1").await;
    // The cookie lasts 30 days; an operator shortened the record.
    let shortened = format!("UPDATE rullst_auth_sessions SET expires_at = {}", NOW + 60);
    sqlx::query(sqlx::AssertSqlSafe(shortened))
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        registry
            .authenticate(issued.token(), KEY, NOW + 60)
            .await
            .is_ok()
    );
    assert_eq!(
        registry
            .authenticate(issued.token(), KEY, NOW + 61)
            .await
            .err(),
        Some(SessionRegistryError::Expired)
    );
}

#[tokio::test]
async fn inputs_out_of_range_are_refused_before_storage() {
    let (registry, pool) = registry().await;
    for max_age in [Duration::ZERO, Duration::from_secs(34_560_001)] {
        let session = NewSession::new(1, 0).with_max_age(max_age);
        assert_eq!(
            registry.create_session(&session, KEY, NOW).await.err(),
            Some(SessionRegistryError::InvalidInput("max_age"))
        );
    }
    let longest = NewSession::new(1, 0).with_max_age(Duration::from_secs(34_560_000));
    assert!(registry.create_session(&longest, KEY, NOW).await.is_ok());
    for now in [u64::MAX, i64::MAX as u64] {
        assert_eq!(
            registry
                .create_session(&NewSession::new(1, 0), KEY, now)
                .await
                .err(),
            Some(SessionRegistryError::InvalidInput("time"))
        );
    }
    assert_eq!(
        registry
            .create_session(&NewSession::new(1, 0), b"short", NOW)
            .await
            .err(),
        Some(SessionRegistryError::Configuration)
    );
    assert_eq!(rows(&pool).await.len(), 1);
    assert_eq!(
        registry.list_sessions(1, u64::MAX).await.err(),
        Some(SessionRegistryError::InvalidInput("time"))
    );
    assert_eq!(
        registry.purge_expired(u64::MAX, 1).await.err(),
        Some(SessionRegistryError::InvalidInput("time"))
    );
    let issued = login(&registry, 1, NOW).await;
    assert_eq!(
        registry
            .authenticate(issued.token(), b"short", NOW)
            .await
            .err(),
        Some(SessionRegistryError::Configuration)
    );
    assert_eq!(
        registry.revoke_token(issued.token(), b"short").await.err(),
        Some(SessionRegistryError::Configuration)
    );
}

#[tokio::test]
async fn epoch_cookies_and_registered_cookies_do_not_mix() {
    let (registry, _) = registry().await;
    let issued = login(&registry, 1, NOW).await;
    // A route that skips the registry cannot accept a registered cookie.
    assert!(matches!(
        decrypt_session_claims(issued.token(), KEY),
        Err(crate::AuthError::SessionDecryptionError(_))
    ));
    // The registry refuses the stateless cookies.
    let epoch = crate::encrypt_session_with_epoch(1, 3, DEFAULT_SESSION_MAX_AGE, KEY).unwrap();
    let legacy = crate::encrypt_session(1, KEY).unwrap();
    for cookie in [epoch, legacy] {
        assert_eq!(
            registry.authenticate(&cookie, KEY, NOW).await.err(),
            Some(SessionRegistryError::InvalidToken)
        );
        assert_eq!(
            registry.revoke_token(&cookie, KEY).await.err(),
            Some(SessionRegistryError::InvalidToken)
        );
    }
    // The registered cookie carries the epoch for `verify_epoch`.
    let session = registry
        .authenticate(issued.token(), KEY, NOW)
        .await
        .unwrap();
    assert_eq!(session.claims().verify_epoch(3), Ok(1));
    assert_eq!(
        session.claims().verify_epoch(4),
        Err(crate::AuthError::SessionRevoked)
    );
}

#[tokio::test]
async fn values_print_without_secrets() {
    let (registry, _) = registry().await;
    let issued = login(&registry, 1, NOW).await;
    let session = registry
        .authenticate(issued.token(), KEY, NOW)
        .await
        .unwrap();
    let reference = issued.reference().as_str();
    assert_eq!(
        format!("{registry:?}"),
        "SessionRegistry { database: \"[REDACTED]\" }"
    );
    assert_eq!(
        format!("{issued:?}"),
        format!(
            "IssuedSession {{ token: \"[REDACTED]\", reference: SessionRef(\"{reference}\"), expires_at: {} }}",
            NOW + 2_592_000
        )
    );
    assert_eq!(
        format!("{:?}", session.current()),
        format!(
            "CurrentSession {{ user_id: 1, reference: SessionRef(\"{reference}\"), expires_at: {}, .. }}",
            NOW + 2_592_000
        )
    );
    let printed = format!("{session:?}");
    assert!(!printed.contains(issued.token()));
    assert!(!printed.contains(&session.current().token_hash));
    assert_eq!(format!("{}", issued.reference()), reference);
    let clone = registry.clone();
    assert!(clone.authenticate(issued.token(), KEY, NOW).await.is_ok());
}
