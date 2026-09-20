use super::*;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::{path::Path, time::Duration};
use tokio::sync::Barrier;

async fn connection(path: &Path) -> SqliteConnection {
    SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(path))
        .await
        .unwrap()
}

#[tokio::test]
async fn consumed_signed_proof_survives_pool_shutdown_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), store.clone()).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001))
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
    store.close().await;
    let reopened = SqliteReplayStore::open(&path, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), reopened.clone()).unwrap();
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
    reopened.close().await;
}

#[tokio::test]
async fn a_fresh_process_observes_a_previously_consumed_nonce() {
    const FIXTURE: &str = "RULLST_AGE_REPLAY_CHILD_FIXTURE";
    if let Some(path) = std::env::var_os(FIXTURE) {
        let store = SqliteReplayStore::open(Path::new(&path), 1).await.unwrap();
        assert!(!store.claim([17; 32], 20, 10).await.unwrap());
        store.close().await;
        println!("age replay child verified persisted consumption");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 1).await.unwrap();
    assert!(store.claim([17; 32], 20, 10).await.unwrap());
    store.close().await;
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "sqlite::a_fresh_process_observes_a_previously_consumed_nonce",
            "--nocapture",
        ])
        .env(FIXTURE, &path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("age replay child exceeded its bounded runtime");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("age replay child verified persisted consumption")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn independent_pools_and_concurrent_verifiers_allow_one_consumption() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let first = SqliteReplayStore::open(&path, 10).await.unwrap();
    let second = SqliteReplayStore::open(&path, 10).await.unwrap();
    let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
    let barrier = Arc::new(Barrier::new(8));
    let mut tasks = Vec::new();
    for index in 0..8 {
        let verifier = AgeVerifier::new(
            issuer(),
            if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            },
        )
        .unwrap();
        let challenge = challenge.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            assess(&verifier, &challenge, &FixedClock(1001)).await
        }));
    }
    let mut allowed = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(assessment) => {
                assert_eq!(assessment.decision(), AgeDecision::Allowed);
                allowed += 1;
            }
            Err(error) => assert_eq!(error, AgeError::Replay),
        }
    }
    assert_eq!(allowed, 1);
    first.close().await;
    second.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_unique_claims_cannot_exceed_the_persisted_quota() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let first = SqliteReplayStore::open(&path, 1).await.unwrap();
    let second = SqliteReplayStore::open(&path, 1).await.unwrap();
    let (a, b) = tokio::join!(first.claim([1; 32], 20, 10), second.claim([2; 32], 20, 10));
    assert!(matches!(
        (a, b),
        (Ok(true), Err(AgeError::StoreCapacity)) | (Err(AgeError::StoreCapacity), Ok(true))
    ));
    let winner = if a.is_ok() { [1; 32] } else { [2; 32] };
    assert!(!second.claim(winner, 20, 10).await.unwrap());
    assert!(second.claim([3; 32], 30, 20).await.unwrap());
    first.close().await;
    second.close().await;
    let reopened = SqliteReplayStore::open(&path, 1).await.unwrap();
    assert_eq!(
        reopened.claim(winner, 20, 10).await,
        Err(AgeError::ClockRollback)
    );
    assert_eq!(
        reopened.claim([4; 32], 40, 20).await,
        Err(AgeError::StoreCapacity)
    );
    reopened.close().await;
}

#[tokio::test]
async fn configuration_schema_and_missing_state_are_never_silently_reset() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    for capacity in [0, 100_001] {
        assert!(matches!(
            SqliteReplayStore::open(&path, capacity).await,
            Err(AgeError::InvalidConfiguration)
        ));
    }
    assert!(matches!(
        SqliteReplayStore::open(":memory:", 1).await,
        Err(AgeError::InvalidConfiguration)
    ));
    assert!(matches!(
        SqliteReplayStore::open(directory.path(), 1).await,
        Err(AgeError::InvalidConfiguration)
    ));
    let store = SqliteReplayStore::open(&path, 1).await.unwrap();
    assert!(store.claim([1; 32], 20, 10).await.unwrap());
    assert!(matches!(
        SqliteReplayStore::open(&path, 2).await,
        Err(AgeError::StoreConfiguration)
    ));
    let mut database = connection(&path).await;
    sqlx::query("UPDATE rullst_age_replay_meta SET schema_version = 99")
        .execute(&mut database)
        .await
        .unwrap();
    assert_eq!(
        store.claim([2; 32], 20, 10).await,
        Err(AgeError::StoreConfiguration)
    );
    sqlx::query("DELETE FROM rullst_age_replay_meta")
        .execute(&mut database)
        .await
        .unwrap();
    assert_eq!(
        store.claim([2; 32], 20, 10).await,
        Err(AgeError::StoreConfiguration)
    );
    store.close().await;
    assert!(matches!(
        SqliteReplayStore::open(&path, 1).await,
        Err(AgeError::StoreConfiguration)
    ));
    sqlx::query("DROP TABLE rullst_age_replay_claims")
        .execute(&mut database)
        .await
        .unwrap();
    assert!(matches!(
        SqliteReplayStore::open(&path, 1).await,
        Err(AgeError::StoreConfiguration)
    ));
    database.close().await.unwrap();
}

