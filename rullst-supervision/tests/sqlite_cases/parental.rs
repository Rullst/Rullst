use super::*;

#[tokio::test]
async fn enrollment_restricts_original_access_decision_until_operator_removal() {
    let (temp, store, clock) = fixture().await;
    let actor = context("learner-a");
    let course = OpaqueId::new("course-a").unwrap();
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::Unmanaged
    );
    let managed = store.enroll_parental(&operator(), &scope()).await.unwrap();
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::MissingPolicy
    );
    let policy = CoursePolicy::new(["course-a"], 1001, 1500).unwrap();
    for denied in [
        actor.clone(),
        context("teacher"),
        context("delegate"),
        Context::new("school-b", "delegate").unwrap(),
    ] {
        assert!(matches!(
            store
                .set_course_policy(&denied, &scope(), managed.revision(), &policy)
                .await,
            Err(Error::Forbidden)
        ));
    }
    let grant = store
        .provision_authority(
            &operator(),
            &key(AuthorityAction::ParentalManage),
            None,
            2000,
        )
        .await
        .unwrap();
    let updated = store
        .set_course_policy(&context("delegate"), &scope(), managed.revision(), &policy)
        .await
        .unwrap();
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::OutsideWindow
    );
    clock.set(1001);
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::Allowed
    );
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &OpaqueId::new("course-b").unwrap())
            .await
            .unwrap(),
        AccessDecision::CourseDenied
    );
    assert!(matches!(
        store
            .set_course_policy(&context("delegate"), &scope(), managed.revision(), &policy)
            .await,
        Err(Error::Conflict)
    ));
    store
        .revoke_authority(&operator(), grant.key(), grant.revision())
        .await
        .unwrap();
    assert!(matches!(
        store
            .set_course_policy(&context("delegate"), &scope(), updated.revision(), &policy)
            .await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store.parental(&context("delegate"), &scope()).await,
        Err(Error::Forbidden)
    ));
    assert_eq!(
        store
            .learning_access(&actor, &scope(), &OpaqueId::new("course-b").unwrap())
            .await
            .unwrap(),
        AccessDecision::CourseDenied
    );
    store.close().await;
    let reopened = SqliteSupervision::open(
        temp.path().join("supervision.sqlite"),
        config(),
        clock.clone(),
    )
    .await
    .unwrap();
    assert_eq!(
        reopened
            .parental(&actor, &scope())
            .await
            .unwrap()
            .unwrap()
            .revision(),
        updated.revision()
    );
    clock.set(1500);
    assert_eq!(
        reopened
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::OutsideWindow
    );
    assert!(matches!(
        reopened
            .remove_parental(&operator(), &scope(), managed.revision())
            .await,
        Err(Error::Conflict)
    ));
    reopened
        .remove_parental(&operator(), &scope(), updated.revision())
        .await
        .unwrap();
    assert_eq!(
        reopened
            .learning_access(&actor, &scope(), &course)
            .await
            .unwrap(),
        AccessDecision::Unmanaged
    );
    let next = reopened
        .enroll_parental(&operator(), &scope())
        .await
        .unwrap();
    assert!(next.revision().value() > updated.revision().value());
    assert!(matches!(
        reopened
            .remove_parental(&operator(), &scope(), updated.revision())
            .await,
        Err(Error::Conflict)
    ));
}

#[tokio::test]
async fn same_learner_and_resource_names_cannot_cross_tenants_or_authority_actions() {
    let (_temp, store, _) = fixture().await;
    let foreign = Scope::new("school-b", "learner-a", "resource-a").unwrap();
    assert!(matches!(
        store.enroll_parental(&operator(), &foreign).await,
        Err(Error::Forbidden)
    ));
    let managed = store.enroll_parental(&operator(), &scope()).await.unwrap();
    store
        .provision_authority(&operator(), &key(AuthorityAction::ExamReview), None, 2000)
        .await
        .unwrap();
    assert!(matches!(
        store
            .set_course_policy(
                &context("delegate"),
                &scope(),
                managed.revision(),
                &CoursePolicy::new(["course-a"], 0, 2000).unwrap()
            )
            .await,
        Err(Error::Forbidden)
    ));
    let foreign_operator =
        Operator::new(Context::new("school-b", "operator").unwrap(), "evidence-b").unwrap();
    store
        .enroll_parental(&foreign_operator, &foreign)
        .await
        .unwrap();
    let learner_b = Context::new("school-b", "learner-a").unwrap();
    assert!(matches!(
        store.parental(&learner_b, &scope()).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store
            .learning_access(&learner_b, &scope(), &OpaqueId::new("course-a").unwrap())
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(
        store
            .learning_access(&learner_b, &foreign, &OpaqueId::new("course-a").unwrap())
            .await
            .unwrap(),
        AccessDecision::MissingPolicy
    );
}
