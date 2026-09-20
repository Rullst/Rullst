use super::*;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

#[tokio::test]
async fn independent_pools_cannot_apply_competing_policy_revisions() {
    let (temp, store, clock) = fixture().await;
    let other = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    let managed = store.enroll_parental(&operator(), &scope()).await.unwrap();
    store
        .provision_authority(
            &operator(),
            &key(AuthorityAction::ParentalManage),
            None,
            2000,
        )
        .await
        .unwrap();
    let first = CoursePolicy::new(["first"], 0, 2000).unwrap();
    let second = CoursePolicy::new(["second"], 0, 2000).unwrap();
    let actor = context("delegate");
    let scope = scope();
    let (left, right) = tokio::join!(
        store.set_course_policy(&actor, &scope, managed.revision(), &first),
        other.set_course_policy(&actor, &scope, managed.revision(), &second)
    );
    assert!(matches!(
        (&left, &right),
        (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
    ));
    let current = store
        .parental(&context("learner-a"), &scope)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        current.policy().unwrap(),
        if left.is_ok() { &first } else { &second }
    );
}

#[tokio::test]
async fn a_completed_pause_fences_events_from_another_pool() {
    let (temp, store, clock) = fixture().await;
    let other = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    let actor = context("learner-a");
    let scope = scope();
    let session = store
        .start_exam(&actor, &scope, &policy(), &acknowledgement())
        .await
        .unwrap();
    let (pause, event) = tokio::join!(
        store.pause_exam(&actor, &scope, session.id(), session.revision()),
        other.record_visibility(
            &actor,
            &scope,
            session.id(),
            session.revision(),
            1,
            VisibilityEvent::PageHidden
        )
    );
    let paused = pause.unwrap();
    assert!(event.is_ok() || matches!(event, Err(Error::Conflict)));
    assert!(matches!(
        other
            .record_visibility(
                &actor,
                &scope,
                session.id(),
                paused.revision(),
                2,
                VisibilityEvent::PageVisible
            )
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        other
            .session(&actor, &scope, session.id())
            .await
            .unwrap()
            .state(),
        SessionState::Paused
    );
}

#[tokio::test]
async fn expiry_while_waiting_for_a_writer_rejects_without_partial_event() {
    let (temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement())
        .await
        .unwrap();
    let mut lock = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut lock)
        .await
        .unwrap();
    let calls = clock.calls.load(Ordering::SeqCst);
    let worker = store.clone();
    let id = session.id().clone();
    let revision = session.revision();
    let pending = tokio::spawn(async move {
        worker
            .record_visibility(
                &context("learner-a"),
                &scope(),
                &id,
                revision,
                1,
                VisibilityEvent::PageHidden,
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while clock.calls.load(Ordering::SeqCst) == calls {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    clock.set(1300);
    sqlx::query("ROLLBACK").execute(&mut lock).await.unwrap();
    assert!(matches!(pending.await.unwrap(), Err(Error::Expired)));
    assert!(
        store
            .events(&actor, &scope(), session.id(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn cancelling_a_blocked_operation_does_not_acknowledge_or_commit_it() {
    let (temp, store, clock) = fixture().await;
    let mut lock = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut lock)
        .await
        .unwrap();
    let calls = clock.calls.load(Ordering::SeqCst);
    let worker = store.clone();
    let pending = tokio::spawn(async move { worker.enroll_parental(&operator(), &scope()).await });
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while clock.calls.load(Ordering::SeqCst) == calls {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    sqlx::query("ROLLBACK").execute(&mut lock).await.unwrap();
    assert!(
        store
            .parental(&context("learner-a"), &scope())
            .await
            .unwrap()
            .is_none()
    );
}
