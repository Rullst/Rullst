//! Tests of the `byo_runner_controller` example (a subdirectory without
//! `main.rs`, so Cargo does not build it as an example of its own).
use super::*;
use rullst_labs::sqlite::{JobResult, JobState, ResultEvidence};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The wall clock, except that its second read steps back 30 seconds
/// like an NTP correction.
struct SteppedClock(AtomicUsize);
impl Clock for SteppedClock {
    fn now(&self) -> Result<i64, LabError> {
        let now = SystemClock.now()?;
        Ok(if self.0.fetch_add(1, Ordering::SeqCst) == 1 {
            now - 30
        } else {
            now
        })
    }
}

#[tokio::test]
async fn a_receipt_that_cannot_be_signed_still_fences_and_reconciles_the_attempt() {
    let demo = Demo::start().await.unwrap();
    let queued = demo.submit().await.unwrap();
    let job = demo.controller.claim_next().await.unwrap().unwrap();
    let clock = SteppedClock(AtomicUsize::new(0));
    // `finished_at` precedes `started_at`, so `sign` refuses the receipt.
    let error = execute_claimed(&demo.controller, &demo.signer, &clock, &job)
        .await
        .unwrap_err();
    assert_eq!(error, LabError::Protocol);
    // The attempt is fenced and attested at once instead of staying
    // `Running` under a live lease until a later recovery pass.
    let view = demo.status(&queued.id).await.unwrap();
    assert_eq!(view.state, JobState::Cancelled);
    assert!(!view.cleanup_pending && view.result.is_none());
    assert_eq!(recover(&demo.controller, &demo.signer).await.unwrap(), 0);
    demo.close().await;
}

#[tokio::test]
async fn fixed_verdict_receipt_is_accepted_and_never_graded_as_executed() {
    let view = run().await.unwrap();
    assert_eq!(view.state, JobState::Failed);
    assert!(!view.cleanup_pending);
    assert!(matches!(
        view.result,
        Some(JobResult::Rejected {
            failure: ExecutionFailure::Isolation,
            diagnostics: None,
            evidence: ResultEvidence::Experimental { .. },
        })
    ));
}

/// A worker that never answers is stopped at the wall limit and the job
/// ends `Failed` with `TimeLimit`, not `Cancelled` like a learner's cancel.
#[tokio::test]
async fn wall_limit_is_reported_as_a_time_limit_not_a_cancellation() {
    let demo = Demo::with_wall_limit(5).await.unwrap();
    let queued = demo.submit().await.unwrap();
    let job = demo.controller.claim_next().await.unwrap().unwrap();
    let started_at = SystemClock.now().unwrap();
    let stalled = attempt(&demo.controller, &job, std::future::pending())
        .await
        .unwrap();
    let signed = demo
        .signer
        .sign(ExecutionReceipt {
            output: stalled.output,
            started_at,
            finished_at: SystemClock.now().unwrap(),
            observation_digest: stalled.observations,
            teardown: Teardown::Confirmed,
        })
        .unwrap();
    demo.controller
        .complete(job.scope(), job.id(), &signed)
        .await
        .unwrap();
    let view = demo.status(&queued.id).await.unwrap();
    assert_eq!(view.state, JobState::Failed);
    assert!(matches!(
        view.result,
        Some(JobResult::Rejected {
            failure: ExecutionFailure::TimeLimit,
            diagnostics: None,
            ..
        })
    ));
    demo.close().await;
}

#[tokio::test]
async fn cancellation_before_release_is_fenced_and_reconciled() {
    let demo = Demo::start().await.unwrap();
    let queued = demo.submit().await.unwrap();
    let job = demo.controller.claim_next().await.unwrap().unwrap();
    let alice = Reference::new("alice").unwrap();
    demo.app
        .cancel(
            &DemoPolicy,
            &alice,
            &demo_scope().unwrap(),
            &queued.id,
            job.revision(),
        )
        .await
        .unwrap();
    let error = execute_claimed(&demo.controller, &demo.signer, &SystemClock, &job)
        .await
        .unwrap_err();
    assert_eq!(error, LabError::Conflict);
    let view = demo.status(&queued.id).await.unwrap();
    assert_eq!(view.state, JobState::Cancelled);
    assert!(!view.cleanup_pending && view.result.is_none());
    assert!(
        process_one(&demo.controller, &demo.signer)
            .await
            .unwrap()
            .is_none()
    );
    demo.close().await;
}

#[tokio::test]
async fn startup_recovery_attests_a_lease_left_by_a_lost_controller() {
    let demo = Demo::start().await.unwrap();
    let queued = demo.submit().await.unwrap();
    // A controller claims and then disappears; the learner cancels.
    let lost = demo.controller.claim_next().await.unwrap().unwrap();
    let alice = Reference::new("alice").unwrap();
    let cancelled = demo
        .app
        .cancel(
            &DemoPolicy,
            &alice,
            &demo_scope().unwrap(),
            &queued.id,
            lost.revision(),
        )
        .await
        .unwrap();
    assert!(cancelled.cleanup_pending);
    assert_eq!(recover(&demo.controller, &demo.signer).await.unwrap(), 1);
    let view = demo.status(&queued.id).await.unwrap();
    assert_eq!(view.state, JobState::Cancelled);
    assert!(!view.cleanup_pending);
    assert_eq!(recover(&demo.controller, &demo.signer).await.unwrap(), 0);
    demo.close().await;
}
