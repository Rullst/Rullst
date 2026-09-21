#![cfg(all(feature = "sqlite", feature = "receipt-signing"))]
mod support;
use rullst_labs::{sqlite::*, *};
use std::sync::atomic::Ordering;
use support::*;
fn output(job: &LeasedJob, value: i64) -> WorkerOutput {
    WorkerOutput {
        binding: job.input().binding().clone(),
        outcome: WorkerOutcome::Executed {
            artifact: ContentHash::of(b"test-only-wasm-artifact"),
            cases: vec![CaseOutput::Value(value)],
        },
    }
}
fn signer(seed: u8) -> ReceiptSigner {
    ReceiptSigner::from_seed(zeroize::Zeroizing::new([seed; 32])).unwrap()
}
fn profile(key: &ReceiptSigner) -> ExecutionProfile {
    ExecutionProfile::LinuxExperimental {
        receipt_key: key.public_key().unwrap(),
        tools: ToolIdentity {
            runner: ContentHash::of(b"test-runner"),
            compiler: ContentHash::of(b"test-compiler"),
            wasm_toolchain: ContentHash::of(b"test-toolchain"),
            runtime: ContentHash::of(b"test-runtime"),
            launcher: ContentHash::of(b"test-launcher"),
            syscall_policy: ContentHash::of(b"test-policy"),
            filesystem_policy: ContentHash::of(b"test-filesystem-policy"),
        },
    }
}
fn receipt(key: &ReceiptSigner, output: WorkerOutput, teardown: Teardown) -> SignedReceipt {
    key.sign(ExecutionReceipt {
        output,
        started_at: NOW,
        finished_at: NOW,
        observation_digest: ContentHash::of(b"synthetic-contract-test-evidence-not-real-execution"),
        teardown,
    })
    .unwrap()
}
#[tokio::test]
async fn one_lease_and_exact_trusted_grading_without_worker_answers_or_authoritative_pass_flag() {
    let f = Fixture::new(2).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let (first, second) = tokio::join!(f.store.claim_next(), f.store.claim_next());
    let first = first.unwrap();
    let second = second.unwrap();
    assert_ne!(first.is_some(), second.is_some());
    let job = first.or(second).unwrap();
    assert_eq!(
        f.store.lease_status(&job).await.unwrap(),
        LeaseStatus::Active
    );
    let input = serde_json::to_string(job.input()).unwrap();
    assert!(!input.contains("expected"));
    assert!(!input.contains("private-case"));
    assert!(!input.contains("alice"));
    assert!(!format!("{:?}", job.input()).contains("learner-secret-source"));
    let mut malformed = output(&job, 579);
    if let WorkerOutcome::Executed { cases, .. } = &mut malformed.outcome {
        cases.push(CaseOutput::Value(579));
    }
    assert_eq!(
        f.store
            .complete_simulation(&scope(), &id("one"), &malformed)
            .await
            .unwrap_err(),
        LabError::Protocol
    );
    let result = f
        .store
        .complete_simulation(&scope(), &id("one"), &output(&job, 579))
        .await
        .unwrap();
    assert_eq!(result.state, JobState::Simulated);
    assert!(matches!(
        result.result,
        Some(JobResult::Graded {
            passed: 1,
            total: 1,
            evidence: ResultEvidence::Simulation,
            ..
        })
    ));
    assert_eq!(f.store.lease_status(&job).await.unwrap(), LeaseStatus::Stop);
    let mut db = f.database().await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs WHERE content IS NOT NULL")
        .fetch_one(&mut db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    f.store.close().await;
}
#[tokio::test]
async fn cancellation_fences_late_results_and_retention_waits_for_confirmed_cleanup() {
    let f = Fixture::new(1).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    let view = f
        .store
        .cancel(
            &f.policy,
            &id("alice"),
            &scope(),
            &id("one"),
            job.revision(),
        )
        .await
        .unwrap();
    assert!(view.cleanup_pending);
    assert_eq!(f.store.lease_status(&job).await.unwrap(), LeaseStatus::Stop);
    assert_eq!(
        f.store
            .complete_simulation(&scope(), &id("one"), &output(&job, 579))
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    let cleanup = f.store.cleanup_candidates(10).await.unwrap().pop().unwrap();
    let view = f
        .store
        .reconcile_simulation_cleanup(&cleanup, true)
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Cancelled);
    assert!(!view.cleanup_pending);
    assert!(f.store.claim_next().await.unwrap().is_none());
    assert_eq!(
        f.store
            .purge_terminal(&f.policy, &id("teacher"), &scope(), 86400, 10)
            .await
            .unwrap(),
        0
    );
    f.clock.0.store(NOW + 86400, Ordering::SeqCst);
    f.policy.0.store(NOW + 100000, Ordering::SeqCst);
    assert_eq!(
        f.store
            .purge_terminal(&f.policy, &id("alice"), &scope(), 86400, 10)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        f.store
            .purge_terminal(&f.policy, &id("teacher"), &scope(), 86400, 10)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        f.store
            .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
            .await
            .unwrap_err(),
        LabError::NotFound
    );
    f.store.close().await;
}
#[tokio::test]
async fn lost_worker_requires_cleanup_and_a_fresh_nonce_before_one_bounded_retry() {
    let f = Fixture::new(1).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let old = f.store.claim_next().await.unwrap().unwrap();
    f.clock.0.store(old.expires_at(), Ordering::SeqCst);
    assert_eq!(
        f.store
            .complete_simulation(&scope(), &id("one"), &output(&old, 579))
            .await
            .unwrap_err(),
        LabError::Expired
    );
    let cleanup = f.store.cleanup_candidates(1).await.unwrap().pop().unwrap();
    assert!(f.store.claim_next().await.unwrap().is_none());
    let view = f
        .store
        .reconcile_simulation_cleanup(&cleanup, true)
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Queued);
    let new = f.store.claim_next().await.unwrap().unwrap();
    assert_ne!(old.input().binding().nonce, new.input().binding().nonce);
    assert_eq!(
        f.store
            .complete_simulation(&scope(), &id("one"), &output(&old, 579))
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    f.clock.0.store(new.expires_at(), Ordering::SeqCst);
    let cleanup = f.store.cleanup_candidates(1).await.unwrap().pop().unwrap();
    assert_eq!(
        f.store
            .reconcile_simulation_cleanup(&cleanup, true)
            .await
            .unwrap()
            .state,
        JobState::Cancelled
    );
    assert!(f.store.claim_next().await.unwrap().is_none());
    f.store.close().await;
}
#[tokio::test]
async fn pinned_receipt_key_exact_binding_and_teardown_control_experimental_grades() {
    let key = signer(4);
    let f = Fixture::with_profile(2, profile(&key)).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    f.store
        .submit(&f.policy, &id("bob"), &scope(), submission("two"))
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    let other = f.store.claim_next().await.unwrap().unwrap();
    let output = output(&job, 579);
    assert_eq!(
        f.store
            .complete_simulation(job.scope(), job.id(), &output)
            .await
            .unwrap_err(),
        LabError::Unsupported
    );
    assert_eq!(
        f.store
            .complete(
                job.scope(),
                job.id(),
                &receipt(&signer(5), output.clone(), Teardown::Confirmed)
            )
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    assert_eq!(
        f.store
            .complete(
                job.scope(),
                job.id(),
                &receipt(&key, output.clone(), Teardown::Uncertain)
            )
            .await
            .unwrap_err(),
        LabError::Uncertain
    );
    let signed = receipt(&key, output, Teardown::Confirmed);
    assert_eq!(
        f.store
            .complete(other.scope(), other.id(), &signed)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    let mut forged = serde_json::to_value(&signed).unwrap();
    forged["receipt"]["output"]["outcome"]["Executed"]["cases"][0] = serde_json::json!({"Value":0});
    let forged = SignedReceipt::from_bytes(&serde_json::to_vec(&forged).unwrap()).unwrap();
    assert_eq!(
        f.store
            .complete(job.scope(), job.id(), &forged)
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    let result = f
        .store
        .complete(job.scope(), job.id(), &signed)
        .await
        .unwrap();
    assert_eq!(result.state, JobState::Completed);
    assert!(matches!(
        result.result,
        Some(JobResult::Graded {
            passed: 1,
            total: 1,
            evidence: ResultEvidence::Experimental { .. },
            ..
        })
    ));
    assert_eq!(
        f.store
            .complete(job.scope(), job.id(), &signed)
            .await
            .unwrap(),
        result
    );
    assert_eq!(
        f.store
            .complete(other.scope(), other.id(), &signed)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    f.store.close().await;
}
#[tokio::test]
async fn withdrawal_and_expiry_prevent_a_validly_signed_late_grade() {
    let key = signer(4);
    let f = Fixture::with_profile(1, profile(&key)).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    let signed = receipt(&key, output(&job, 579), Teardown::Confirmed);
    f.store
        .set_exercise_enabled(
            &f.policy,
            &id("teacher"),
            &scope(),
            &id("sum"),
            &id("v1"),
            false,
        )
        .await
        .unwrap();
    assert_eq!(f.store.lease_status(&job).await.unwrap(), LeaseStatus::Stop);
    assert_eq!(
        f.store
            .complete(job.scope(), job.id(), &signed)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    f.store
        .set_exercise_enabled(
            &f.policy,
            &id("teacher"),
            &scope(),
            &id("sum"),
            &id("v1"),
            true,
        )
        .await
        .unwrap();
    f.clock.0.store(job.expires_at(), Ordering::SeqCst);
    assert_eq!(
        f.store
            .complete(job.scope(), job.id(), &signed)
            .await
            .unwrap_err(),
        LabError::Expired
    );
    f.store.close().await;
}
