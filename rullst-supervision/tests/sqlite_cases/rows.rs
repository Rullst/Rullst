//! Each stored bound is checked on its own: a row changed by an external
//! writer that breaks exactly one of them is a configuration error.
use super::*;
use rullst_supervision::exam::{BrowserEvent, Capability, Collection, ObservationRequest};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

/// A raw connection that, like an external writer, ignores CHECK constraints.
pub(super) async fn raw(temp: &tempfile::TempDir) -> SqliteConnection {
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    run(&mut connection, "PRAGMA ignore_check_constraints=ON").await;
    connection
}

pub(super) async fn run(connection: &mut SqliteConnection, sql: &'static str) {
    sqlx::query(sql).execute(&mut *connection).await.unwrap();
}

#[tokio::test]
async fn every_stored_session_bound_is_checked_on_its_own() {
    let (temp, store, _) = fixture().await;
    let actor = context("learner-a");
    let session = store
        .start_exam(&actor, &scope(), &policy(), &acknowledgement(), None)
        .await
        .unwrap();
    assert_eq!(session.started_at(), 1000);
    assert_eq!(session.last_event_at(), None);
    let mut db = raw(&temp).await;
    // Started at 1000, expiring at 1300 and retained until 4900; now is 1000.
    let reset = "UPDATE rullst_supervision_sessions SET started_at=1000,expires_at=1300,\
                 retain_until=4900,sequence=0,event_count=0,last_event_at=NULL";
    for change in [
        "UPDATE rullst_supervision_sessions SET started_at=-1,expires_at=299,retain_until=3899",
        "UPDATE rullst_supervision_sessions SET started_at=1001",
        "UPDATE rullst_supervision_sessions SET expires_at=1000,retain_until=4600",
        "UPDATE rullst_supervision_sessions SET expires_at=1601,retain_until=5201",
        "UPDATE rullst_supervision_sessions SET sequence=-1,event_count=-1,last_event_at=1000",
        "UPDATE rullst_supervision_sessions SET sequence=1,event_count=2,last_event_at=1000",
        "UPDATE rullst_supervision_sessions SET sequence=1025,event_count=1025,last_event_at=1000",
        "UPDATE rullst_supervision_sessions SET last_event_at=1000",
        "UPDATE rullst_supervision_sessions SET sequence=1,event_count=1,last_event_at=999",
        "UPDATE rullst_supervision_sessions SET sequence=1,event_count=1,last_event_at=1001",
        // A revision the store has not issued yet.
        "UPDATE rullst_supervision_sessions SET revision=revision+1000",
    ] {
        run(&mut db, reset).await;
        run(&mut db, change).await;
        assert!(
            matches!(
                store.session(&actor, &scope(), session.id()).await,
                Err(Error::Configuration)
            ),
            "{change}"
        );
    }
    run(
        &mut db,
        "UPDATE rullst_supervision_sessions SET revision=revision-1000",
    )
    .await;
    // A session may start at the epoch.
    run(
        &mut db,
        "UPDATE rullst_supervision_sessions SET started_at=0,expires_at=300,retain_until=3900,\
         sequence=0,event_count=0,last_event_at=NULL",
    )
    .await;
    let restored = store.session(&actor, &scope(), session.id()).await.unwrap();
    assert_eq!(restored.started_at(), 0);
}

