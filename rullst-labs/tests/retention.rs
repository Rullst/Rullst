#![cfg(feature = "sqlite")]
mod support;
use rullst_labs::{sqlite::*, *};
use std::sync::atomic::Ordering;
use support::*;

#[tokio::test]
async fn offline_maintenance_clears_expired_queue_but_never_claims_worker_teardown() {
    let f = Fixture::new(3).await;
    for name in ["active-worker", "queued-one", "queued-two"] {
        f.store
            .submit(&f.policy, &id("alice"), &scope(), submission(name))
            .await
            .unwrap();
    }
    let leased = f.store.claim_next().await.unwrap().unwrap();
    assert_eq!(leased.id(), &id("active-worker"));
    assert_eq!(
        f.store
            .expire_queued(&f.policy, &id("teacher"), &scope(), 2)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        f.store
            .expire_queued(&f.policy, &id("alice"), &scope(), 2)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        f.store
            .expire_queued(
                &f.policy,
                &id("teacher"),
                &Scope::new("other", "rust").unwrap(),
                2
            )
            .await
            .unwrap_err(),
        LabError::Denied
    );
    for limit in [0, 101] {
        assert_eq!(
            f.store
                .expire_queued(&f.policy, &id("teacher"), &scope(), limit)
                .await
                .unwrap_err(),
            LabError::InvalidInput
        );
    }
    f.clock.0.store(NOW + 300, Ordering::SeqCst);
    for _ in 0..2 {
        assert_eq!(
            f.store
                .expire_queued(&f.policy, &id("teacher"), &scope(), 1)
                .await
                .unwrap(),
            1
        );
    }
    assert_eq!(
        f.store
            .expire_queued(&f.policy, &id("teacher"), &scope(), 100)
            .await
            .unwrap(),
        0
    );
    let view = f
        .store
        .get_job(&f.policy, &id("alice"), &scope(), leased.id())
        .await
        .unwrap();
    assert_eq!(view.state, JobState::Running);
    assert!(view.result.is_none());
    let mut database = f.database().await;
    let retained: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs WHERE content IS NOT NULL")
            .fetch_one(&mut database)
            .await
            .unwrap();
    assert_eq!(retained, 1);
    for name in ["queued-one", "queued-two"] {
        let view = f
            .store
            .get_job(&f.policy, &id("alice"), &scope(), &id(name))
            .await
            .unwrap();
        assert_eq!(view.state, JobState::Expired);
        assert!(!view.cleanup_pending && view.result.is_none());
    }
    let cleanup = f.store.cleanup_candidates(1).await.unwrap().pop().unwrap();
    assert_eq!(cleanup.id, *leased.id());
    f.store.close().await;
}

#[tokio::test]
async fn instructor_cannot_remove_a_grader_until_withdrawal_and_job_retention_finish() {
    let f = Fixture::new(1).await;
    let reference = ExerciseRef::new("sum", "v1").unwrap();
    assert_eq!(
        f.store
            .remove_exercise(&f.policy, &id("alice"), &scope(), &reference)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        f.store
            .remove_exercise(&f.policy, &id("teacher"), &scope(), &reference)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    f.store
        .set_exercise_enabled(
            &f.policy,
            &id("teacher"),
            &scope(),
            &reference.id,
            &reference.revision,
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        f.store
            .remove_exercise(&f.policy, &id("teacher"), &scope(), &reference)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    f.clock.0.store(NOW + 300, Ordering::SeqCst);
    assert_eq!(
        f.store
            .expire_queued(&f.policy, &id("teacher"), &scope(), 1)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        f.store
            .remove_exercise(&f.policy, &id("teacher"), &scope(), &reference)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    f.clock.0.store(NOW + 300 + 86400, Ordering::SeqCst);
    f.policy.0.store(NOW + 100000, Ordering::SeqCst);
    assert_eq!(
        f.store
            .purge_terminal(&f.policy, &id("teacher"), &scope(), 86400, 1)
            .await
            .unwrap(),
        1
    );
    f.store
        .remove_exercise(&f.policy, &id("teacher"), &scope(), &reference)
        .await
        .unwrap();
    assert_eq!(
        f.store
            .get_exercise(
                &f.policy,
                &id("teacher"),
                &scope(),
                &reference.id,
                &reference.revision
            )
            .await
            .unwrap_err(),
        LabError::NotFound
    );
    f.store.close().await;
}

#[tokio::test]
async fn corrupt_queued_metadata_rolls_back_the_whole_retention_batch() {
    let f = Fixture::new(2).await;
    for name in ["one", "two"] {
        f.store
            .submit(&f.policy, &id("alice"), &scope(), submission(name))
            .await
            .unwrap();
    }
    let mut database = f.database().await;
    sqlx::query("UPDATE labs_jobs SET expires_at=expires_at+1 WHERE id='two'")
        .execute(&mut database)
        .await
        .unwrap();
    f.clock.0.store(NOW + 301, Ordering::SeqCst);
    assert_eq!(
        f.store
            .expire_queued(&f.policy, &id("teacher"), &scope(), 2)
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    let intact: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM labs_jobs WHERE content IS NOT NULL AND state='Queued'",
    )
    .fetch_one(&mut database)
    .await
    .unwrap();
    assert_eq!(intact, 2);
    f.store.close().await;
}
