use super::*;

async fn submit(f: &Fixture, name: &str) -> JobView {
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission(name))
        .await
        .unwrap()
}

fn at(f: &Fixture, now: i64) {
    f.clock.0.store(now, Ordering::SeqCst);
}

#[tokio::test]
async fn a_claim_needs_the_wall_time_plus_five_seconds_before_expiry() {
    let f = Fixture::new(4).await;
    submit(&f, "job-a").await;
    submit(&f, "job-b").await;
    // Both expire at NOW + 300; the 10-second exercise needs 16 left.
    at(&f, NOW + 284);
    let leased = f.store.claim_next().await.unwrap().unwrap();
    assert_eq!(leased.id(), &id("job-a"));
    at(&f, NOW + 285);
    assert!(f.store.claim_next().await.unwrap().is_none());
    let view = f
        .store
        .get_job(&f.policy, &id("alice"), &scope(), &id("job-b"))
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Expired);
}

#[tokio::test]
async fn a_stale_revision_cannot_cancel() {
    let f = Fixture::new(4).await;
    let view = submit(&f, "job-a").await;
    assert_eq!(
        f.store
            .cancel(
                &f.policy,
                &id("alice"),
                &scope(),
                &id("job-a"),
                view.revision + 1
            )
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    let cancelled = f
        .store
        .cancel(
            &f.policy,
            &id("alice"),
            &scope(),
            &id("job-a"),
            view.revision,
        )
        .await
        .unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
}

#[tokio::test]
async fn a_repeated_simulation_result_conflicts() {
    let f = Fixture::new(4).await;
    submit(&f, "job-a").await;
    let job = f.store.claim_next().await.unwrap().unwrap();
    let output = WorkerOutput {
        binding: job.input().binding().clone(),
        outcome: WorkerOutcome::Rejected(ExecutionFailure::Isolation),
    };
    f.store
        .complete_simulation(job.scope(), job.id(), &output)
        .await
        .unwrap();
    assert_eq!(
        f.store
            .complete_simulation(job.scope(), job.id(), &output)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
}

#[tokio::test]
async fn reconciling_an_uncertain_attempt_needs_its_binding_and_time_for_a_retry() {
    let f = Fixture::new(4).await;
    submit(&f, "job-a").await;
    submit(&f, "job-b").await;
    let a = f.store.claim_next().await.unwrap().unwrap();
    let b = f.store.claim_next().await.unwrap().unwrap();
    let a = f.store.abandon_attempt(&a).await.unwrap();
    let b = f.store.abandon_attempt(&b).await.unwrap();
    let mut stale = a.clone();
    stale.binding.nonce = id("another-attempt");
    assert_eq!(
        f.store
            .reconcile_simulation_cleanup(&stale, true)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    // A retry needs more than the 10-second wall time plus five seconds.
    at(&f, NOW + 284);
    let retried = f
        .store
        .reconcile_simulation_cleanup(&a, true)
        .await
        .unwrap();
    assert_eq!(retried.state, JobState::Queued);
    at(&f, NOW + 285);
    let expired = f
        .store
        .reconcile_simulation_cleanup(&b, true)
        .await
        .unwrap();
    assert_eq!(expired.state, JobState::Expired);
}

#[tokio::test]
async fn a_lease_stops_when_its_deadline_arrives() {
    let f = Fixture::new(4).await;
    submit(&f, "job-a").await;
    // Claimed at NOW, a 10-second exercise holds its lease until NOW + 25.
    let job = f.store.claim_next().await.unwrap().unwrap();
    assert_eq!(job.expires_at(), NOW + 25);
    at(&f, NOW + 24);
    assert_eq!(
        f.store.lease_status(&job).await.unwrap(),
        LeaseStatus::Active
    );
    at(&f, NOW + 25);
    assert_eq!(f.store.lease_status(&job).await.unwrap(), LeaseStatus::Stop);
}
