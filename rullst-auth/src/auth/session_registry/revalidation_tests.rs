//! The check of a long-lived response of a session. Every revocation test is
//! in one test function: the notices are process-wide, and accounts from
//! 9001 on are used by no other test of this binary.

use super::*;
use crate::auth::session_registry::{NewSession, SqliteSessionRegistry};
use sqlx::sqlite::SqlitePoolOptions;
use std::time::Duration;

const KEY: &[u8; 32] = b"0123456789abcdefghijklmnopqrstuv";

async fn registry() -> (SqliteSessionRegistry, sqlx::SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    (SessionRegistry::open(pool.clone()).await.unwrap(), pool)
}

/// A signed-in session of `user_id`, issued `ago` seconds ago and valid for
/// `lifetime` seconds.
async fn signed_in(
    registry: &SqliteSessionRegistry,
    user_id: i32,
    ago: u64,
    lifetime: u64,
) -> CurrentSession {
    let now = unix_now().unwrap() - ago;
    let login = NewSession::new(user_id, 0).with_max_age(Duration::from_secs(lifetime));
    let issued = registry.create_session(&login, KEY, now).await.unwrap();
    registry
        .authenticate(issued.token(), KEY, now)
        .await
        .unwrap()
        .into_current()
}

/// Whether `check`'s change signal fires within a short wait.
async fn signals(check: &mut SessionRevalidation<sqlx::Sqlite>) -> bool {
    tokio::time::timeout(Duration::from_millis(200), check.changed())
        .await
        .is_ok()
}

fn reference(byte: char) -> SessionRef {
    SessionRef(std::iter::repeat_n(byte, 22).collect())
}

#[test]
fn a_revocation_covers_exactly_the_sessions_it_ended() {
    let session = CurrentSession {
        user_id: 7,
        token_hash: "h".repeat(64),
        reference: reference('a'),
        created_at: 0,
        expires_at: 0,
    };
    let one = |user_id, byte| Revoked::One {
        user_id,
        reference: reference(byte),
    };
    let others = |user_id, byte| Revoked::Others {
        user_id,
        kept: reference(byte),
    };
    assert!(one(7, 'a').covers(&session));
    assert!(!one(7, 'b').covers(&session));
    assert!(!one(8, 'a').covers(&session));
    assert!(others(7, 'b').covers(&session));
    assert!(!others(7, 'a').covers(&session));
    assert!(!others(8, 'b').covers(&session));
    assert!(Revoked::All { user_id: 7 }.covers(&session));
    assert!(!Revoked::All { user_id: 8 }.covers(&session));
}

#[tokio::test]
async fn the_check_follows_the_record() {
    let (registry, pool) = registry().await;
    let live = signed_in(&registry, 9001, 0, 600).await;
    let mut check = registry.revalidation(&live);
    assert!(check.still_valid().await);
    assert!(format!("{check:?}").starts_with("SessionRevalidation { session: CurrentSession"));
    // Valid through its last second, expired after it (at fixed times: the
    // wall clock may pass a second boundary between two statements).
    let ending = signed_in(&registry, 9001, 10, 10).await;
    assert!(registry.is_live(&ending, ending.expires_at).await);
    assert!(!registry.is_live(&ending, ending.expires_at + 1).await);
    let expired = signed_in(&registry, 9001, 20, 10).await;
    assert!(!registry.revalidation(&expired).still_valid().await);
    // A record moved to another account no longer counts.
    sqlx::query("UPDATE rullst_auth_sessions SET user_id = 9002")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!check.still_valid().await);
    sqlx::query("UPDATE rullst_auth_sessions SET user_id = 9001")
        .execute(&pool)
        .await
        .unwrap();
    assert!(check.still_valid().await);
    // A failing database answers false.
    pool.close().await;
    assert!(!check.still_valid().await);
}

#[tokio::test]
async fn revocations_in_this_process_signal_the_sessions_they_end() {
    let (registry, pool) = registry().await;
    let current = signed_in(&registry, 9011, 0, 600).await;
    let sibling = signed_in(&registry, 9011, 0, 600).await;
    let stranger = signed_in(&registry, 9012, 0, 600).await;

    // Sign-out of this session.
    let mut check = registry.revalidation(&current);
    assert!(!signals(&mut check).await);
    registry.revoke(&sibling).await.unwrap();
    registry.revoke_all_sessions(9012).await.unwrap();
    assert!(!signals(&mut check).await);
    registry.revoke(&current).await.unwrap();
    assert!(signals(&mut check).await);
    assert!(!check.still_valid().await);

    // Revoking it by its reference, or every other session.
    let current = signed_in(&registry, 9011, 0, 600).await;
    let other = signed_in(&registry, 9011, 0, 600).await;
    let mut check = registry.revalidation(&current);
    registry.revoke_other_sessions(&current).await.unwrap();
    assert!(!signals(&mut check).await);
    let mut other_check = registry.revalidation(&other);
    registry.revoke_other_sessions(&current).await.unwrap();
    assert!(signals(&mut other_check).await);
    registry
        .revoke_session(9011, current.reference())
        .await
        .unwrap();
    assert!(signals(&mut check).await);

    // Every session of the account, and a sign-out by the cookie value.
    let current = signed_in(&registry, 9011, 0, 600).await;
    let mut check = registry.revalidation(&current);
    registry.revoke_all_sessions(9011).await.unwrap();
    assert!(signals(&mut check).await);
    let now = unix_now().unwrap();
    let issued = registry
        .create_session(&NewSession::new(9011, 0), KEY, now)
        .await
        .unwrap();
    let current = registry
        .authenticate(issued.token(), KEY, now)
        .await
        .unwrap()
        .into_current();
    let mut check = registry.revalidation(&current);
    registry.revoke_token(issued.token(), KEY).await.unwrap();
    assert!(signals(&mut check).await);

    // Inside a transaction.
    let current = signed_in(&registry, 9011, 0, 600).await;
    let mut check = registry.revalidation(&current);
    let mut transaction = pool.begin().await.unwrap();
    SqliteSessionRegistry::revoke_all_sessions_on(&mut transaction, 9011)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    assert!(signals(&mut check).await);

    // A check that missed notices checks at once.
    let mut check = registry.revalidation(&stranger);
    for user_id in 0..300 {
        announce(Revoked::All {
            user_id: 20_000 + user_id,
        });
    }
    assert!(signals(&mut check).await);
    assert!(!signals(&mut check).await);
}
