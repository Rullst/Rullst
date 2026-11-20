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
        evidence: ResultEvidence::SignedReceipt {
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

/// `ResultEvidence::SignedReceipt` was `Experimental` before 13.0.0-rc.1. A
/// completed record, as 13.0.0-alpha.6 sealed it into the store.
const COMPLETED_RECORD_JSON: &str = concat!(
    r#"{"view":{"id":"job-1","scope":{"tenant":"school","course":"rust"},"learner":"alice","#,
    r#""exercise":{"id":"sum","revision":"v1"},"#,
    r#""source_digest":"41cf6794ba4200b839c53531555f0f3998df4cbb01a4d5cb0b94e3ca5e23947d","#,
    r#""exercise_digest":"f55101cb5e7453c770d2a062a242f13479743de0c24329462d3cbf3f2d6bc7e1","#,
    r#""state":"Completed","revision":2,"created_at":1800000000,"updated_at":1800000000,"#,
    r#""expires_at":1800000300,"cleanup_pending":false,"result":{"Graded":{"cases":["Passed"],"#,
    r#""passed":1,"total":1,"#,
    r#""artifact":"c7c5c1d70c5dec4416ab6158afd0b223ef40c29b1dc1f97ed9428b94d4cadb1c","#,
    r#""evidence":{"Experimental":{"#,
    r#""receipt":"6f32860910ca0fb2a20c7fda143666b09dbf8db5238195c90a586fb542ff0cad","#,
    r#""observations":"bfc5a6b45b6a16128fbf94195a8b6895883b709e9ab0bf0bc5dc5a14fa18cf2f"}}}}},"#,
    r#""request_digest":"1f58b9145b24d108d7ac38887338b3ea3229833b9c1e418250343f907bfd1047","#,
    r#""profile_digest":"1900eab6c028483d7126599ee6f50de0d27907b5c65fa90524580b4b0f9852b0","#,
    r#""lease":null,"attempts":0}"#,
);

#[test]
fn signed_receipt_evidence_keeps_the_experimental_stored_name() {
    let json = serde_json::to_string(&completed()).unwrap();
    assert_eq!(json, COMPLETED_RECORD_JSON);
    // The digest of the same JSON computed before the rename.
    assert_eq!(
        ContentHash::of(json.as_bytes()).as_str(),
        "90ae05ba6b9c31d9e3f50b59f8e1153cd624b1ca8a82d92ca429b85039450593"
    );
    let stored: Record = serde_json::from_str(COMPLETED_RECORD_JSON).unwrap();
    assert!(stored.validate(false).is_ok());
    assert!(matches!(
        stored.view.result.as_ref().unwrap().evidence(),
        ResultEvidence::SignedReceipt { receipt, observations }
            if *receipt == ContentHash::of(b"receipt")
                && *observations == ContentHash::of(b"observations")
    ));
    assert_eq!(
        serde_json::to_string(&stored).unwrap(),
        COMPLETED_RECORD_JSON
    );
    let evidence = ResultEvidence::SignedReceipt {
        receipt: ContentHash::of(b"receipt"),
        observations: ContentHash::of(b"observations"),
    };
    assert_eq!(
        serde_json::to_string(&evidence).unwrap(),
        concat!(
            r#"{"Experimental":{"receipt":"6f32860910ca0fb2a20c7fda143666b09dbf8db5238195c90a586fb542ff0cad","#,
            r#""observations":"bfc5a6b45b6a16128fbf94195a8b6895883b709e9ab0bf0bc5dc5a14fa18cf2f"}}"#,
        )
    );
    assert_eq!(
        serde_json::to_string(&ResultEvidence::Simulation).unwrap(),
        r#""Simulation""#
    );
    // One stored name only: the Rust name is never read.
    let renamed = COMPLETED_RECORD_JSON.replace(r#""Experimental""#, r#""SignedReceipt""#);
    assert!(serde_json::from_str::<Record>(&renamed).is_err());
}
