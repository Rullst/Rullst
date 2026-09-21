#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use std::sync::{Arc, atomic::Ordering};
use support::*;

async fn service(
    fixture: &Fixture,
    directory: &tempfile::TempDir,
    clock: TestClock,
) -> MediaService<BunnyStream, TestClock> {
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        directory.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}

#[tokio::test]
async fn instructor_upload_processing_publication_entitlement_and_deletion_are_complete() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let learner = reference("learner");
    let id = reference("lesson-1");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    assert_eq!(created.lifecycle, Lifecycle::Active);
    assert!(!created.pending);
    assert!(!created.published);
    assert_eq!(
        app.create(&auth, &teacher, &scope, &id, metadata())
            .await
            .unwrap(),
        created
    );
    let upload = app.upload(&auth, &teacher, &scope, &id, 300).await.unwrap();
    assert_eq!(upload.video, created.video.clone().unwrap());
    assert_eq!(
        app.upload(&auth, &learner, &scope, &id, 300)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    fixture.ready(&upload.video);
    let event = notification(app.provider(), &upload.video, 3);
    assert!(app.notification(&event).await.unwrap());
    assert!(!app.notification(&event).await.unwrap());
    let ready = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(ready.processing, Processing::Ready);
    assert!(!ready.published);
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    let published = app
        .publish(&auth, &teacher, &scope, &id, ready.revision)
        .await
        .unwrap();
    assert!(published.published);
    auth.until.store(NOW + 30, Ordering::SeqCst);
    let playback = app
        .playback(&auth, &learner, &scope, &id, 300, PlaybackKind::Hls)
        .await
        .unwrap();
    assert_eq!(playback.expires_at, NOW + 30);
    auth.revoked.store(true, Ordering::SeqCst);
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 300, PlaybackKind::Hls)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    auth.revoked.store(false, Ordering::SeqCst);
    auth.until.store(NOW + 10_000, Ordering::SeqCst);
    clock.advance(3);
    assert!(
        app.notification(&notification(app.provider(), &upload.video, 5))
            .await
            .unwrap()
    );
    let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(current.processing, Processing::Ready);
    assert!(current.published);
    let changed = app
        .update(
            &auth,
            &teacher,
            &scope,
            &id,
            current.revision,
            Metadata::new("Revised lesson", "New transcript").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(changed.metadata.title(), "Revised lesson");
    let deleted = app
        .delete(&auth, &teacher, &scope, &id, changed.revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    assert!(!deleted.published);
    assert!(!deleted.pending);
    assert_eq!(deleted.metadata.description(), "");
    assert!(
        !fixture
            .remote
            .lock()
            .unwrap()
            .videos
            .contains_key(upload.video.as_str())
    );
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    app.close().await;
}

#[tokio::test]
async fn uncertain_creation_is_recovered_after_restart_without_a_second_post() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    fixture.remote.lock().unwrap().lost_create = true;
    assert_eq!(
        app.create(&auth, &teacher, &scope, &id, metadata())
            .await
            .unwrap_err(),
        MediaError::Unavailable
    );
    assert_eq!(
        app.create(&auth, &teacher, &scope, &id, metadata())
            .await
            .unwrap_err(),
        MediaError::Busy
    );
    let pending = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert!(pending.pending);
    assert_eq!(pending.lifecycle, Lifecycle::Creating);
    app.close().await;
    clock.advance(46);
    let provider = fixture.provider();
    let store = SqliteMedia::open(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    let app = MediaService::new(provider, store).unwrap();
    let recovered = app.reconcile(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(recovered.lifecycle, Lifecycle::Active);
    assert!(!recovered.pending);
    assert_eq!(
        fixture
            .remote
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|v| *v == "create")
            .count(),
        1
    );
    assert!(
        app.create(
            &auth,
            &teacher,
            &scope,
            &id,
            Metadata::new("Different input", "").unwrap()
        )
        .await
        .is_err()
    );
    app.close().await;
}

#[tokio::test]
async fn course_scoping_revision_and_provider_failures_never_grant_access() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let other = Scope::new("school-b", "rust-course").unwrap();
    assert_eq!(
        app.get(&auth, &teacher, &other, &id).await.unwrap_err(),
        MediaError::Denied
    );
    assert_eq!(
        app.update(&auth, &teacher, &scope, &id, 0, metadata())
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    assert_eq!(
        app.list(&auth, &reference("learner"), &scope, None, 100)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    fixture.ready(created.video.as_ref().unwrap());
    app.publish(&auth, &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    fixture.remote.lock().unwrap().fail_reads = true;
    assert_eq!(
        app.playback(
            &auth,
            &reference("learner"),
            &scope,
            &id,
            60,
            PlaybackKind::Embed
        )
        .await
        .unwrap_err(),
        MediaError::Unavailable
    );
    assert!(app.get(&auth, &teacher, &scope, &id).await.unwrap().pending);
    fixture.remote.lock().unwrap().fail_reads = false;
    clock.advance(46);
    app.reconcile(&auth, &teacher, &scope, &id).await.unwrap();
    fixture.remote.lock().unwrap().videos.clear();
    assert_eq!(
        app.playback(&auth, &teacher, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    let missing = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(missing.processing, Processing::Missing);
    assert!(!missing.published);
    app.close().await;
}

#[tokio::test]
async fn withdrawing_during_a_provider_read_fences_its_late_result() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = Arc::new(service(&fixture, &dir, clock.clone()).await);
    let auth = Arc::new(Auth::new());
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(auth.as_ref(), &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    fixture.ready(created.video.as_ref().unwrap());
    app.publish(auth.as_ref(), &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    let gate = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    fixture.remote.lock().unwrap().gate = Some(gate.clone());
    let running = {
        let app = app.clone();
        let auth = auth.clone();
        let scope = scope.clone();
        let id = id.clone();
        tokio::spawn(async move {
            app.playback(
                auth.as_ref(),
                &reference("learner"),
                &scope,
                &id,
                60,
                PlaybackKind::Embed,
            )
            .await
        })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), gate.0.notified())
        .await
        .unwrap();
    let pending = app.get(auth.as_ref(), &teacher, &scope, &id).await.unwrap();
    assert!(pending.pending);
    app.withdraw(auth.as_ref(), &teacher, &scope, &id, pending.revision)
        .await
        .unwrap();
    fixture.remote.lock().unwrap().gate = None;
    gate.1.notify_one();
    assert_eq!(running.await.unwrap().unwrap_err(), MediaError::Conflict);
    clock.advance(46);
    let recovered = app
        .reconcile(auth.as_ref(), &teacher, &scope, &id)
        .await
        .unwrap();
    assert!(!recovered.published);
    app.close().await;
}

#[tokio::test]
async fn permission_expiry_during_publication_does_not_publish() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = Arc::new(service(&fixture, &dir, clock.clone()).await);
    let auth = Arc::new(Auth::new());
    let id = reference("lesson");
    let scope = scope();
    let teacher = reference("teacher");
    let created = app
        .create(auth.as_ref(), &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    fixture.ready(created.video.as_ref().unwrap());
    auth.until.store(NOW + 1, Ordering::SeqCst);
    let gate = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    fixture.remote.lock().unwrap().gate = Some(gate.clone());
    let running = {
        let app = app.clone();
        let auth = auth.clone();
        let scope = scope.clone();
        let id = id.clone();
        tokio::spawn(async move {
            app.publish(
                auth.as_ref(),
                &reference("teacher"),
                &scope,
                &id,
                created.revision,
            )
            .await
        })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), gate.0.notified())
        .await
        .unwrap();
    clock.advance(2);
    fixture.remote.lock().unwrap().gate = None;
    gate.1.notify_one();
    assert_eq!(running.await.unwrap().unwrap_err(), MediaError::Denied);
    auth.until.store(NOW + 10_000, Ordering::SeqCst);
    clock.advance(46);
    let recovered = app
        .reconcile(auth.as_ref(), &teacher, &scope, &id)
        .await
        .unwrap();
    assert!(!recovered.published);
    app.close().await;
}

#[tokio::test]
async fn deletion_failure_blocks_playback_and_reconciliation_erases_metadata() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    fixture.ready(created.video.as_ref().unwrap());
    let published = app
        .publish(&auth, &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    fixture.remote.lock().unwrap().fail_deletes = true;
    assert_eq!(
        app.delete(&auth, &teacher, &scope, &id, published.revision)
            .await
            .unwrap_err(),
        MediaError::Unavailable
    );
    assert_eq!(
        app.playback(&auth, &teacher, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    fixture.remote.lock().unwrap().fail_deletes = false;
    clock.advance(46);
    assert_eq!(
        app.reconcile(&auth, &teacher, &scope, &id)
            .await
            .unwrap()
            .lifecycle,
        Lifecycle::Deleted
    );
    app.close().await;
}

#[tokio::test]
async fn production_rejects_mock_state_and_reopen_checks_configuration_and_clock() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    assert!(StoreConfig::production(app.provider().binding(), 32).is_err());
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    app.create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    clock.advance(10);
    app.get(&auth, &teacher, &scope, &id).await.unwrap();
    clock.set(NOW);
    assert_eq!(
        app.get(&auth, &teacher, &scope, &id).await.unwrap_err(),
        MediaError::Clock
    );
    app.close().await;
    assert!(
        SqliteMedia::open(
            dir.path().join("video.sqlite"),
            StoreConfig::testing(fixture.provider().binding(), 31).unwrap(),
            clock.clone()
        )
        .await
        .is_err()
    );
    assert!(
        SqliteMedia::open(
            dir.path().join("video.sqlite"),
            StoreConfig::testing(fixture.provider().binding(), 32).unwrap(),
            clock
        )
        .await
        .is_err()
    );
}
