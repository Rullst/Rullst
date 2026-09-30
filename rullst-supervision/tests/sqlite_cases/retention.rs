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
