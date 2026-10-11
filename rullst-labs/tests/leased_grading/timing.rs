use super::*;

fn timed(
    key: &ReceiptSigner,
    output: WorkerOutput,
    started_at: i64,
    finished_at: i64,
) -> SignedReceipt {
    key.sign(ExecutionReceipt {
        output,
        started_at,
        finished_at,
        observation_digest: ContentHash::of(b"synthetic-timing-test-evidence"),
        teardown: Teardown::Confirmed,
    })
    .unwrap()
}

#[tokio::test]
async fn a_receipt_must_fall_between_the_claim_and_the_store_time() {
    let key = signer(6);
    let f = Fixture::with_profile(2, profile(&key)).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    f.clock.0.store(NOW + 5, Ordering::SeqCst);
    let result = output(&job, 579);
    for (started, finished) in [(NOW - 1, NOW + 4), (NOW + 1, NOW + 6)] {
        assert_eq!(
            f.store
                .complete(
                    job.scope(),
                    job.id(),
                    &timed(&key, result.clone(), started, finished)
                )
                .await
                .unwrap_err(),
            LabError::Protocol,
            "{started} {finished}"
        );
    }
    let view = f
        .store
        .complete(
            job.scope(),
            job.id(),
            &timed(&key, result, NOW + 1, NOW + 4),
        )
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Completed);
}

#[tokio::test]
async fn only_a_worker_loss_attestation_reconciles_an_abandoned_attempt() {
    let key = signer(7);
    let f = Fixture::with_profile(2, profile(&key)).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let job = f.store.claim_next().await.unwrap().unwrap();
    f.store.abandon_attempt(&job).await.unwrap();
    let attestation = |failure| {
        timed(
            &key,
            WorkerOutput {
                binding: job.input().binding().clone(),
                outcome: WorkerOutcome::Rejected(failure),
            },
            NOW,
            NOW,
        )
    };
    assert_eq!(
        f.store
            .reconcile_cleanup(
                job.scope(),
                job.id(),
                &attestation(ExecutionFailure::Isolation),
                false
            )
            .await
            .unwrap_err(),
        LabError::Uncertain
    );
    let view = f
        .store
        .reconcile_cleanup(
            job.scope(),
            job.id(),
            &attestation(ExecutionFailure::WorkerLost),
            false,
        )
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Cancelled);
}
