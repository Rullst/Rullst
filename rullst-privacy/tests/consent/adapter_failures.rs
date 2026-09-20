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
