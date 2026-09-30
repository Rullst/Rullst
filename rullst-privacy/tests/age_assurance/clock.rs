use super::*;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::Notify;

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl AgeClock for ManualClock {
    fn now(&self) -> Result<i64, AgeError> {
        let value = self.0.load(Ordering::SeqCst);
        if value < 0 {
            Err(AgeError::InvalidChallenge)
        } else {
            Ok(value)
        }
    }
}

struct PausedAfterClaim {
    store: Arc<MemoryReplayStore>,
    claimed: Arc<Notify>,
    release: Arc<Notify>,
}

impl ReplayStore for PausedAfterClaim {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::ProcessLocal
    }

    async fn claim(&self, nonce: [u8; 32], expires: i64, now: i64) -> Result<bool, AgeError> {
        let result = self.store.claim(nonce, expires, now).await?;
        self.claimed.notify_one();
        self.release.notified().await;
        Ok(result)
    }
}

#[tokio::test]
async fn expiry_rollback_and_clock_failure_during_storage_never_return_permission() {
    for (after, expected) in [
        (1300, AgeError::Expired),
        (999, AgeError::ClockRollback),
        (-1, AgeError::InvalidChallenge),
    ] {
        let store = Arc::new(MemoryReplayStore::new(10).unwrap());
        let claimed = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let verifier = AgeVerifier::for_development(
            issuer(),
            PausedAfterClaim {
                store: store.clone(),
                claimed: claimed.clone(),
                release: release.clone(),
            },
        );
        let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
        let in_flight = challenge.clone();
        let clock = ManualClock(Arc::new(AtomicI64::new(1001)));
        let task_clock = clock.clone();
        let task = tokio::spawn(async move { assess(&verifier, &in_flight, &task_clock).await });
        tokio::time::timeout(std::time::Duration::from_secs(5), claimed.notified())
            .await
            .unwrap();
        clock.0.store(after, Ordering::SeqCst);
        release.notify_one();
        assert_eq!(task.await.unwrap(), Err(expected));
        // A failure after consumption does not resurrect the proof.
        let retry = AgeVerifier::for_development(issuer(), store);
        assert_eq!(
            assess(&retry, &challenge, &FixedClock(1001)).await,
            Err(AgeError::Replay)
        );
    }
}

#[tokio::test]
async fn cancellation_after_consumption_cannot_yield_an_assessment_or_replay() {
    let store = Arc::new(MemoryReplayStore::new(10).unwrap());
    let claimed = Arc::new(Notify::new());
    let verifier = AgeVerifier::for_development(
        issuer(),
        PausedAfterClaim {
            store: store.clone(),
            claimed: claimed.clone(),
            release: Arc::new(Notify::new()),
        },
    );
    let challenge = Arc::new(challenge(AgeMethod::VerifiedAttribute));
    let in_flight = challenge.clone();
    let task = tokio::spawn(async move { assess(&verifier, &in_flight, &FixedClock(1001)).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), claimed.notified())
        .await
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let retry = AgeVerifier::for_development(issuer(), store);
    assert_eq!(
        assess(&retry, &challenge, &FixedClock(1001)).await,
        Err(AgeError::Replay)
    );
}

#[tokio::test]
async fn default_verification_uses_server_time_and_memory_rejects_clock_rollback() {
    let now = SystemAgeClock.now().unwrap();
    let challenge =
        AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, now).unwrap();
    let (payload, signature) = signed(&challenge, AgeOutcome::MeetsThreshold);
    assert_eq!(
        verifier()
            .verify(&policy(), &binding(), &challenge, &payload, &signature)
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
    let store = MemoryReplayStore::new(1).unwrap();
    assert!(store.claim([1; 32], 20, 10).await.unwrap());
    assert!(store.claim([2; 32], 30, 20).await.unwrap());
    assert_eq!(
        store.claim([1; 32], 20, 10).await,
        Err(AgeError::ClockRollback)
    );
    for (expires, now) in [(20, -1), (20, 20), (20, 21), (1000, 10), (i64::MAX, 0)] {
        assert_eq!(
            store.claim([3; 32], expires, now).await,
            Err(AgeError::InvalidChallenge)
        );
    }
}

/// Samples `stale` first, as a request that waited for the store's lock after
/// reading its time just before a second boundary, then `current`.
struct SampledBeforeLockWait {
    stale: i64,
    current: i64,
    sampled: std::sync::atomic::AtomicBool,
}

impl AgeClock for SampledBeforeLockWait {
    fn now(&self) -> Result<i64, AgeError> {
        Ok(if self.sampled.swap(true, Ordering::SeqCst) {
            self.current
        } else {
            self.stale
        })
    }
}

#[tokio::test]
async fn a_claim_that_loses_the_lock_to_a_later_second_is_retried_at_fresh_time() {
    let store = Arc::new(MemoryReplayStore::new(10).unwrap());
    // Another verifier sampled 1002 and committed its claim first.
    assert!(store.claim([9; 32], 1100, 1002).await.unwrap());
    let verifier = AgeVerifier::for_development(issuer(), store.clone());
    let late = SampledBeforeLockWait {
        stale: 1001,
        current: 1002,
        sampled: Default::default(),
    };
    let challenge = challenge(AgeMethod::VerifiedAttribute);
    assert_eq!(
        assess(&verifier, &challenge, &late)
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
    assert_eq!(
        assess(&verifier, &challenge, &FixedClock(1002)).await,
        Err(AgeError::Replay)
    );
    // Without an advancing clock the rollback check still fails closed, and
    // the rejected proof was not consumed.
    let other =
        AgeChallenge::issue(&policy(), binding(), AgeMethod::VerifiedAttribute, 1000).unwrap();
    assert_eq!(
        assess(&verifier, &other, &FixedClock(1001)).await,
        Err(AgeError::ClockRollback)
    );
    assert_eq!(
        assess(&verifier, &other, &FixedClock(1002))
            .await
            .unwrap()
            .decision(),
        AgeDecision::Allowed
    );
}
