use super::*;

#[tokio::test]
async fn visible_session_controls_and_event_sequences_survive_pool_reopen() {
    let (temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    assert!(matches!(
        store
            .start_exam(
                &context("learner-b"),
                &scope(),
                &policy(),
                &acknowledgement(),
                None
            )
            .await,
        Err(Error::Forbidden)
    ));
    let wrong = Acknowledgement::new("old-policy", "notice-v1", true).unwrap();
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &wrong, None)
            .await,
        Err(Error::Conflict)
    ));
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
            .await,
        Err(Error::Conflict)
    ));
    let first = store
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
    assert_eq!(first.received_at(), 1000);
    assert!(matches!(
        store
            .record_visibility(
                &actor,
                &scope(),
                session.id(),
                session.revision(),
                1,
                VisibilityEvent::PageHidden
            )
            .await,
        Err(Error::Sequence)
    ));
    assert!(matches!(
        store
            .record_visibility(
                &actor,
                &scope(),
                session.id(),
                session.revision(),
                2,
                VisibilityEvent::PageHidden
            )
            .await,
        Err(Error::RateLimited)
    ));
    let paused = store
        .pause_exam(&actor, &scope(), session.id(), session.revision())
        .await
        .unwrap();
    clock.set(1001);
    assert!(matches!(
        store
            .record_visibility(
                &actor,
                &scope(),
                session.id(),
                paused.revision(),
                2,
                VisibilityEvent::PageHidden
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store
            .resume_exam(
                &actor,
                &scope(),
                session.id(),
                session.revision(),
                &acknowledgement()
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store
            .resume_exam(&actor, &scope(), session.id(), paused.revision(), &wrong)
            .await,
        Err(Error::Conflict)
    ));
    let resumed = store
        .resume_exam(
            &actor,
            &scope(),
            session.id(),
            paused.revision(),
            &acknowledgement(),
        )
        .await
        .unwrap();
    assert_eq!(resumed.expires_at(), session.expires_at());
    store
        .record_visibility(
            &actor,
            &scope(),
            session.id(),
            resumed.revision(),
            2,
            VisibilityEvent::PageHidden,
        )
        .await
        .unwrap();
    let ended = store
        .end_exam(&actor, &scope(), session.id(), resumed.revision())
        .await
        .unwrap();
    assert_eq!(ended.state(), SessionState::Ended);
    assert!(matches!(
        store
            .resume_exam(
                &actor,
                &scope(),
                session.id(),
                ended.revision(),
                &acknowledgement()
            )
            .await,
        Err(Error::Conflict)
    ));
    store.close().await;
    let reopened = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    let read = reopened
        .session(&actor, &scope(), session.id())
        .await
        .unwrap();
    assert_eq!(read.state(), SessionState::Ended);
    assert_eq!(read.last_sequence(), 2);
    assert_eq!(
        reopened
            .events(&actor, &scope(), session.id(), 0, 100)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        reopened
            .events(&actor, &scope(), session.id(), 1, 1)
            .await
            .unwrap()[0]
            .event(),
        VisibilityEvent::PageHidden
    );
}

#[tokio::test]
async fn review_is_scoped_separate_from_parental_authority_and_revocable() {
    let (_temp, store, _) = fixture().await;
    let session = store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy(),
            &acknowledgement(),
            None,
        )
        .await
        .unwrap();
    for denied in [
        context("delegate"),
        context("teacher"),
        Context::new("school-b", "learner-a").unwrap(),
    ] {
        assert!(matches!(
            store.session(&denied, &scope(), session.id()).await,
            Err(Error::Forbidden)
        ));
        assert!(matches!(
            store.events(&denied, &scope(), session.id(), 0, 10).await,
            Err(Error::Forbidden)
        ));
    }
    store
        .provision_authority(
            &operator(),
            &key(AuthorityAction::ParentalManage),
            None,
            2000,
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .session(&context("delegate"), &scope(), session.id())
            .await,
        Err(Error::Forbidden)
    ));
    let grant = store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    assert!(
        store
            .session(&context("delegate"), &scope(), session.id())
            .await
            .is_ok()
    );
    let other = Scope::new("school-a", "learner-b", "resource-a").unwrap();
    assert!(matches!(
        store
            .session(&context("delegate"), &other, session.id())
            .await,
        Err(Error::Forbidden)
    ));
    store
        .revoke_authority(&operator(), grant.key(), grant.revision())
        .await
        .unwrap();
    assert!(matches!(
        store
            .events(&context("delegate"), &scope(), session.id(), 0, 10)
            .await,
        Err(Error::Forbidden)
    ));
    assert!(
        store
            .session(&context("learner-a"), &scope(), session.id())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn expiry_and_retention_deny_reads_before_bounded_purge() {
    let (_temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let grant = store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 1100)
        .await
        .unwrap();
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
    clock.set(1100);
    assert!(matches!(
        store
            .session(&context("delegate"), &scope(), session.id())
            .await,
        Err(Error::Forbidden)
    ));
    clock.set(1300);
    assert_eq!(
        store
            .session(&actor, &scope(), session.id())
            .await
            .unwrap()
            .state(),
        SessionState::Expired
    );
    assert!(matches!(
        store
            .record_visibility(
                &actor,
                &scope(),
                session.id(),
                session.revision(),
                2,
                VisibilityEvent::PageVisible
            )
            .await,
        Err(Error::Expired)
    ));
    clock.set(4600);
    assert!(
        store
            .events(&actor, &scope(), session.id(), 0, 100)
            .await
            .unwrap()
            .is_empty()
    );
    let purge = store.purge_expired(&operator(), 1).await.unwrap();
    assert_eq!((purge.events, purge.sessions, purge.grants), (1, 0, 0));
    clock.set(4900);
    assert!(matches!(
        store.session(&actor, &scope(), session.id()).await,
        Err(Error::Expired)
    ));
    let purge = store.purge_expired(&operator(), 100).await.unwrap();
    assert_eq!((purge.events, purge.sessions, purge.grants), (0, 1, 1));
    let replacement = store
        .provision_authority(&operator(), grant.key(), None, 5900)
        .await
        .unwrap();
    assert!(replacement.revision().value() > grant.revision().value());
    assert!(matches!(
        store
            .revoke_authority(&operator(), grant.key(), grant.revision())
            .await,
        Err(Error::Conflict)
    ));
}

#[tokio::test]
async fn ended_session_requires_a_fresh_start_revision_even_after_reopen() {
    let (temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    assert!(store.latest_exam(&actor, &scope()).await.unwrap().is_none());
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert_eq!(
        store
            .latest_exam(&actor, &scope())
            .await
            .unwrap()
            .unwrap()
            .id(),
        session.id()
    );
    let ended = store
        .end_exam(&actor, &scope(), session.id(), session.revision())
        .await
        .unwrap();
    store.close().await;
    let store = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    assert_eq!(
        store
            .latest_exam(&actor, &scope())
            .await
            .unwrap()
            .unwrap()
            .revision(),
        ended.revision()
    );
    for stale in [None, Some(session.revision())] {
        assert!(matches!(
            store
                .start_exam(&actor, &scope(), &policy(), &acknowledgement(), stale)
                .await,
            Err(Error::Conflict)
        ));
    }
    assert!(matches!(
        store.latest_exam(&context("learner-b"), &scope()).await,
        Err(Error::Forbidden)
    ));
    let next = store
        .start_exam(
            &actor,
            &scope(),
            &policy(),
            &acknowledgement(),
            Some(ended.revision()),
        )
        .await
        .unwrap();
    assert_ne!(session.id(), next.id());
    assert!(matches!(
        store
            .start_exam(
                &actor,
                &scope(),
                &policy(),
                &acknowledgement(),
                Some(ended.revision())
            )
            .await,
        Err(Error::Conflict)
    ));
}

#[tokio::test]
async fn latest_session_preserves_ended_state_and_expires_at_exact_boundaries() {
    let (_temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    let other = Scope::new("school-a", "learner-a", "other-resource").unwrap();
    assert!(store.latest_exam(&actor, &other).await.unwrap().is_none());
    let ended = store
        .start_exam(&actor, &other, &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    store
        .end_exam(&actor, &other, ended.id(), ended.revision())
        .await
        .unwrap();
    clock.set(session.expires_at() - 1);
    assert_eq!(
        store
            .latest_exam(&actor, &scope())
            .await
            .unwrap()
            .unwrap()
            .state(),
        SessionState::Active
    );
    assert_eq!(
        store
            .latest_exam(&actor, &other)
            .await
            .unwrap()
            .unwrap()
            .state(),
        SessionState::Ended
    );
    clock.set(session.expires_at());
    let expired = store.latest_exam(&actor, &scope()).await.unwrap().unwrap();
    assert_eq!(expired.state(), SessionState::Expired);
    assert_eq!(expired.revision(), session.revision());
    assert_eq!(
        store
            .latest_exam(&actor, &other)
            .await
            .unwrap()
            .unwrap()
            .state(),
        SessionState::Ended
    );
    // Retained expired state still binds the next start form until retention ends.
    assert!(matches!(
        store
            .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
            .await,
        Err(Error::Conflict)
    ));
    clock.set(session.expires_at() + 3600);
    assert!(store.latest_exam(&actor, &scope()).await.unwrap().is_none());
    assert!(store.latest_exam(&actor, &other).await.unwrap().is_none());
    let replacement = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert_ne!(replacement.id(), session.id());
}
