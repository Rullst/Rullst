//! Trusted time, revisions and store-wide bounds across whole operations.
use super::rows::{raw, run};
use super::*;
use rullst_supervision::parental::CoursePolicy;
use std::{collections::VecDeque, sync::Mutex};

/// Returns queued times in order, then keeps returning the last one.
#[derive(Clone)]
struct Script(Arc<Mutex<(VecDeque<i64>, i64)>>);
impl Script {
    fn then(&self, times: &[i64]) {
        self.0.lock().unwrap().0.extend(times);
    }
}
impl Clock for Script {
    fn now(&self) -> Result<i64, Error> {
        let mut state = self.0.lock().unwrap();
        if let Some(next) = state.0.pop_front() {
            state.1 = next;
        }
        Ok(state.1)
    }
}

async fn scripted() -> (tempfile::TempDir, SqliteSupervision<Script>, Script) {
    let temp = tempfile::tempdir().unwrap();
    let clock = Script(Arc::new(Mutex::new((VecDeque::new(), 1000))));
    let store = SqliteSupervision::initialize(
        temp.path().join("supervision.sqlite"),
        config(),
        clock.clone(),
    )
    .await
    .unwrap();
    (temp, store, clock)
}

#[tokio::test]
async fn time_may_advance_but_never_regress_within_an_operation() {
    let key = key(AuthorityAction::ExamReview);
    let (_temp, store, clock) = scripted().await;
    // Begin reads 1001 and 1002, the commit checks 1003 and 1004 and
    // confirms at 1005.
    clock.then(&[1001, 1002, 1003, 1004, 1005]);
    assert!(store.authority(&operator(), &key).await.unwrap().is_none());

    let (_temp, store, clock) = scripted().await;
    // The commit check reads 1003, after the operation began at 1005.
    clock.then(&[1005, 1005, 1003]);
    assert!(matches!(
        store.authority(&operator(), &key).await,
        Err(Error::Clock)
    ));
}

#[tokio::test]
async fn reaching_a_deadline_only_after_the_commit_is_uncertain() {
    let (_temp, store, clock) = scripted().await;
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    // The active session's deadline is 1300: every check before the commit
    // reads 1299 and the confirmation reads 1300.
    clock.then(&[1299, 1299, 1299, 1299, 1300]);
    assert!(matches!(
        store.session(&actor, &scope(), session.id()).await,
        Err(Error::UncertainCommit)
    ));
}

#[tokio::test]
async fn a_completed_operation_keeps_the_time_high_water_mark() {
    let (_temp, store, clock) = fixture().await;
    let key = key(AuthorityAction::ExamReview);
    store.authority(&operator(), &key).await.unwrap();
    clock.set(999);
    assert!(matches!(
        store.authority(&operator(), &key).await,
        Err(Error::Clock)
    ));
}

#[tokio::test]
async fn course_rows_are_bounded_by_64_per_managed_learner() {
    let (temp, store, _) = fixture().await;
    let mut db = raw(&temp).await;
    run(&mut db, "PRAGMA foreign_keys=OFF").await;
    // 81 rows exceed `managed + 64` but not the 16 * 64 the store allows.
    run(
        &mut db,
        "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 81) \
         INSERT INTO rullst_supervision_courses SELECT 'other','learner-x','resource-x','course-' || i FROM n",
    )
    .await;
    assert!(
        store
            .authority(&operator(), &key(AuthorityAction::ExamReview))
            .await
            .is_ok()
    );
}

async fn meta_revision(db: &mut sqlx::SqliteConnection) -> i64 {
    sqlx::query_scalar("SELECT revision FROM rullst_supervision_meta")
        .fetch_one(&mut *db)
        .await
        .unwrap()
}

