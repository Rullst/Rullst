#![cfg(feature = "consent")]

use rullst_privacy::consent::*;
#[path = "consent/adapter_failures.rs"]
mod adapter_failures;
#[cfg(feature = "consent-postgres")]
#[path = "consent/postgres/mod.rs"]
mod postgres;
#[cfg(feature = "consent-sqlite")]
#[path = "consent/sqlite.rs"]
mod sqlite;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

struct Clock(AtomicI64);
impl Clock {
    fn at(now: i64) -> Self {
        Self(AtomicI64::new(now))
    }
    fn set(&self, now: i64) {
        self.0.store(now, Ordering::SeqCst);
    }
}
impl ConsentClock for Clock {
    fn now(&self) -> Result<i64, ConsentError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

fn subject() -> ConsentSubject {
    ConsentSubject::new("user-1", "school-1").unwrap()
}
fn purpose() -> ConsentPurpose {
    ConsentPurpose::new("optional-digest", "notice-v1").unwrap()
}
fn submission(revision: u64, choice: ConsentChoice) -> ConsentSubmission {
    ConsentSubmission::new(purpose(), revision, choice).unwrap()
}
fn store() -> Arc<MemoryConsentStore> {
    Arc::new(MemoryConsentStore::new(8).unwrap())
}

#[test]
fn invalid_configuration_and_production_memory_state_are_rejected() {
    for value in ["", "email@example.invalid", "a/b", "ç", &"a".repeat(129)] {
        assert!(ConsentSubject::new(value, "tenant").is_err());
        assert!(ConsentSubject::new("subject", value).is_err());
        assert!(ConsentPurpose::new(value, "v1").is_err());
        assert!(ConsentPurpose::new("purpose", value).is_err());
    }
    for capacity in [0, 100_001, usize::MAX] {
        assert!(MemoryConsentStore::new(capacity).is_err());
    }
    assert!(matches!(
        ConsentGate::new(store()),
        Err(ConsentError::DurableStateRequired)
    ));
    assert!(ConsentSubmission::new(purpose(), u64::MAX, ConsentChoice::Granted).is_err());
    for choice in [ConsentChoice::Unset, ConsentChoice::Withdrawn] {
        assert!(ConsentSubmission::new(purpose(), 0, choice).is_err());
    }
}

#[tokio::test]
async fn absent_refused_and_withdrawn_choices_never_allow_processing() {
    let gate = ConsentGate::for_development(store());
    let clock = Clock::at(1000);
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    let initial = gate
        .current_with_clock(&subject(), &purpose(), &clock)
        .await
        .unwrap();
    assert_eq!(initial.revision(), 0);
    assert_eq!(initial.choice(), ConsentChoice::Unset);
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Declined),
        0,
        &clock,
    )
    .await
    .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    let granted = gate
        .choose_with_clock(
            &subject(),
            &purpose(),
            &submission(1, ConsentChoice::Granted),
            1100,
            &clock,
        )
        .await
        .unwrap();
    assert_eq!(granted.revision(), 2);
    assert!(
        gate.allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    let withdrawn = gate
        .withdraw_with_clock(&subject(), &purpose(), &clock)
        .await
        .unwrap();
    assert_eq!(withdrawn.choice(), ConsentChoice::Withdrawn);
    assert_eq!(withdrawn.revision(), 3);
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    assert!(matches!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(2, ConsentChoice::Granted),
            1100,
            &clock
        )
        .await,
        Err(ConsentError::RevisionConflict)
    ));
    // A fresh, explicit choice after withdrawal can grant again.
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(3, ConsentChoice::Granted),
        1100,
        &clock,
    )
    .await
    .unwrap();
    assert!(
        gate.allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn policy_versions_subjects_tenants_and_purposes_remain_isolated() {
    let gate = ConsentGate::for_development(store());
    let clock = Clock::at(1000);
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &clock,
    )
    .await
    .unwrap();
    for other in [
        ConsentSubject::new("user-2", "school-1").unwrap(),
        ConsentSubject::new("user-1", "school-2").unwrap(),
    ] {
        assert!(
            !gate
                .allows_with_clock(&other, &purpose(), &clock)
                .await
                .unwrap()
        );
    }
    for other in [
        ConsentPurpose::new("other", "notice-v1").unwrap(),
        ConsentPurpose::new("optional-digest", "notice-v2").unwrap(),
    ] {
        assert!(
            !gate
                .allows_with_clock(&subject(), &other, &clock)
                .await
                .unwrap()
        );
        // Even an unchanged revision cannot transfer an old displayed notice.
        assert!(matches!(
            gate.choose_with_clock(
                &subject(),
                &other,
                &submission(1, ConsentChoice::Granted),
                1100,
                &clock
            )
            .await,
            Err(ConsentError::BindingMismatch)
        ));
    }
    let updated = ConsentPurpose::new("optional-digest", "notice-v2").unwrap();
    let response = ConsentSubmission::new(updated.clone(), 1, ConsentChoice::Granted).unwrap();
    gate.choose_with_clock(&subject(), &updated, &response, 1100, &clock)
        .await
        .unwrap();
    // A withdrawal from an older notice still stops that same optional purpose.
    gate.withdraw_with_clock(&subject(), &purpose(), &clock)
        .await
        .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &updated, &clock)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn concurrent_old_forms_cannot_override_a_completed_withdrawal() {
    let shared = store();
    let gate = Arc::new(ConsentGate::for_development(shared));
    let clock = Clock::at(1000);
    gate.withdraw_with_clock(&subject(), &purpose(), &clock)
        .await
        .unwrap();
    let mut jobs = Vec::new();
    for _ in 0..20 {
        let gate = gate.clone();
        jobs.push(tokio::spawn(async move {
            gate.choose_with_clock(
                &subject(),
                &purpose(),
                &submission(0, ConsentChoice::Granted),
                1100,
                &Clock::at(1000),
            )
            .await
        }));
    }
    for job in jobs {
        assert!(matches!(
            job.await.unwrap(),
            Err(ConsentError::RevisionConflict)
        ));
    }
    assert_eq!(
        gate.current_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
            .revision(),
        1
    );
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn expiry_and_clock_rollback_fail_closed_without_eviction() {
    let shared = Arc::new(MemoryConsentStore::new(1).unwrap());
    let gate = ConsentGate::for_development(shared.clone());
    let clock = Clock::at(1000);
    for expiry in [-1, 0, 1000, 31_537_001, i64::MAX] {
        assert!(
            gate.choose_with_clock(
                &subject(),
                &purpose(),
                &submission(0, ConsentChoice::Granted),
                expiry,
                &clock
            )
            .await
            .is_err()
        );
    }
    assert!(
        gate.choose_with_clock(
            &subject(),
            &purpose(),
            &submission(0, ConsentChoice::Declined),
            1100,
            &clock
        )
        .await
        .is_err()
    );
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &clock,
    )
    .await
    .unwrap();
    clock.set(1100);
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
    );
    clock.set(1099);
    assert!(matches!(
        gate.allows_with_clock(&subject(), &purpose(), &clock).await,
        Err(ConsentError::ClockRollback)
    ));
    clock.set(1200);
    let other = ConsentSubject::new("other", "school-1").unwrap();
    assert!(matches!(
        gate.withdraw_with_clock(&other, &purpose(), &clock).await,
        Err(ConsentError::StoreCapacity)
    ));
    gate.withdraw_with_clock(&subject(), &purpose(), &clock)
        .await
        .unwrap();
    assert_eq!(
        gate.current_with_clock(&subject(), &purpose(), &clock)
            .await
            .unwrap()
            .revision(),
        2
    );
    assert!(shared.read(&subject(), purpose().id(), -1).await.is_err());
}

