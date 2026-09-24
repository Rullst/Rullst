use super::*;

#[tokio::test]
async fn identical_learner_names_in_distinct_tenants_keep_independent_exam_state() {
    let (_temp, store, _) = fixture().await;
    let first_actor = context("learner-a");
    let first_scope = scope();
    let other_actor = Context::new("school-b", "learner-a").unwrap();
    let other_scope = Scope::new("school-b", "learner-a", "resource-a").unwrap();
    let first = store
        .start_exam(
            &first_actor,
            &first_scope,
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    let other = store
        .start_exam(
            &other_actor,
            &other_scope,
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    assert_ne!(first.id(), other.id());
    assert!(matches!(
        store.session(&other_actor, &first_scope, first.id()).await,
        Err(Error::Forbidden)
    ));
    assert!(
        store
            .session(&other_actor, &other_scope, first.id())
            .await
            .is_err()
    );
    store
        .record_visibility(
            &first_actor,
            &first_scope,
            first.id(),
            first.revision(),
            1,
            VisibilityEvent::PageHidden,
        )
        .await
        .unwrap();
    store
        .pause_exam(&first_actor, &first_scope, first.id(), first.revision())
        .await
        .unwrap();
    let unaffected = store
        .session(&other_actor, &other_scope, other.id())
        .await
        .unwrap();
    assert_eq!(unaffected.id(), other.id());
    assert_eq!(unaffected.scope(), &other_scope);
    assert_eq!(unaffected.state(), SessionState::Active);
    assert_eq!(unaffected.revision(), other.revision());
    assert_eq!(unaffected.last_sequence(), 0);
    assert_eq!(unaffected.accepted_event_count(), 0);
    assert_eq!(unaffected.last_event_at(), None);
    assert!(
        store
            .events(&other_actor, &other_scope, other.id(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
    // Identical sequence numbers and timestamps are valid in independent scopes.
    store
        .record_visibility(
            &other_actor,
            &other_scope,
            other.id(),
            other.revision(),
            1,
            VisibilityEvent::PageVisible,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .events(&first_actor, &first_scope, first.id(), 0, 100)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .events(&other_actor, &other_scope, other.id(), 0, 100)
            .await
            .unwrap()
            .len(),
        1
    );
    store.close().await;
}