#[tokio::test]
async fn only_a_purge_that_removes_rows_issues_a_revision() {
    let (temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    store
        .record_visibility(
            &actor,
            &scope(),
            session.id(),
            session.revision(),
            1,
            VisibilityEvent::PageVisible,
        )
        .await
        .unwrap();
    store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    let mut db = raw(&temp).await;
    let before = meta_revision(&mut db).await;
    let nothing = store.purge_expired(&operator(), 10).await.unwrap();
    assert_eq!(
        (nothing.events, nothing.sessions, nothing.grants),
        (0, 0, 0)
    );
    assert_eq!(meta_revision(&mut db).await, before);
    clock.set(2000);
    let grants = store.purge_expired(&operator(), 10).await.unwrap();
    assert_eq!((grants.events, grants.sessions, grants.grants), (0, 0, 1));
    assert_eq!(meta_revision(&mut db).await, before + 1);
    clock.set(4600);
    let events = store.purge_expired(&operator(), 10).await.unwrap();
    assert_eq!((events.events, events.sessions, events.grants), (1, 0, 0));
    assert_eq!(meta_revision(&mut db).await, before + 2);
}

#[tokio::test]
async fn the_store_wide_event_budget_counts_only_removed_events() {
    let (_temp, store, clock) = fixture().await;
    let learners = ["learner-a", "learner-b"].map(|subject| {
        (
            context(subject),
            Scope::new("school-a", subject, "resource-a").unwrap(),
        )
    });
    let mut sessions = Vec::new();
    for (actor, scope) in &learners {
        sessions.push(
            store
                .start_exam(actor, scope, &policy(), &acknowledgement(), None)
                .await
                .unwrap(),
        );
    }
    // 64 unexpired events each fill the store-wide budget of 128.
    for sequence in 1..=64 {
        clock.set(999 + sequence);
        for ((actor, scope), session) in learners.iter().zip(&sessions) {
            let receipt = store
                .record_visibility(
                    actor,
                    scope,
                    session.id(),
                    session.revision(),
                    sequence,
                    VisibilityEvent::PageVisible,
                )
                .await
                .unwrap();
            assert_eq!(receipt.sequence(), sequence);
        }
    }
    let (actor, scope) = &learners[0];
    let session = store.session(actor, scope, sessions[0].id()).await.unwrap();
    assert_eq!(session.accepted_event_count(), 64);
    assert_eq!(session.last_event_at(), Some(1063));
    clock.set(1064);
    assert!(matches!(
        store
            .record_visibility(
                actor,
                scope,
                sessions[0].id(),
                sessions[0].revision(),
                65,
                VisibilityEvent::PageHidden,
            )
            .await,
        Err(Error::Capacity)
    ));
}

#[tokio::test]
async fn provisioned_authority_lasts_at_most_30_days_and_until_the_last_valid_time() {
    let (_temp, store, clock) = fixture().await;
    let review = key(AuthorityAction::ExamReview);
    for expires_at in [1000, 1000 + 2_592_001] {
        assert!(matches!(
            store
                .provision_authority(&operator(), &review, None, expires_at)
                .await,
            Err(Error::InvalidInput)
        ));
    }
    let grant = store
        .provision_authority(&operator(), &review, None, 1000 + 2_592_000)
        .await
        .unwrap();
    assert_eq!(grant.expires_at(), 1000 + 2_592_000);
    const MAX: i64 = 253_402_300_799;
    clock.set(MAX - 1000);
    let manage = key(AuthorityAction::ParentalManage);
    assert!(matches!(
        store
            .provision_authority(&operator(), &manage, None, MAX + 1)
            .await,
        Err(Error::InvalidInput)
    ));
    let last = store
        .provision_authority(&operator(), &manage, None, MAX)
        .await
        .unwrap();
    assert_eq!(last.expires_at(), MAX);
}

#[tokio::test]
async fn a_course_policy_records_the_delegate_that_set_it() {
    let (_temp, store, _) = fixture().await;
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
    let policy = CoursePolicy::new(["course-a"], 1001, 1500).unwrap();
    store
        .set_course_policy(&context("delegate"), &scope(), managed.revision(), &policy)
        .await
        .unwrap();
    let learner = store
        .parental(&context("learner-a"), &scope())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        learner.policy_actor().map(OpaqueId::as_str),
        Some("delegate")
    );
}

#[tokio::test]
async fn stored_configuration_schema_paths_and_closed_pools_fail_closed() {
    let (temp, store, _) = fixture().await;
    let mut db = raw(&temp).await;
    let stored: String = sqlx::query_scalar("SELECT config FROM rullst_supervision_meta")
        .fetch_one(&mut db)
        .await
        .unwrap();
    // Default per-learner quotas keep the key that predates them.
    assert_eq!(stored, "v2|deployment-v1|16|16|128|16|1024|1|3600|600");
    store.close().await;
    assert!(matches!(
        store
            .authority(&operator(), &key(AuthorityAction::ExamReview))
            .await,
        Err(Error::Storage)
    ));
    run(
        &mut db,
        "ALTER TABLE rullst_supervision_courses ADD COLUMN extra TEXT",
    )
    .await;
    assert!(matches!(
        SqliteSupervision::open(
            temp.path().join("supervision.sqlite"),
            config(),
            TestClock::new()
        )
        .await,
        Err(Error::Configuration)
    ));
    assert!(matches!(
        SqliteSupervision::open(temp.path(), config(), TestClock::new()).await,
        Err(Error::Configuration)
    ));

    let explicit = tempfile::tempdir().unwrap();
    let limits = Limits::new(16, 16, 128, 16)
        .unwrap()
        .subject_sessions(8)
        .unwrap()
        .subject_events(100)
        .unwrap();
    let store = SqliteSupervision::initialize(
        explicit.path().join("supervision.sqlite"),
        StoreConfig::new("deployment-v1", limits, 3600, 600).unwrap(),
        TestClock::new(),
    )
    .await
    .unwrap();
    store.close().await;
    let mut db = super::rows::raw(&explicit).await;
    let stored: String = sqlx::query_scalar("SELECT config FROM rullst_supervision_meta")
        .fetch_one(&mut db)
        .await
        .unwrap();
    assert_eq!(
        stored,
        "v2|deployment-v1|16|16|128|16|1024|1|3600|600|subject-sessions=8|subject-events=100"
    );
}

#[tokio::test]
async fn a_session_may_be_retained_until_the_last_valid_time() {
    const MAX: i64 = 253_402_300_799;
    let (_temp, store, clock) = fixture().await;
    // 300 seconds of exam plus 3600 of retention end exactly at MAX.
    clock.set(MAX - 3900);
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert_eq!(session.retain_until(), MAX);
    let loaded = store.session(&actor, &scope(), session.id()).await.unwrap();
    assert_eq!(loaded.retain_until(), MAX);
}