#[tokio::test]
async fn every_stored_event_bound_is_checked_on_its_own() {
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
    let mut db = raw(&temp).await;
    let reset = "UPDATE rullst_supervision_events SET sequence=1,received_at=1000,expires_at=4600";
    for change in [
        "UPDATE rullst_supervision_events SET sequence=2",
        "UPDATE rullst_supervision_events SET received_at=999,expires_at=4599",
        "UPDATE rullst_supervision_events SET received_at=1001,expires_at=4601",
        "UPDATE rullst_supervision_events SET expires_at=4601",
    ] {
        run(&mut db, reset).await;
        run(&mut db, change).await;
        assert!(
            matches!(
                store.events(&actor, &scope(), session.id(), 0, 10).await,
                Err(Error::Configuration)
            ),
            "{change}"
        );
        assert!(
            matches!(
                store
                    .observations(&actor, &scope(), session.id(), 0, 10)
                    .await,
                Err(Error::Configuration)
            ),
            "{change}"
        );
    }
    // Received at the session's expiry, which has now passed.
    clock.set(1300);
    run(
        &mut db,
        "UPDATE rullst_supervision_events SET sequence=1,received_at=1300,expires_at=4900",
    )
    .await;
    assert!(matches!(
        store.events(&actor, &scope(), session.id(), 0, 10).await,
        Err(Error::Configuration)
    ));
    assert!(matches!(
        store
            .observations(&actor, &scope(), session.id(), 0, 10)
            .await,
        Err(Error::Configuration)
    ));
    run(&mut db, reset).await;
    assert_eq!(
        store
            .events(&actor, &scope(), session.id(), 0, 10)
            .await
            .unwrap()
            .len(),
        1
    );
    for (after, limit) in [(-1, 1), (0, 0), (0, 101)] {
        assert!(matches!(
            store
                .events(&actor, &scope(), session.id(), after, limit)
                .await,
            Err(Error::InvalidInput)
        ));
    }
}

#[tokio::test]
async fn an_adapter_kind_stored_as_a_browser_observation_is_rejected() {
    let (temp, store, clock) = fixture().await;
    let collection = Collection::new([Capability::Visibility, Capability::AudioActivity]).unwrap();
    let session = super::observations::start(&store, collection).await;
    let actor = context("learner-a");
    let scope = scope();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    store
        .record_browser(request, BrowserEvent::PageVisible)
        .await
        .unwrap();
    clock.set(1001);
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 2).unwrap();
    let receipt = store
        .record_browser(request, BrowserEvent::PageHidden)
        .await
        .unwrap();
    assert_eq!(receipt.sequence(), 2);
    let mut db = raw(&temp).await;
    // 25 is an audio-activity code, which only an adapter may report.
    run(&mut db, "UPDATE rullst_supervision_events SET kind=25").await;
    assert!(matches!(
        store
            .observations(&actor, &scope, session.id(), 0, 10)
            .await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn stored_grants_and_course_rows_must_be_consistent() {
    let (temp, store, _) = fixture().await;
    let key = key(AuthorityAction::ExamReview);
    let grant = store
        .provision_authority(&operator(), &key, None, 2000)
        .await
        .unwrap();
    assert_eq!(grant.expires_at(), 2000);
    assert!(!grant.is_revoked());
    let mut db = raw(&temp).await;
    for (change, restore) in [
        (
            "UPDATE rullst_supervision_grants SET revoked=2",
            "UPDATE rullst_supervision_grants SET revoked=0",
        ),
        (
            "UPDATE rullst_supervision_grants SET expires_at=0",
            "UPDATE rullst_supervision_grants SET expires_at=2000",
        ),
    ] {
        run(&mut db, change).await;
        assert!(
            matches!(
                store.authority(&operator(), &key).await,
                Err(Error::Configuration)
            ),
            "{change}"
        );
        run(&mut db, restore).await;
    }
    let revoked = store
        .revoke_authority(&operator(), &key, grant.revision())
        .await
        .unwrap();
    assert!(revoked.is_revoked());

    store.enroll_parental(&operator(), &scope()).await.unwrap();
    run(
        &mut db,
        "INSERT INTO rullst_supervision_courses VALUES ('school-a','learner-a','resource-a','course-a')",
    )
    .await;
    assert!(matches!(
        store
            .learning_access(
                &context("learner-a"),
                &scope(),
                &OpaqueId::new("course-a").unwrap()
            )
            .await,
        Err(Error::Configuration)
    ));
}