struct SteppingClock(AtomicI64);
impl ConsentClock for SteppingClock {
    fn now(&self) -> Result<i64, ConsentError> {
        Ok(self.0.fetch_add(100, Ordering::SeqCst))
    }
}

#[tokio::test]
async fn expiry_observed_after_storage_cannot_be_revived_by_a_later_rollback() {
    let shared = store();
    let gate = ConsentGate::for_development(shared);
    gate.choose_with_clock(
        &subject(),
        &purpose(),
        &submission(0, ConsentChoice::Granted),
        1100,
        &Clock::at(1000),
    )
    .await
    .unwrap();
    assert!(
        !gate
            .allows_with_clock(&subject(), &purpose(), &SteppingClock(AtomicI64::new(1000)))
            .await
            .unwrap()
    );
    assert!(matches!(
        gate.allows_with_clock(&subject(), &purpose(), &Clock::at(1050))
            .await,
        Err(ConsentError::ClockRollback)
    ));
}

#[test]
fn restored_records_are_bounded_and_debug_omits_subjects() {
    for (revision, choice, changed, expiry) in [
        (0, ConsentChoice::Granted, 1, 2),
        (u64::MAX, ConsentChoice::Granted, 1, 2),
        (1, ConsentChoice::Unset, 1, 0),
        (1, ConsentChoice::Granted, -1, 2),
        (1, ConsentChoice::Withdrawn, 1, 2),
        (1, ConsentChoice::Granted, 1, i64::MAX),
    ] {
        assert!(
            ConsentRecord::from_stored(subject(), purpose(), revision, choice, changed, expiry)
                .is_err()
        );
    }
    let record =
        ConsentRecord::from_stored(subject(), purpose(), 1, ConsentChoice::Granted, 1, 2).unwrap();
    assert!(!format!("{record:?} {:?}", record.subject()).contains("user-1"));
    assert_eq!(record.subject().tenant_ref(), "school-1");
    assert_eq!(record.subject().subject_ref(), "user-1");
}
