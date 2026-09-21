use super::{account, connect};
use crate::recovery::RecoveryError;
use std::time::{Duration, Instant};

#[tokio::test]
#[ignore = "requires the owned PostgreSQL fixture"]
async fn contention_and_locked_database_preserve_revocation_and_deadlines() {
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL").unwrap();
    let a = connect(&url).await;
    let b = connect(&url).await;
    let owner = account(&a, "postgres-contention").await;
    let one = a.create_session(&owner, 1000, 600).await.unwrap();
    let two = a.create_session(&owner, 1000, 600).await.unwrap();
    let (first, second) = tokio::join!(
        a.revoke_other_sessions(one.expose(), 1001),
        b.revoke_other_sessions(two.expose(), 1001),
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(matches!(first, Ok(1) | Err(RecoveryError::InvalidAction)));
    assert!(matches!(second, Ok(1) | Err(RecoveryError::InvalidAction)));
    let (retained, revoked) = if first.is_ok() {
        (&one, &two)
    } else {
        (&two, &one)
    };
    assert_eq!(
        a.verify_session(revoked.expose(), 1001).await.unwrap(),
        None
    );
    assert_eq!(
        b.active_sessions(retained.expose(), 1001)
            .await
            .unwrap()
            .len(),
        1
    );

    let mut lock = b.pool.begin().await.unwrap();
    b.lock_writes(&mut lock).await.unwrap();
    let started = Instant::now();
    let outcome = tokio::time::timeout(
        Duration::from_secs(20),
        a.active_sessions(retained.expose(), 1001),
    )
    .await;
    lock.rollback().await.unwrap();
    assert_eq!(outcome.unwrap(), Err(RecoveryError::Storage));
    assert!(started.elapsed() >= Duration::from_secs(9));
    assert!(started.elapsed() < Duration::from_secs(20));
    assert_eq!(
        a.active_sessions(retained.expose(), 1001)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(a.purge_expired_sessions(1600, 100).await.unwrap() >= 1);
    assert_eq!(
        b.verify_session(retained.expose(), 1600).await.unwrap(),
        None
    );
    a.close().await;
    b.close().await;
}
