use super::*;
use std::sync::atomic::AtomicBool;

enum Fault {
    WrongSubject,
    Downgrade,
    Failure,
    WrongAcknowledgment,
}
struct FaultyStore {
    fault: Fault,
    local: AtomicBool,
}
impl ConsentStore for FaultyStore {
    fn durability(&self) -> ConsentDurability {
        if self.local.load(Ordering::SeqCst) {
            ConsentDurability::ProcessLocal
        } else {
            ConsentDurability::SharedDurable
        }
    }
    async fn read(
        &self,
        who: &ConsentSubject,
        why: &str,
        now: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        if matches!(self.fault, Fault::Failure) {
            return Err(ConsentError::StoreUnavailable);
        }
        if matches!(self.fault, Fault::Downgrade) {
            self.local.store(true, Ordering::SeqCst);
        }
        let who = if matches!(self.fault, Fault::WrongSubject) {
            ConsentSubject::new("other", "school-1").unwrap()
        } else {
            who.clone()
        };
        ConsentRecord::from_stored(
            who,
            ConsentPurpose::new(why, "notice-v1").unwrap(),
            1,
            ConsentChoice::Granted,
            now,
            now + 100,
        )
    }
    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        if matches!(self.fault, Fault::WrongAcknowledgment) {
            return ConsentRecord::from_stored(
                update.subject().clone(),
                update.purpose().clone(),
                7,
                ConsentChoice::Granted,
                update.now(),
                update.now() + 100,
            );
        }
        self.read(update.subject(), update.purpose().id(), update.now())
            .await
    }
}

#[tokio::test]
async fn faulty_or_downgraded_adapters_never_allow_processing() {
    for (fault, error) in [
        (Fault::WrongSubject, ConsentError::BindingMismatch),
        (Fault::Downgrade, ConsentError::DurableStateRequired),
        (Fault::Failure, ConsentError::StoreUnavailable),
    ] {
        let gate = ConsentGate::new(FaultyStore {
            fault,
            local: AtomicBool::new(false),
        })
        .unwrap();
        assert_eq!(
            gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1000))
                .await,
            Err(error)
        );
    }
    let gate = ConsentGate::new(FaultyStore {
        fault: Fault::WrongAcknowledgment,
        local: AtomicBool::new(false),
    })
    .unwrap();
    assert_eq!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            1100,
            &Clock::at(1000)
        )
        .await,
        Err(ConsentError::StoreConfiguration)
    );
    assert_eq!(
        gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Err(ConsentError::StoreConfiguration)
    );
}

struct Backwards(AtomicI64);
impl ConsentClock for Backwards {
    fn now(&self) -> Result<i64, ConsentError> {
        Ok(self.0.fetch_sub(1, Ordering::SeqCst))
    }
}

#[tokio::test]
async fn clock_changes_across_storage_fail_without_acknowledging_or_reviving_expired_grants() {
    let shared = store();
    let gate = ConsentGate::for_development(shared);
    assert_eq!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            1100,
            &Backwards(AtomicI64::new(1000))
        )
        .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Backwards(AtomicI64::new(1000)))
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(1, ConsentChoice::Granted),
            1100,
            &SteppingClock(AtomicI64::new(1000))
        )
        .await
        .is_err()
    );
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1050))
            .await,
        Err(ConsentError::ClockRollback)
    );
}

/// A request that sampled `stale` just before a second boundary and then waited
/// for the store's lock; the trusted clock reads `current` afterwards.
struct SampledBeforeLockWait {
    stale: i64,
    current: i64,
    sampled: AtomicBool,
}
impl SampledBeforeLockWait {
    fn new(stale: i64, current: i64) -> Self {
        Self {
            stale,
            current,
            sampled: AtomicBool::new(false),
        }
    }
}
impl ConsentClock for SampledBeforeLockWait {
    fn now(&self) -> Result<i64, ConsentError> {
        Ok(if self.sampled.swap(true, Ordering::SeqCst) {
            self.current
        } else {
            self.stale
        })
    }
}

#[tokio::test]
async fn losing_the_lock_to_a_later_second_is_not_a_clock_rollback() {
    let shared = store();
    let gate = ConsentGate::for_development(shared.clone());
    let other = ConsentSubject::new("other", "school-1").unwrap();
    // Another request sampled 1001 and committed its observation first.
    gate.current_with_clock(&other, &purpose(), &Clock::at(1001))
        .await
        .unwrap();
    let granted = gate
        .choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            2000,
            &SampledBeforeLockWait::new(1000, 1001),
        )
        .await
        .unwrap();
    assert_eq!(granted.changed_at(), 1001);
    gate.current_with_clock(&other, &purpose(), &Clock::at(1002))
        .await
        .unwrap();
    assert!(
        gate.allows_with_clock(
            &subject(),
            &purpose(),
            &SampledBeforeLockWait::new(1001, 1002)
        )
        .await
        .unwrap()
    );
    gate.current_with_clock(&other, &purpose(), &Clock::at(1003))
        .await
        .unwrap();
    let withdrawn = gate
        .withdraw_with_clock(
            &subject(),
            &purpose(),
            &SampledBeforeLockWait::new(1002, 1003),
        )
        .await
        .unwrap();
    assert_eq!(withdrawn.choice(), ConsentChoice::Withdrawn);
    assert_eq!(withdrawn.changed_at(), 1003);
    // A clock that has not advanced past the high-water mark still fails closed.
    assert_eq!(
        gate.current_with_clock(&subject(), &purpose(), &Clock::at(1002))
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(
        gate.withdraw_with_clock(
            &subject(),
            &purpose(),
            &SampledBeforeLockWait::new(1001, 1002)
        )
        .await,
        Err(ConsentError::ClockRollback)
    );
}

struct AlwaysBehind(AtomicI64);
impl ConsentStore for AlwaysBehind {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::SharedDurable
    }
    async fn read(
        &self,
        _: &ConsentSubject,
        _: &str,
        _: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ConsentError::ClockRollback)
    }
    async fn update(&self, _: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ConsentError::ClockRollback)
    }
}

#[tokio::test]
async fn clock_race_retries_are_bounded() {
    let attempts = Arc::new(AlwaysBehind(AtomicI64::new(0)));
    let gate = ConsentGate::new(attempts.clone()).unwrap();
    let advancing = SteppingClock(AtomicI64::new(1000));
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &advancing)
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(attempts.0.load(Ordering::SeqCst), 3);
    assert_eq!(
        gate.withdraw_with_clock(&subject(), &purpose(), &advancing)
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(attempts.0.load(Ordering::SeqCst), 6);
}