#[tokio::test]
async fn closure_corruption_and_failed_inserts_fail_closed_without_partial_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 1).await.unwrap();
    let mut database = connection(&path).await;
    sqlx::query("CREATE TRIGGER fail_claim BEFORE INSERT ON rullst_age_replay_claims BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END")
        .execute(&mut database).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), store.clone()).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::StoreUnavailable)
    );
    let last: i64 = sqlx::query_scalar("SELECT last_now FROM rullst_age_replay_meta")
        .fetch_one(&mut database)
        .await
        .unwrap();
    assert_eq!(
        last, 0,
        "the failed insert rolled back the same transaction's clock update"
    );
    sqlx::query("DROP TRIGGER fail_claim")
        .execute(&mut database)
        .await
        .unwrap();
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001))
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
    store.close().await;
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::StoreUnavailable)
    );
    database.close().await.unwrap();
    let corrupt = directory.path().join("corrupt.sqlite");
    std::fs::write(&corrupt, b"not a SQLite database").unwrap();
    assert!(matches!(
        SqliteReplayStore::open(&corrupt, 1).await,
        Err(AgeError::StoreUnavailable)
    ));
}

#[tokio::test]
async fn cancelling_a_blocked_transaction_does_not_leave_a_live_claim_or_lock() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 1).await.unwrap();
    let mut database = connection(&path).await;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut database)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), store.claim([1; 32], 20, 10))
            .await
            .is_err()
    );
    sqlx::query("ROLLBACK")
        .execute(&mut database)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(6), store.claim([1; 32], 20, 10))
            .await
            .unwrap()
            .unwrap()
    );
    store.close().await;
    database.close().await.unwrap();
}

// A fault-injection adapter around the real persistent store. It deliberately
// loses the commit acknowledgement; it is not a separate production backend.
struct LostAcknowledgement(SqliteReplayStore);
impl ReplayStore for LostAcknowledgement {
    fn durability(&self) -> ReplayDurability {
        self.0.durability()
    }
    async fn claim(&self, nonce: [u8; 32], expires: i64, now: i64) -> Result<bool, AgeError> {
        self.0.claim(nonce, expires, now).await?;
        Err(AgeError::StoreUnavailable)
    }
}

#[tokio::test]
async fn uncertain_commit_never_returns_permission_and_remains_consumed_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), LostAcknowledgement(store.clone())).unwrap();
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::StoreUnavailable)
    );
    store.close().await;
    let reopened = SqliteReplayStore::open(&path, 10).await.unwrap();
    let verifier = AgeVerifier::new(issuer(), reopened.clone()).unwrap();
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
    reopened.close().await;
}

#[tokio::test]
async fn storage_contains_only_nonce_digests_expiry_and_bounded_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.sqlite");
    let store = SqliteReplayStore::open(&path, 1).await.unwrap();
    assert!(store.claim([7; 32], 20, 10).await.unwrap());
    let mut database = connection(&path).await;
    let (digest, expiry): (Vec<u8>, i64) =
        sqlx::query_as("SELECT nonce_digest, expires_at FROM rullst_age_replay_claims")
            .fetch_one(&mut database)
            .await
            .unwrap();
    assert_eq!(digest.len(), 32);
    assert_ne!(digest, vec![7; 32]);
    assert_eq!(expiry, 20);
    for (expires, now) in [(10, 10), (1000, 10), (20, -1), (i64::MAX, 0)] {
        assert_eq!(
            store.claim([8; 32], expires, now).await,
            Err(AgeError::InvalidChallenge)
        );
    }
    store.close().await;
    database.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn existing_symlinks_are_rejected_without_modifying_the_target() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target");
    let link = directory.path().join("link");
    std::fs::write(&target, b"preserve this file").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(matches!(
        SqliteReplayStore::open(&link, 1).await,
        Err(AgeError::InvalidConfiguration)
    ));
    assert_eq!(std::fs::read(&target).unwrap(), b"preserve this file");
}
