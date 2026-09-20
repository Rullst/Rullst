use super::*;
use rullst_supervision::{
    Revision,
    exam::{
        BrowserEvent as B, Capability as C, CaptureDevice, CaptureEvent, Collection, Observation,
        ObservationRequest, ObservationSource,
    },
};

pub(super) async fn start(
    store: &SqliteSupervision<TestClock>,
    collection: Collection,
) -> rullst_supervision::exam::Session {
    store
        .start_exam(
            &context("learner-a"),
            &scope(),
            &policy().with_collection(collection),
            &Acknowledgement::for_collection("policy-v1", "notice-v1", collection, true).unwrap(),
            None,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn acknowledgement_must_bind_exact_collection_and_disabled_events_never_advance_sequence() {
    let (_temp, store, _) = fixture().await;
    let selected = Collection::new([C::Visibility, C::WindowFocus]).unwrap();
    assert!(matches!(
        store
            .start_exam(
                &context("learner-a"),
                &scope(),
                &policy().with_collection(selected),
                &acknowledgement(),
                None
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(
        store
            .latest_exam(&context("learner-a"), &scope())
            .await
            .unwrap()
            .is_none()
    );
    let session = start(&store, selected).await;
    let actor = context("learner-a");
    let scope = scope();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    assert!(matches!(
        store.record_browser(request, B::CopyAttempt).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store
            .record_capture(request, CaptureDevice::Camera, CaptureEvent::Started)
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(
        store
            .session(&actor, &scope, session.id())
            .await
            .unwrap()
            .last_sequence(),
        0
    );
    assert!(
        store
            .record_browser(request, B::WindowBlurred)
            .await
            .is_ok()
    );
    assert!(matches!(
        store.record_browser(request, B::WindowFocused).await,
        Err(Error::Sequence)
    ));
    assert!(
        ObservationRequest::new(
            &context("reviewer"),
            &scope,
            session.id(),
            session.revision(),
            2
        )
        .is_err()
    );
    let wrong = Scope::new("school-a", "learner-a", "other-resource").unwrap();
    let wrong =
        ObservationRequest::new(&actor, &wrong, session.id(), session.revision(), 2).unwrap();
    assert!(matches!(
        store.record_browser(wrong, B::PageHidden).await,
        Err(Error::Forbidden)
    ));
}

#[tokio::test]
async fn persisted_browser_and_capture_observations_round_trip_without_becoming_legacy_visibility()
{
    let (temp, store, clock) = fixture().await;
    let selected = Collection::new(C::ALL).unwrap();
    let session = start(&store, selected).await;
    let actor = context("learner-a");
    let scope = scope();
    let browser = [
        B::PageVisible,
        B::PageHidden,
        B::WindowFocused,
        B::WindowBlurred,
        B::CopyAttempt,
        B::CutAttempt,
        B::PasteAttempt,
        B::FullscreenEntered,
        B::FullscreenExited,
    ];
    let mut expected = Vec::new();
    for event in browser {
        clock.set(1000 + expected.len() as i64);
        let request = ObservationRequest::new(
            &actor,
            &scope,
            session.id(),
            session.revision(),
            expected.len() as i64 + 1,
        )
        .unwrap();
        expected.push(store.record_browser(request, event).await.unwrap());
    }
    for device in [
        CaptureDevice::Camera,
        CaptureDevice::Microphone,
        CaptureDevice::ScreenShare,
    ] {
        for event in [
            CaptureEvent::Started,
            CaptureEvent::Stopped,
            CaptureEvent::PermissionDenied,
            CaptureEvent::Unavailable,
        ] {
            clock.set(1000 + expected.len() as i64);
            let request = ObservationRequest::new(
                &actor,
                &scope,
                session.id(),
                session.revision(),
                expected.len() as i64 + 1,
            )
            .unwrap();
            let receipt = store.record_capture(request, device, event).await.unwrap();
            assert_eq!(
                receipt.observation(),
                Observation::Capture { device, event }
            );
            expected.push(receipt);
        }
    }
    assert!(
        expected
            .iter()
            .all(|receipt| receipt.source() == &ObservationSource::Browser)
    );
    store.close().await;
    let reopened = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    assert_eq!(
        reopened
            .observations(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap(),
        expected
    );
    assert_eq!(
        reopened
            .observations(&actor, &scope, session.id(), 7, 3)
            .await
            .unwrap(),
        expected[7..10]
    );
    assert_eq!(
        reopened
            .events(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(
        reopened
            .observations(&actor, &scope, session.id(), 0, 101)
            .await
            .is_err()
    );
    assert!(
        reopened
            .observations(&context("reviewer"), &scope, session.id(), 0, 10)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn restricting_collection_fences_old_requests_preserves_history_and_requires_fresh_resume_ack()
 {
    let (temp, store, clock) = fixture().await;
    let selected = Collection::new([C::Visibility, C::ClipboardActivity]).unwrap();
    let session = start(&store, selected).await;
    let actor = context("learner-a");
    let scope = scope();
    let old = ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    let receipt = store.record_browser(old, B::CopyAttempt).await.unwrap();
    let current = store
        .restrict_collection(
            &actor,
            &scope,
            session.id(),
            session.revision(),
            Collection::visibility_only(),
        )
        .await
        .unwrap();
    assert_eq!(current.initial_collection(), selected);
    assert!(matches!(
        store.record_browser(old, B::PageHidden).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store
            .restrict_collection(&actor, &scope, session.id(), current.revision(), selected)
            .await,
        Err(Error::Forbidden)
    ));
    assert!(
        store
            .restrict_collection(
                &actor,
                &scope,
                session.id(),
                Revision::new(1).unwrap(),
                Collection::none()
            )
            .await
            .is_err()
    );
    clock.set(1002);
    let current_request =
        ObservationRequest::new(&actor, &scope, session.id(), current.revision(), 2).unwrap();
    assert!(matches!(
        store.record_browser(current_request, B::CopyAttempt).await,
        Err(Error::Forbidden)
    ));
    let paused = store
        .pause_exam(&actor, &scope, session.id(), current.revision())
        .await
        .unwrap();
    let broad_ack =
        Acknowledgement::for_collection("policy-v1", "notice-v1", selected, true).unwrap();
    assert!(matches!(
        store
            .resume_exam(&actor, &scope, session.id(), paused.revision(), &broad_ack)
            .await,
        Err(Error::Conflict)
    ));
    let resumed = store
        .resume_exam(
            &actor,
            &scope,
            session.id(),
            paused.revision(),
            &acknowledgement(),
        )
        .await
        .unwrap();
    let none = store
        .restrict_collection(
            &actor,
            &scope,
            session.id(),
            resumed.revision(),
            Collection::none(),
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .record_browser(
                ObservationRequest::new(&actor, &scope, none.id(), none.revision(), 2).unwrap(),
                B::PageVisible
            )
            .await,
        Err(Error::Forbidden)
    ));
    store.close().await;
    let store = SqliteSupervision::open(temp.path().join("supervision.sqlite"), config(), clock)
        .await
        .unwrap();
    let restored = store.session(&actor, &scope, session.id()).await.unwrap();
    assert_eq!(restored.collection(), Collection::none());
    assert_eq!(restored.initial_collection(), selected);
    assert_eq!(
        store
            .observations(&actor, &scope, session.id(), 0, 100)
            .await
            .unwrap(),
        vec![receipt]
    );
}

#[tokio::test]
async fn unpublished_schema_v1_is_refused_without_migrating_or_erasing_state() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v1.sqlite");
    let raw = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true),
    )
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("schema-v1.sql"))
        .execute(&raw)
        .await
        .unwrap();
    sqlx::query("INSERT INTO rullst_supervision_meta VALUES (1,1,'old-configuration',1000,37)")
        .execute(&raw)
        .await
        .unwrap();
    let before: Vec<(String, String)> =
        sqlx::query_as("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
            .fetch_all(&raw)
            .await
            .unwrap();
    raw.close().await;
    assert!(matches!(
        SqliteSupervision::open(&path, config(), TestClock::new()).await,
        Err(Error::Configuration)
    ));
    let raw =
        sqlx::SqlitePool::connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&path))
            .await
            .unwrap();
    let after: Vec<(String, String)> =
        sqlx::query_as("SELECT name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY name")
            .fetch_all(&raw)
            .await
            .unwrap();
    assert_eq!(before, after);
    let metadata: (i64, String, i64, i64) =
        sqlx::query_as("SELECT version,config,last_now,revision FROM rullst_supervision_meta")
            .fetch_one(&raw)
            .await
            .unwrap();
    assert_eq!(metadata, (1, "old-configuration".to_owned(), 1000, 37));
}

#[tokio::test]
async fn corrupted_observation_category_or_source_is_never_returned_as_a_valid_receipt() {
    let (temp, store, _) = fixture().await;
    let session = start(&store, Collection::new([C::CameraStatus]).unwrap()).await;
    let actor = context("learner-a");
    let scope = scope();
    let request =
        ObservationRequest::new(&actor, &scope, session.id(), session.revision(), 1).unwrap();
    store
        .record_capture(request, CaptureDevice::Camera, CaptureEvent::Started)
        .await
        .unwrap();
    let raw = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new().filename(temp.path().join("supervision.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE rullst_supervision_events SET kind=1")
        .execute(&raw)
        .await
        .unwrap();
    assert!(matches!(
        store
            .observations(&actor, &scope, session.id(), 0, 10)
            .await,
        Err(Error::Configuration)
    ));
    assert!(matches!(
        store.events(&actor, &scope, session.id(), 0, 10).await,
        Err(Error::Configuration)
    ));
    sqlx::query("UPDATE rullst_supervision_events SET kind=10,source=2,adapter_id='forged-adapter',adapter_version='v1'").execute(&raw).await.unwrap();
    assert!(matches!(
        store
            .observations(&actor, &scope, session.id(), 0, 10)
            .await,
        Err(Error::Configuration)
    ));
}

#[tokio::test]
async fn current_revision_cannot_authorize_paused_reports_or_change_an_ended_collection() {
    let (_temp, store, _) = fixture().await;
    let selected = Collection::new([C::Visibility, C::ClipboardActivity]).unwrap();
    let session = start(&store, selected).await;
    let actor = context("learner-a");
    let scope = scope();
    let paused = store
        .pause_exam(&actor, &scope, session.id(), session.revision())
        .await
        .unwrap();
    let report =
        ObservationRequest::new(&actor, &scope, paused.id(), paused.revision(), 1).unwrap();
    assert!(matches!(
        store.record_browser(report, B::PageHidden).await,
        Err(Error::Conflict)
    ));
    let narrowed = store
        .restrict_collection(
            &actor,
            &scope,
            paused.id(),
            paused.revision(),
            Collection::visibility_only(),
        )
        .await
        .unwrap();
    assert_eq!(narrowed.state(), SessionState::Paused);
    let ended = store
        .end_exam(&actor, &scope, narrowed.id(), narrowed.revision())
        .await
        .unwrap();
    let report = ObservationRequest::new(&actor, &scope, ended.id(), ended.revision(), 1).unwrap();
    assert!(matches!(
        store.record_browser(report, B::PageHidden).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store
            .restrict_collection(
                &actor,
                &scope,
                ended.id(),
                ended.revision(),
                Collection::none()
            )
            .await,
        Err(Error::Conflict)
    ));
    let unchanged = store.session(&actor, &scope, ended.id()).await.unwrap();
    assert_eq!(unchanged.state(), SessionState::Ended);
    assert_eq!(unchanged.collection(), Collection::visibility_only());
    assert_eq!(unchanged.revision(), ended.revision());
    assert_eq!(unchanged.last_sequence(), 0);
}
