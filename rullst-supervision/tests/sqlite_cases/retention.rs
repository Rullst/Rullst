use super::*;

#[tokio::test]
async fn an_older_retained_session_stays_latest_after_a_shorter_newer_one_leaves_retention() {
    let (_temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let long = ExamPolicy::new("policy-v1", "notice-v1", 600).unwrap();
    let short = ExamPolicy::new("policy-v1", "notice-v1", 60).unwrap();
    let first = store
        .start_exam(&actor, &scope(), &long, &acknowledgement(), None)
        .await
        .unwrap();
    let first = store
        .end_exam(&actor, &scope(), first.id(), first.revision())
        .await
        .unwrap();
    let second = store
        .start_exam(
            &actor,
            &scope(),
            &short,
            &acknowledgement(),
            Some(first.revision()),
        )
        .await
        .unwrap();
    assert!(second.retain_until() < first.retain_until());
    clock.set(second.retain_until());
    let latest = store.latest_exam(&actor, &scope()).await.unwrap().unwrap();
    assert_eq!(latest.id(), first.id());
    assert_eq!(latest.revision(), first.revision());
    // `None` means no retained session, so it cannot start over the older one.
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
            .await,
        Err(Error::Conflict)
    ));
    store
        .start_exam(
            &actor,
            &scope(),
            &policy(),
            &acknowledgement(),
            Some(first.revision()),
        )
        .await
        .unwrap();
}

async fn bounded(limits: Limits) -> (tempfile::TempDir, SqliteSupervision<TestClock>, TestClock) {
    let temp = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let store = SqliteSupervision::initialize(
        temp.path().join("bounded.sqlite"),
        StoreConfig::new("epoch", limits, 3600, 600).unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    (temp, store, clock)
}

#[tokio::test]
async fn one_learner_cannot_fill_the_store_wide_session_budget_by_start_end_loops() {
    let (_temp, store, clock) = bounded(Limits::new(16, 128, 128, 16).unwrap()).await;
    let actor = context("learner-a");
    let mut previous = None;
    for _ in 0..64 {
        let started = store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), previous)
            .await
            .unwrap();
        let ended = store
            .end_exam(&actor, &scope(), started.id(), started.revision())
            .await
            .unwrap();
        previous = Some(ended.revision());
    }
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), previous)
            .await,
        Err(Error::Capacity)
    ));
    let other_resource = Scope::new("school-a", "learner-a", "resource-b").unwrap();
    assert!(matches!(
        store
            .start_exam(&actor, &other_resource, &policy(), &acknowledgement(), None)
            .await,
        Err(Error::Capacity)
    ));
    // Other learners, in this tenant or another, still start sessions.
    let classmate = Scope::new("school-a", "learner-b", "resource-a").unwrap();
    store
        .start_exam(
            &context("learner-b"),
            &classmate,
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    let elsewhere = Scope::new("school-b", "learner-a", "resource-a").unwrap();
    store
        .start_exam(
            &Context::new("school-b", "learner-a").unwrap(),
            &elsewhere,
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    // The learner's slots return once those sessions leave retention.
    clock.set(1000 + 300 + 3600);
    store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn sessions_past_retention_never_block_admission_in_another_tenant() {
    let (_temp, store, clock) = bounded(Limits::new(16, 1, 128, 16).unwrap()).await;
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
            VisibilityEvent::PageHidden,
        )
        .await
        .unwrap();
    let other = Scope::new("school-b", "learner-b", "resource-a").unwrap();
    let other_context = Context::new("school-b", "learner-b").unwrap();
    assert!(matches!(
        store
            .start_exam(&other_context, &other, &policy(), &acknowledgement(), None)
            .await,
        Err(Error::Capacity)
    ));
    // Only school-a's operator could purge this row; admission reclaims it
    // once it is past retention instead of failing for every tenant.
    clock.set(session.retain_until());
    store
        .start_exam(&other_context, &other, &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert!(matches!(
        store.session(&actor, &scope(), session.id()).await,
        Err(Error::Forbidden)
    ));
    // Its expired events went with it; the tenant's operator finds nothing left.
    let receipt = store.purge_expired(&operator(), 100).await.unwrap();
    assert_eq!((receipt.events, receipt.sessions), (0, 0));
}

#[tokio::test]
async fn expired_events_never_block_new_observations() {
    let (_temp, store, clock) = bounded(Limits::new(16, 16, 1, 16).unwrap()).await;
    let actor = context("learner-a");
    let first = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    let receipt = store
        .record_visibility(
            &actor,
            &scope(),
            first.id(),
            first.revision(),
            1,
            VisibilityEvent::PageHidden,
        )
        .await
        .unwrap();
    clock.set(receipt.expires_at());
    let other = Scope::new("school-a", "learner-a", "resource-b").unwrap();
    let second = store
        .start_exam(&actor, &other, &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    store
        .record_visibility(
            &actor,
            &other,
            second.id(),
            second.revision(),
            1,
            VisibilityEvent::PageVisible,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn an_explicit_learner_quota_is_enforced_and_bound_into_the_configuration() {
    let limits = Limits::new(16, 16, 128, 16)
        .unwrap()
        .subject_sessions(2)
        .unwrap();
    let (temp, store, clock) = bounded(limits.clone()).await;
    let actor = context("learner-a");
    let mut previous = None;
    for _ in 0..2 {
        let started = store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), previous)
            .await
            .unwrap();
        let ended = store
            .end_exam(&actor, &scope(), started.id(), started.revision())
            .await
            .unwrap();
        previous = Some(ended.revision());
    }
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), previous)
            .await,
        Err(Error::Capacity)
    ));
    store.close().await;
    let path = temp.path().join("bounded.sqlite");
    let default = StoreConfig::new("epoch", Limits::new(16, 16, 128, 16).unwrap(), 3600, 600);
    assert!(matches!(
        SqliteSupervision::open(&path, default.unwrap(), clock.clone()).await,
        Err(Error::Configuration)
    ));
    let explicit = StoreConfig::new("epoch", limits, 3600, 600).unwrap();
    SqliteSupervision::open(&path, explicit, clock.clone())
        .await
        .unwrap();

    // Naming the default explicitly keeps the key of existing stores.
    let (temp, store, _) = fixture().await;
    store.close().await;
    let same = Limits::new(16, 16, 128, 16)
        .unwrap()
        .subject_sessions(16)
        .unwrap();
    SqliteSupervision::open(
        temp.path().join("supervision.sqlite"),
        StoreConfig::new("deployment-v1", same, 3600, 600).unwrap(),
        clock,
    )
    .await
    .unwrap();
}
