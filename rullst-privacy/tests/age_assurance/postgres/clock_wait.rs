use super::*;
use std::sync::atomic::{AtomicI64, Ordering};

struct MovingClock(Arc<AtomicI64>);

impl AgeClock for MovingClock {
    fn now(&self) -> Result<i64, AgeError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

pub(super) async fn recheck_after_locked_storage(url: &str, database: &mut PgConnection) {
    reset(database).await;
    let store = PostgresReplayStore::initialize(url, 10).await.unwrap();
    for (after, expected) in [(1300, AgeError::Expired), (1000, AgeError::ClockRollback)] {
        sqlx::query("BEGIN").execute(&mut *database).await.unwrap();
        sqlx::query("SELECT id FROM rullst_age_replay.metadata FOR UPDATE")
            .execute(&mut *database)
            .await
            .unwrap();
        let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
        let verifier = AgeVerifier::new(issuer(), store.clone()).unwrap();
        let now = Arc::new(AtomicI64::new(1001));
        let task_clock = MovingClock(now.clone());
        let task_challenge = challenge.clone();
        let task =
            tokio::spawn(async move { assess(&verifier, &task_challenge, &task_clock).await });
        let mut waiting = false;
        for _ in 0..50 {
            // A fresh statement snapshot sees the blocked verifier connection.
            waiting = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name = 'rullst-age-replay-v1' AND wait_event_type = 'Lock')")
                .fetch_one(&mut *database).await.unwrap();
            if waiting {
                break;
            }
            sqlx::query("SELECT pg_stat_clear_snapshot()")
                .execute(&mut *database)
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            waiting,
            "the verifier must actually wait on the database row lock"
        );
        now.store(after, Ordering::SeqCst);
        sqlx::query("ROLLBACK")
            .execute(&mut *database)
            .await
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(10), task)
                .await
                .unwrap()
                .unwrap(),
            Err(expected)
        );
        let raw: serde_json::Value =
            serde_json::from_slice(&challenge.request_json().unwrap()).unwrap();
        let consumed: [u8; 32] = serde_json::from_value(raw["nonce"].clone()).unwrap();
        assert!(
            !store.claim(consumed, 1300, 1001).await.unwrap(),
            "a denied post-storage result can already have consumed the nonce"
        );
    }
    store.close().await;
}
