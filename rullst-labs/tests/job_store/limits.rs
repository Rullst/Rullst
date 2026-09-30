//! Submission lifetime, projection and per-learner capacity limits.
use super::*;

#[tokio::test]
async fn submissions_that_could_never_be_claimed_are_refused() {
    let f = Fixture::new(2).await;
    // The fixture exercise has a 10-second wall limit: a job needs over 15 s.
    let short = |ttl| {
        Submission::new(
            id("short"),
            ExerciseRef::new("sum", "v1").unwrap(),
            RustSource::new(SOURCE).unwrap(),
            ttl,
        )
        .unwrap()
    };
    assert_eq!(
        f.store
            .submit(&f.policy, &id("alice"), &scope(), short(15))
            .await
            .unwrap_err(),
        LabError::InvalidInput
    );
    // A per-request Submit grant shorter than that would expire every job.
    f.policy.0.store(NOW + 15, Ordering::SeqCst);
    assert_eq!(
        f.store
            .submit(&f.policy, &id("alice"), &scope(), submission("one"))
            .await
            .unwrap_err(),
        LabError::Expired
    );
    f.policy.0.store(NOW + 16, Ordering::SeqCst);
    let queued = f
        .store
        .submit(&f.policy, &id("alice"), &scope(), short(16))
        .await
        .unwrap();
    assert_eq!(queued.expires_at, NOW + 16);
    let mut db = f.database().await;
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs")
        .fetch_one(&mut db)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn returned_views_never_carry_the_unkeyed_exercise_digest() {
    let f = Fixture::new(2).await;
    // The raw digest hashes the hidden case: a learner holding it could test
    // guesses of the case inputs and expected values offline.
    let raw = exercise().digest().unwrap();
    let first = f
        .store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    assert_ne!(first.exercise_digest, raw);
    assert!(
        !serde_json::to_string(&first)
            .unwrap()
            .contains(raw.as_str())
    );
    // It still identifies the same exercise snapshot across jobs and reads.
    let second = f
        .store
        .submit(&f.policy, &id("bob"), &scope(), submission("two"))
        .await
        .unwrap();
    assert_eq!(second.exercise_digest, first.exercise_digest);
    let read = f
        .store
        .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
        .await
        .unwrap();
    assert_eq!(read, first);
    let cancelled = f
        .store
        .cancel(&f.policy, &id("alice"), &scope(), &id("one"), 1)
        .await
        .unwrap();
    assert_eq!(cancelled.exercise_digest, first.exercise_digest);
}

#[tokio::test]
async fn one_learner_cannot_fill_the_store_wide_job_budget() {
    let f = Fixture::new(101).await;
    for n in 0..100 {
        f.store
            .submit(
                &f.policy,
                &id("alice"),
                &scope(),
                submission(&format!("a{n}")),
            )
            .await
            .unwrap();
    }
    // Cancelled jobs keep their slot until retention purge.
    f.store
        .cancel(&f.policy, &id("alice"), &scope(), &id("a0"), 1)
        .await
        .unwrap();
    assert_eq!(
        f.store
            .submit(&f.policy, &id("alice"), &scope(), submission("a100"))
            .await
            .unwrap_err(),
        LabError::Capacity
    );
    f.store
        .submit(&f.policy, &id("bob"), &scope(), submission("b0"))
        .await
        .unwrap();
}

#[tokio::test]
async fn an_explicit_learner_quota_is_enforced_and_bound_into_the_configuration() {
    assert!(config(4).learner_jobs(0).is_err());
    assert!(config(4).learner_jobs(5).is_err());
    let f = Fixture::new(4).await;
    let key = || ContentKey::new([9; 32]).unwrap();
    let path = f.dir.path().join("quota.sqlite");
    let store = SqliteLabs::initialize(
        &path,
        config(4).learner_jobs(1).unwrap(),
        key(),
        f.clock.clone(),
    )
    .await
    .unwrap();
    store
        .register_exercise(&f.policy, &id("teacher"), &exercise())
        .await
        .unwrap();
    store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    assert_eq!(
        store
            .submit(&f.policy, &id("alice"), &scope(), submission("two"))
            .await
            .unwrap_err(),
        LabError::Capacity
    );
    store
        .submit(&f.policy, &id("bob"), &scope(), submission("three"))
        .await
        .unwrap();
    store.close().await;
    assert!(matches!(
        SqliteLabs::open(&path, config(4), key(), f.clock.clone()).await,
        Err(LabError::Configuration)
    ));
    SqliteLabs::open(
        &path,
        config(4).learner_jobs(1).unwrap(),
        key(),
        f.clock.clone(),
    )
    .await
    .unwrap()
    .close()
    .await;
    // Naming the default quota keeps the binding of existing stores.
    f.store.close().await;
    SqliteLabs::open(
        f.dir.path().join("jobs.sqlite"),
        config(4).learner_jobs(4).unwrap(),
        key(),
        f.clock.clone(),
    )
    .await
    .unwrap();
}
