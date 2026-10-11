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

/// Fails every operation as unavailable and counts the attempts.
struct Unavailable(AtomicI64);
impl ConsentStore for Unavailable {
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
        Err(ConsentError::StoreUnavailable)
    }
    async fn update(&self, _: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ConsentError::StoreUnavailable)
    }
}

#[tokio::test]
async fn only_clock_rollbacks_are_retried() {
    let attempts = Arc::new(Unavailable(AtomicI64::new(0)));
    let gate = ConsentGate::new(attempts.clone()).unwrap();
    let advancing = SteppingClock(AtomicI64::new(1000));
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &advancing)
            .await,
        Err(ConsentError::StoreUnavailable)
    );
    assert_eq!(
        gate.withdraw_with_clock(&subject(), &purpose(), &advancing)
            .await,
        Err(ConsentError::StoreUnavailable)
    );
    assert_eq!(attempts.0.load(Ordering::SeqCst), 2);
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

/// Accepts every operation and counts the calls that reach storage.
struct Permissive(AtomicI64);
impl ConsentStore for Permissive {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::SharedDurable
    }
    async fn read(
        &self,
        who: &ConsentSubject,
        why: &str,
        _: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        ConsentRecord::absent(who.clone(), why)
    }
    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        update.apply_to(&ConsentRecord::absent(
            update.subject().clone(),
            update.purpose().id(),
        )?)
    }
}

#[tokio::test]
async fn negative_times_and_invalid_expiries_never_reach_storage() {
    let calls = Arc::new(Permissive(AtomicI64::new(0)));
    let gate = ConsentGate::new(calls.clone()).unwrap();
    assert_eq!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(-1))
            .await,
        Err(ConsentError::ClockRollback)
    );
    assert_eq!(
        gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(-1))
            .await,
        Err(ConsentError::InvalidConfiguration)
    );
    for (valid_until, now) in [(100, -1), (1000, 1000)] {
        assert_eq!(
            gate.choose_with_clock(
                &subject(),
                &purpose(),
                &submission(0, ConsentChoice::Granted),
                valid_until,
                &Clock::at(now)
            )
            .await,
            Err(ConsentError::InvalidConfiguration)
        );
    }
    assert_eq!(calls.0.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn the_epoch_is_a_valid_trusted_time() {
    let gate = ConsentGate::for_development(store());
    let current = gate
        .current_with_clock(&subject(), &purpose(), &Clock::at(0))
        .await
        .unwrap();
    assert_eq!(current.revision(), 0);
    let granted = gate
        .choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Granted),
            100,
            &Clock::at(0),
        )
        .await
        .unwrap();
    assert_eq!(granted.changed_at(), 0);
}

/// Acknowledges every update with a fixed record.
struct Acknowledges(ConsentRecord);
impl ConsentStore for Acknowledges {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::SharedDurable
    }
    async fn read(
        &self,
        who: &ConsentSubject,
        why: &str,
        _: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        ConsentRecord::absent(who.clone(), why)
    }
    async fn update(&self, _: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        Ok(self.0.clone())
    }
}

fn acknowledgement(
    version: &str,
    choice: ConsentChoice,
    changed_at: i64,
    valid_until: i64,
) -> ConsentRecord {
    let notice = ConsentPurpose::new("optional-digest", version).unwrap();
    ConsentRecord::from_stored(subject(), notice, 1, choice, changed_at, valid_until).unwrap()
}

#[tokio::test]
async fn an_acknowledgement_differing_in_any_one_field_is_rejected() {
    for record in [
        acknowledgement("notice-v1", ConsentChoice::Declined, 1000, 0),
        acknowledgement("notice-v2", ConsentChoice::Withdrawn, 1000, 0),
        acknowledgement("notice-v1", ConsentChoice::Withdrawn, 999, 0),
    ] {
        let gate = ConsentGate::new(Acknowledges(record)).unwrap();
        assert_eq!(
            gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1000))
                .await,
            Err(ConsentError::StoreConfiguration)
        );
    }
    let gate = ConsentGate::new(Acknowledges(acknowledgement(
        "notice-v1",
        ConsentChoice::Granted,
        1000,
        1200,
    )))
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
    let exact = acknowledgement("notice-v1", ConsentChoice::Withdrawn, 1000, 0);
    let gate = ConsentGate::new(Acknowledges(exact.clone())).unwrap();
    assert_eq!(
        gate.withdraw_with_clock(&subject(), &purpose(), &Clock::at(1000))
            .await,
        Ok(exact)
    );
}

#[tokio::test]
async fn default_clock_methods_use_current_server_time() {
    let expected = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let now = SystemConsentClock.now().unwrap();
    assert!((expected - 1..=expected + 1).contains(&now));
    let gate = ConsentGate::for_development(store());
    assert_eq!(gate.allows(&subject(), &purpose()).await, Ok(false));
    gate.choose(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        now + 3600,
    )
    .await
    .unwrap();
    assert_eq!(gate.allows(&subject(), &purpose()).await, Ok(true));
}
