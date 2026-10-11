#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::{TrapKind, sqlite::CaseGrade};

const NOW: i64 = 1_800_000_000;

fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}

fn record(state: JobState) -> Record {
    Record {
        view: JobView {
            id: reference("job-1"),
            scope: Scope::new("school", "rust").unwrap(),
            learner: reference("alice"),
            exercise: ExerciseRef::new("sum", "v1").unwrap(),
            source_digest: ContentHash::of(b"source"),
            exercise_digest: ContentHash::of(b"exercise"),
            state,
            revision: 2,
            created_at: NOW,
            updated_at: NOW,
            expires_at: NOW + 300,
            cleanup_pending: false,
            result: None,
        },
        request_digest: ContentHash::of(b"request"),
        profile_digest: ContentHash::of(b"profile"),
        lease: None,
        attempts: 0,
    }
}

fn running() -> Record {
    let mut record = record(JobState::Running);
    record.lease = Some(Lease {
        nonce: reference("nonce"),
        until: NOW + 100,
        revision: 2,
    });
    record.attempts = 1;
    record
}

fn graded(cases: Vec<CaseGrade>, passed: u16, total: u16) -> JobResult {
    JobResult::Graded {
        cases,
        passed,
        total,
        artifact: ContentHash::of(b"artifact"),
        evidence: ResultEvidence::Experimental {
            receipt: ContentHash::of(b"receipt"),
            observations: ContentHash::of(b"observations"),
        },
    }
}

fn completed() -> Record {
    let mut record = record(JobState::Completed);
    record.view.result = Some(graded(vec![CaseGrade::Passed], 1, 1));
    record
}

/// Each case breaks exactly one invariant of an otherwise valid record.
#[test]
fn every_record_invariant_is_checked_on_its_own() {
    assert!(record(JobState::Queued).validate(true).is_ok());
    assert!(running().validate(true).is_ok());
    assert!(completed().validate(false).is_ok());
    let mut longest = record(JobState::Queued);
    longest.view.expires_at = NOW + 86_400;
    assert!(longest.validate(true).is_ok());
    let mut lease_at_expiry = running();
    lease_at_expiry.lease.as_mut().unwrap().until = NOW + 300;
    assert!(lease_at_expiry.validate(true).is_ok());

    let mut broken: Vec<(Record, bool)> = Vec::new();
    let mut too_many_attempts = record(JobState::Queued);
    too_many_attempts.attempts = 3;
    broken.push((too_many_attempts, true));
    let mut unattempted_lease = running();
    unattempted_lease.attempts = 0;
    broken.push((unattempted_lease, true));
    let mut missing_result = completed();
    missing_result.view.result = None;
    broken.push((missing_result, false));
    broken.push((completed(), true));
    let mut updated_before_creation = record(JobState::Queued);
    updated_before_creation.view.updated_at = NOW - 1;
    broken.push((updated_before_creation, true));
    let mut expires_at_creation = record(JobState::Queued);
    expires_at_creation.view.expires_at = NOW;
    broken.push((expires_at_creation, true));
    let mut lives_too_long = record(JobState::Queued);
    lives_too_long.view.expires_at = NOW + 86_401;
    broken.push((lives_too_long, true));
    broken.push((record(JobState::Queued), false));
    let mut leased_in_queue = running();
    leased_in_queue.view.state = JobState::Queued;
    leased_in_queue.view.cleanup_pending = true;
    broken.push((leased_in_queue, true));
    let mut running_without_lease = running();
    running_without_lease.lease = None;
    broken.push((running_without_lease, true));
    let mut stray_cleanup = record(JobState::Queued);
    stray_cleanup.view.cleanup_pending = true;
    broken.push((stray_cleanup, true));
    for (index, (record, has_content)) in broken.into_iter().enumerate() {
        assert!(
            matches!(record.validate(has_content), Err(Error::Integrity)),
            "case {index}"
        );
    }
}

#[test]
fn every_lease_bound_is_checked_on_its_own() {
    for (revision, until) in [(0, NOW + 100), (3, NOW + 100), (2, NOW + 301)] {
        let mut record = running();
        record.lease = Some(Lease {
            nonce: reference("nonce"),
            until,
            revision,
        });
        assert!(
            matches!(record.validate(true), Err(Error::Integrity)),
            "{revision} {until}"
        );
    }
}

#[test]
fn every_graded_result_invariant_is_checked_on_its_own() {
    let passed = |count: usize| vec![CaseGrade::Passed; count];
    assert!(graded(passed(1), 1, 1).validate().is_ok());
    assert!(graded(passed(crate::MAX_CASES), 64, 64).validate().is_ok());
    for result in [
        graded(Vec::new(), 0, 0),
        graded(passed(65), 65, 65),
        graded(vec![CaseGrade::Passed, CaseGrade::WrongAnswer], 1, 1),
        graded(vec![CaseGrade::Trapped(TrapKind::Fuel)], 1, 1),
    ] {
        assert!(matches!(result.validate(), Err(Error::Integrity)));
    }
}
