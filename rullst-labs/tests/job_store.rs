#![cfg(feature = "sqlite")]
mod support;
use rullst_labs::{sqlite::*, *};
use std::sync::atomic::Ordering;
use support::*;

#[tokio::test]
async fn ownership_idempotency_restart_and_cancel_preserve_one_submission() {
    let f = Fixture::new(1).await;
    let first = f
        .store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let restored = SqliteLabs::open(
        f.dir.path().join("jobs.sqlite"),
        config(1),
        ContentKey::new([9; 32]).unwrap(),
        f.clock.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        restored
            .submit(&f.policy, &id("alice"), &scope(), submission("one"))
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        restored
            .submit(&f.policy, &id("bob"), &scope(), submission("one"))
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    assert_eq!(
        restored
            .get_job(&f.policy, &id("bob"), &scope(), &id("one"))
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        restored
            .get_job(
                &f.policy,
                &id("alice"),
                &Scope::new("other", "rust").unwrap(),
                &id("one")
            )
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        restored
            .get_job(&f.policy, &id("teacher"), &scope(), &id("one"))
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        restored
            .submit(&f.policy, &id("alice"), &scope(), submission("two"))
            .await
            .unwrap_err(),
        LabError::Capacity
    );
    let changed = Submission::new(
        id("one"),
        ExerciseRef::new("sum", "v1").unwrap(),
        RustSource::new("pub fn solve(a:i64,b:i64)->i64 { a-b }").unwrap(),
        300,
    )
    .unwrap();
    assert_eq!(
        restored
            .submit(&f.policy, &id("alice"), &scope(), changed)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    assert_eq!(
        restored
            .cancel(&f.policy, &id("bob"), &scope(), &id("one"), 1)
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert_eq!(
        restored
            .cancel(&f.policy, &id("alice"), &scope(), &id("one"), 2)
            .await
            .unwrap_err(),
        LabError::Conflict
    );
    let cancelled = restored
        .cancel(&f.policy, &id("alice"), &scope(), &id("one"), 1)
        .await
        .unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert!(!cancelled.cleanup_pending);
    assert_eq!(
        restored
            .submit(&f.policy, &id("alice"), &scope(), submission("one"))
            .await
            .unwrap(),
        cancelled
    );
    let mut db = f.database().await;
    let content: Option<Vec<u8>> = sqlx::query_scalar("SELECT content FROM labs_jobs")
        .fetch_one(&mut db)
        .await
        .unwrap();
    assert!(content.is_none());
    f.policy.0.store(NOW, Ordering::SeqCst);
    assert_eq!(
        restored
            .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
            .await
            .unwrap_err(),
        LabError::Denied
    );
    restored.close().await;
    f.store.close().await;
}

#[tokio::test]
async fn source_and_grader_are_encrypted_and_record_index_tampering_is_detected() {
    let f = Fixture::new(2).await;
    f.store
        .submit(&f.policy, &id("alice"), &scope(), submission("one"))
        .await
        .unwrap();
    let view = f
        .store
        .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
        .await
        .unwrap();
    let public = serde_json::to_string(&view).unwrap();
    assert!(!public.contains("private-case"));
    assert!(!public.contains("learner-secret-source"));
    for entry in std::fs::read_dir(f.dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for secret in [b"private-case".as_slice(), b"learner-secret-source"] {
            assert!(!bytes.windows(secret.len()).any(|w| w == secret));
        }
    }
    let mut db = f.database().await;
    sqlx::query("UPDATE labs_jobs SET state='Completed'")
        .execute(&mut db)
        .await
        .unwrap();
    assert_eq!(
        f.store
            .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    sqlx::query("UPDATE labs_jobs SET state='Queued'")
        .execute(&mut db)
        .await
        .unwrap();
    let mut body: Vec<u8> = sqlx::query_scalar("SELECT body FROM labs_jobs")
        .fetch_one(&mut db)
        .await
        .unwrap();
    body[15] ^= 1;
    sqlx::query("UPDATE labs_jobs SET body=?")
        .bind(body)
        .execute(&mut db)
        .await
        .unwrap();
    assert_eq!(
        f.store
            .get_job(&f.policy, &id("alice"), &scope(), &id("one"))
            .await
            .unwrap_err(),
        LabError::Integrity
    );
    f.store.close().await;
}

#[tokio::test]
async fn simultaneous_submissions_are_single_and_authority_is_checked_after_lock_wait() {
    let f = Fixture::new(2).await;
    let actor = id("alice");
    let scope = scope();
    let (a, b) = tokio::join!(
        f.store
            .submit(&f.policy, &actor, &scope, submission("same")),
        f.store
            .submit(&f.policy, &actor, &scope, submission("same"))
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let mut db = f.database().await;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut db)
        .await
        .unwrap();
    let store = f.store.clone();
    let policy = f.policy.clone();
    let pending = tokio::spawn(async move {
        store
            .submit(&policy, &id("alice"), &self::scope(), submission("late"))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    f.clock.0.store(NOW + 1000, Ordering::SeqCst);
    sqlx::query("COMMIT").execute(&mut db).await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(LabError::Expired | LabError::Denied)
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs")
        .fetch_one(&mut db)
        .await
        .unwrap();
    assert_eq!(count, 1);
    f.store.close().await;
}

#[tokio::test]
async fn withdrawing_the_registered_grader_denies_new_submissions() {
    let f = Fixture::new(2).await;
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
    assert_eq!(
        f.store
            .submit(&f.policy, &id("alice"), &scope(), submission("one"))
            .await
            .unwrap_err(),
        LabError::Denied
    );
    assert!(
        Submission::new(
            id("one"),
            ExerciseRef::new("sum", "v1").unwrap(),
            RustSource::new(SOURCE).unwrap(),
            0
        )
        .is_err()
    );
    let mut forged = serde_json::to_value(submission("one")).unwrap();
    forged["learner"] = serde_json::json!("teacher");
    assert!(serde_json::from_value::<Submission>(forged).is_err());
    f.store.close().await;
}

#[tokio::test]
async fn authorized_tenants_and_courses_keep_identical_job_ids_independent() {
    // All three scopes are authorized so the durable lookup, rather than a
    // fixture denying other courses, must enforce their separation.
    struct MultiCourse([Scope; 3]);
    impl Authorization for MultiCourse {
        async fn check(
            &self,
            actor: &Reference,
            scope: &Scope,
            action: Action,
        ) -> Result<Permission, LabError> {
            if !self.0.contains(scope)
                || !matches!(actor.as_str(), "teacher" | "alice")
                || (matches!(action, Action::ManageJobs | Action::ManageExercises)
                    && actor.as_str() != "teacher")
            {
                return Err(LabError::Denied);
            }
            Permission::until(NOW + 1000)
        }
    }
    let f = Fixture::new(4).await;
    let scopes = [
        scope(),
        Scope::new("other-school", "rust").unwrap(),
        Scope::new("school", "other-course").unwrap(),
    ];
    let policy = MultiCourse(scopes.clone());
    for scope in &scopes[1..] {
        let exercise = Exercise::new(
            scope.clone(),
            id("sum"),
            id("v1"),
            vec![GraderCase {
                id: id("private-case"),
                input: [123, 456],
                expected: 579,
            }],
            ExecutionLimits::new(10, 100000, 64).unwrap(),
        )
        .unwrap();
        f.store
            .register_exercise(&policy, &id("teacher"), &exercise)
            .await
            .unwrap();
    }
    let mut originals = Vec::new();
    for scope in &scopes {
        originals.push(
            f.store
                .submit(&policy, &id("alice"), scope, submission("same-id"))
                .await
                .unwrap(),
        );
    }
    let cancelled = f
        .store
        .cancel(&policy, &id("alice"), &scopes[0], &id("same-id"), 1)
        .await
        .unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    for (scope, original) in scopes.iter().zip(&originals).skip(1) {
        assert_eq!(
            f.store
                .get_job(&policy, &id("alice"), scope, &id("same-id"))
                .await
                .unwrap(),
            *original
        );
        assert_eq!(
            f.store
                .submit(&policy, &id("alice"), scope, submission("same-id"))
                .await
                .unwrap(),
            *original
        );
    }
    let mut db = f.database().await;
    let retained: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM labs_jobs WHERE content IS NOT NULL")
            .fetch_one(&mut db)
            .await
            .unwrap();
    assert_eq!(retained, 2);
    drop(db);
    f.store.close().await;
}
