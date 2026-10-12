//! Reset windows, notice lifetimes and the shared request budget.
use super::*;
use crate::recovery::RecoverySecrets;

const T: u64 = 1_800_000_000;
const EMAIL: &str = "learner@example.com";

async fn store() -> SqlRecoveryStore {
    let keys = RecoverySecrets::new([3; 32], [9; 32]).unwrap();
    let store = SqlRecoveryStore::connect("sqlite::memory:", keys)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .register_account("learner-1", EMAIL, "Fixture_password_1", T)
        .await
        .unwrap();
    store
}

async fn token_expiry(store: &SqlRecoveryStore) -> i64 {
    sqlx::query_scalar("SELECT expires_at FROM rullst_recovery_tokens")
        .fetch_one(&store.pool)
        .await
        .unwrap()
}

/// Claims due notices until one of `kind` appears.
async fn claim(
    store: &SqlRecoveryStore,
    kind: RecoveryNoticeKind,
    now: u64,
) -> super::super::ClaimedRecoveryNotice {
    loop {
        let claimed = store
            .claim_notice(now)
            .await
            .unwrap()
            .expect("a due notice");
        if claimed.notice().kind() == kind {
            return claimed;
        }
        store.complete_notice(&claimed, now).await.unwrap();
    }
}

#[tokio::test]
async fn an_account_gets_three_reset_links_per_900_second_window() {
    let store = store().await;
    for (offset, expected) in [
        (0, 1_200),
        (1, 1_201),
        (2, 1_202),
        (899, 1_202),
        (900, 2_100),
    ] {
        store
            .request_reset_inner(EMAIL.to_owned(), T + offset)
            .await
            .unwrap();
        assert_eq!(
            token_expiry(&store).await,
            (T + expected) as i64,
            "+{offset}"
        );
    }
    store.close().await;
}

#[tokio::test]
async fn reset_and_change_notices_carry_their_lifetimes_and_recipient() {
    let store = store().await;
    store
        .request_reset_inner(EMAIL.to_owned(), T + 5)
        .await
        .unwrap();
    let reset = claim(&store, RecoveryNoticeKind::PasswordReset, T + 5).await;
    assert_eq!(reset.notice().expires_at(), (T + 5 + 1_200) as i64);
    assert_eq!(reset.notice().recipient(), EMAIL);
    let token = reset.notice().token().unwrap().expose().to_owned();
    store.complete_notice(&reset, T + 5).await.unwrap();
    store
        .complete_password_reset(&token, "Changed_password_2", T + 10)
        .await
        .unwrap();
    let changed = claim(&store, RecoveryNoticeKind::PasswordChanged, T + 10).await;
    assert_eq!(changed.notice().expires_at(), (T + 10 + 86_400) as i64);
    assert_eq!(changed.notice().recipient(), EMAIL);
    store.close().await;
}

#[tokio::test]
async fn the_shared_request_budget_admits_120_per_minute() {
    let store = store().await;
    let start = T as i64;
    sqlx::query(
        "UPDATE rullst_recovery_control SET window_start = $1, attempts = 119 WHERE id = 'request'",
    )
    .bind(start)
    .execute(&store.pool)
    .await
    .unwrap();
    let mut tx = store.pool.begin().await.unwrap();
    let mut take = async |now: i64| take_limit(&mut tx, "request", now, 120).await.unwrap();
    assert!(take(start).await, "the 120th request");
    assert!(!take(start).await, "the 121st request");
    assert!(!take(start + 59).await);
    assert!(take(start + 60).await, "a new window");
    assert!(take(start + 60).await, "a request at the window start");
    assert!(!take(start + 59).await, "time before the window");
    drop(take);
    let row: (i64, i64) = sqlx::query_as(
        "SELECT window_start, attempts FROM rullst_recovery_control WHERE id = 'request'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(row, (start + 60, 2));
    tx.rollback().await.unwrap();
    store.close().await;
}
