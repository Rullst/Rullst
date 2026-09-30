#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use support::*;

async fn service(
    fixture: &Fixture,
    dir: &tempfile::TempDir,
    clock: TestClock,
) -> MediaService<BunnyStream, TestClock> {
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}

#[tokio::test]
async fn stopped_intent_controls_require_management_and_a_recorded_failure() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir, TestClock::new()).await;
    let auth = Auth::new();
    let (teacher, learner, id, scope) = (
        reference("teacher"),
        reference("learner"),
        reference("lesson"),
        scope(),
    );
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    assert_eq!(created.failure, None);
    let calls = fixture.remote.lock().unwrap().calls.len();
    for actor in [&learner, &teacher] {
        let expected = if actor == &learner {
            MediaError::Denied
        } else {
            MediaError::Conflict
        };
        assert_eq!(
            app.retry_failed(&auth, actor, &scope, &id, created.revision)
                .await
                .unwrap_err(),
            expected
        );
        assert_eq!(
            app.discard_failed(&auth, actor, &scope, &id, created.revision)
                .await
                .unwrap_err(),
            expected
        );
    }
    assert_eq!(fixture.remote.lock().unwrap().calls.len(), calls);
    assert_eq!(
        app.get(&auth, &teacher, &scope, &id).await.unwrap(),
        created
    );
    app.close().await;
}

fn calls(fixture: &Fixture, name: &str) -> usize {
    let remote = fixture.remote.lock().unwrap();
    remote.calls.iter().filter(|call| *call == name).count()
}

#[tokio::test]
async fn refused_creation_stops_until_explicitly_retried_or_discarded() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    auth.until
        .store(NOW + 200_000, std::sync::atomic::Ordering::SeqCst);
    let (teacher, scope) = (reference("teacher"), scope());
    let (first, second) = (reference("first"), reference("second"));
    fixture.remote.lock().unwrap().reject_create = true;
    assert_eq!(
        app.create(&auth, &teacher, &scope, &first, metadata())
            .await
            .unwrap_err(),
        MediaError::Rejected
    );
    let stopped = app.get(&auth, &teacher, &scope, &first).await.unwrap();
    assert_eq!(stopped.failure, Some(OperationFailure::Rejected));
    assert!(stopped.pending);
    assert_eq!(stopped.lifecycle, Lifecycle::Creating);
    // Neither the idempotent create nor reconcile retries it silently, and
    // no lease has to expire before the host can decide.
    assert_eq!(
        app.create(&auth, &teacher, &scope, &first, metadata())
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    assert_eq!(
        app.reconcile(&auth, &teacher, &scope, &first)
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    assert_eq!((calls(&fixture, "create"), calls(&fixture, "list")), (1, 0));
    fixture.remote.lock().unwrap().reject_create = false;
    let retried = app
        .retry_failed(&auth, &teacher, &scope, &first, stopped.revision)
        .await
        .unwrap();
    assert_eq!(retried.lifecycle, Lifecycle::Active);
    assert!(!retried.pending && retried.failure.is_none() && retried.video.is_some());
    assert_eq!(calls(&fixture, "create"), 2);

    fixture.remote.lock().unwrap().reject_create = true;
    assert!(
        app.create(&auth, &teacher, &scope, &second, metadata())
            .await
            .is_err()
    );
    let stopped = app.get(&auth, &teacher, &scope, &second).await.unwrap();
    let discarded = app
        .discard_failed(&auth, &teacher, &scope, &second, stopped.revision)
        .await
        .unwrap();
    assert_eq!(discarded.lifecycle, Lifecycle::Deleted);
    assert_eq!(discarded.processing, Processing::Missing);
    assert!(!discarded.pending && discarded.failure.is_none() && discarded.video.is_none());
    assert_eq!(calls(&fixture, "create"), 3);
    // The tombstone keeps the creation ID retired until retention purges it.
    clock.advance(86_400);
    assert_eq!(
        app.purge_deleted(&auth, &teacher, &scope, NOW, 10)
            .await
            .unwrap(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn update_of_a_remotely_deleted_video_stops_and_deletion_stays_possible() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir, TestClock::new()).await;
    let auth = Auth::new();
    let (teacher, id, scope) = (reference("teacher"), reference("lesson"), scope());
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    fixture.remote.lock().unwrap().videos.clear();
    let revised = Metadata::new("Revised lesson", "Revised transcript").unwrap();
    assert_eq!(
        app.update(&auth, &teacher, &scope, &id, created.revision, revised)
            .await
            .unwrap_err(),
        MediaError::NotFound
    );
    let stopped = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(stopped.failure, Some(OperationFailure::RemoteMissing));
    // Deletion asks for a deliberate decision instead of an endless Busy.
    assert_eq!(
        app.delete(&auth, &teacher, &scope, &id, stopped.revision)
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    let discarded = app
        .discard_failed(&auth, &teacher, &scope, &id, stopped.revision)
        .await
        .unwrap();
    assert!(!discarded.pending && discarded.failure.is_none());
    assert_eq!(discarded.lifecycle, Lifecycle::Active);
    assert_eq!(discarded.processing, Processing::Missing);
    assert_eq!(discarded.metadata.title(), "Revised lesson");
    let deleted = app
        .delete(&auth, &teacher, &scope, &id, discarded.revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    app.close().await;
}

#[tokio::test]
async fn unverifiable_or_full_metadata_updates_stop_without_automatic_resend() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let (teacher, id, scope) = (reference("teacher"), reference("lesson"), scope());
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.clone().unwrap();
    let revised = Metadata::new("Revised lesson", "Revised transcript").unwrap();
    fixture.remote.lock().unwrap().ignore_updates = true;
    assert_eq!(
        app.update(&auth, &teacher, &scope, &id, created.revision, revised)
            .await
            .unwrap_err(),
        MediaError::Uncertain
    );
    let stopped = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(
        stopped.failure,
        Some(OperationFailure::VerificationMismatch)
    );
    let updates = calls(&fixture, "update");
    clock.advance(46);
    assert_eq!(
        app.reconcile(&auth, &teacher, &scope, &id)
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    assert_eq!(calls(&fixture, "update"), updates);
    fixture.remote.lock().unwrap().ignore_updates = false;
    let retried = app
        .retry_failed(&auth, &teacher, &scope, &id, stopped.revision)
        .await
        .unwrap();
    assert!(!retried.pending && retried.failure.is_none());
    assert_eq!(retried.metadata.description(), "Revised transcript");

    let full: Vec<_> = (0..50)
        .map(|i| serde_json::json!({"property":format!("tag-{i}"),"value":"keep"}))
        .collect();
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.as_str())
        .unwrap()["metaTags"] = serde_json::json!(full);
    let final_metadata = Metadata::new("Final lesson", "Final transcript").unwrap();
    assert_eq!(
        app.update(
            &auth,
            &teacher,
            &scope,
            &id,
            retried.revision,
            final_metadata
        )
        .await
        .unwrap_err(),
        MediaError::Capacity
    );
    let stopped = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(stopped.failure, Some(OperationFailure::TagCapacity));
    let discarded = app
        .discard_failed(&auth, &teacher, &scope, &id, stopped.revision)
        .await
        .unwrap();
    assert!(!discarded.pending && discarded.failure.is_none());
    assert_eq!(discarded.metadata.title(), "Final lesson");
    assert_eq!(discarded.processing, created.processing);
    app.close().await;
}
