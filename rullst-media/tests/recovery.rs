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
async fn learner_and_webhook_retry_only_expired_refresh_intents() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let learner = reference("learner");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.unwrap();
    fixture.remote.lock().unwrap().fail_reads = true;
    assert_eq!(
        app.upload(&auth, &teacher, &scope, &id, 60)
            .await
            .unwrap_err(),
        MediaError::Unavailable
    );
    fixture.remote.lock().unwrap().fail_reads = false;
    clock.advance(46);
    app.upload(&auth, &teacher, &scope, &id, 60).await.unwrap();
    fixture.ready(&video);
    let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    app.publish(&auth, &teacher, &scope, &id, current.revision)
        .await
        .unwrap();
    fixture.remote.lock().unwrap().fail_reads = true;
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Unavailable
    );
    fixture.remote.lock().unwrap().fail_reads = false;
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Busy
    );
    clock.advance(46);
    app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
        .await
        .unwrap();
    let event = notification(app.provider(), &video, 3);
    fixture.remote.lock().unwrap().fail_reads = true;
    assert_eq!(
        app.notification(&event).await.unwrap_err(),
        MediaError::Unavailable
    );
    fixture.remote.lock().unwrap().fail_reads = false;
    clock.advance(46);
    assert!(app.notification(&event).await.unwrap());
    assert!(!app.notification(&event).await.unwrap());
    // A learner or notification must never take over a pending destructive intent.
    fixture.remote.lock().unwrap().fail_deletes = true;
    let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(
        app.delete(&auth, &teacher, &scope, &id, current.revision)
            .await
            .unwrap_err(),
        MediaError::Unavailable
    );
    clock.advance(46);
    let calls = fixture.remote.lock().unwrap().calls.len();
    assert!(
        app.notification(&notification(app.provider(), &video, 4))
            .await
            .is_err()
    );
    assert!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .is_err()
    );
    assert_eq!(fixture.remote.lock().unwrap().calls.len(), calls);
    app.close().await;
}

#[tokio::test]
async fn absent_or_ambiguous_creation_markers_never_cause_another_post() {
    for ambiguous in [false, true] {
        let fixture = Fixture::new().await;
        let dir = tempfile::tempdir().unwrap();
        let clock = TestClock::new();
        let app = service(&fixture, &dir, clock.clone()).await;
        let auth = Auth::new();
        let teacher = reference("teacher");
        let id = reference("lesson");
        let scope = scope();
        fixture.remote.lock().unwrap().lost_create = true;
        assert!(
            app.create(&auth, &teacher, &scope, &id, metadata())
                .await
                .is_err()
        );
        {
            let mut remote = fixture.remote.lock().unwrap();
            if ambiguous {
                let mut other = remote.videos.values().next().unwrap().clone();
                other["guid"] = serde_json::json!(VIDEO);
                remote.videos.insert(VIDEO.into(), other);
            } else {
                remote.videos.clear();
            }
        }
        for _ in 0..2 {
            clock.advance(46);
            assert!(app.reconcile(&auth, &teacher, &scope, &id).await.is_err());
        }
        assert_eq!(
            fixture
                .remote
                .lock()
                .unwrap()
                .calls
                .iter()
                .filter(|s| *s == "create")
                .count(),
            1
        );
        assert_eq!(
            app.get(&auth, &teacher, &scope, &id)
                .await
                .unwrap()
                .lifecycle,
            Lifecycle::Creating
        );
        app.close().await;
    }
}

#[tokio::test]
async fn independently_opened_instances_create_only_one_remote_video() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let first = service(&fixture, &dir, clock.clone()).await;
    let provider = fixture.provider();
    let second = MediaService::new(
        provider,
        SqliteMedia::open(
            dir.path().join("video.sqlite"),
            StoreConfig::testing(first.provider().binding(), 32).unwrap(),
            clock.clone(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    let (a, b) = tokio::join!(
        first.create(&auth, &teacher, &scope, &id, metadata()),
        second.create(&auth, &teacher, &scope, &id, metadata())
    );
    assert!(a.is_ok() || b.is_ok());
    for result in [a, b] {
        assert!(result.is_ok() || result == Err(MediaError::Busy));
    }
    assert_eq!(
        fixture
            .remote
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|s| *s == "create")
            .count(),
        1
    );
    first.close().await;
    clock.advance(46);
    assert_eq!(
        second
            .reconcile(&auth, &teacher, &scope, &id)
            .await
            .unwrap()
            .lifecycle,
        Lifecycle::Active
    );
    second.close().await;
}

#[tokio::test]
async fn direct_mp4_requires_current_provider_fallback_and_resolution() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir, TestClock::new()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let learner = reference("learner");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.unwrap();
    fixture.ready(&video);
    app.publish(&auth, &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Mp4_720p)
            .await
            .unwrap_err(),
        MediaError::Unsupported
    );
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.as_str())
        .unwrap()["hasMP4Fallback"] = serde_json::json!(true);
    assert!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Mp4_720p)
            .await
            .unwrap()
            .expose_url()
            .contains("play_720p.mp4")
    );
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.as_str())
        .unwrap()["availableResolutions"] = serde_json::json!("360p,480p");
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Mp4_720p)
            .await
            .unwrap_err(),
        MediaError::Unsupported
    );
    app.close().await;
}

#[tokio::test]
async fn pending_or_failed_processing_never_publishes_and_recovery_needs_deliberate_publication() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir, TestClock::new()).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let learner = reference("learner");
    let id = reference("lesson");
    let scope = scope();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.unwrap();
    for status in [0, 1, 2, 3, 5, 6, 7, 8] {
        fixture
            .remote
            .lock()
            .unwrap()
            .videos
            .get_mut(video.as_str())
            .unwrap()["status"] = serde_json::json!(status);
        let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
        assert_eq!(
            app.publish(&auth, &teacher, &scope, &id, current.revision)
                .await
                .unwrap_err(),
            MediaError::Conflict
        );
        assert!(
            !app.get(&auth, &teacher, &scope, &id)
                .await
                .unwrap()
                .published
        );
        assert_eq!(
            app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
                .await
                .unwrap_err(),
            MediaError::Denied
        );
    }
    fixture.ready(&video);
    let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    let refreshed = app
        .refresh(&auth, &teacher, &scope, &id, current.revision)
        .await
        .unwrap();
    assert!(!refreshed.published);
    app.publish(&auth, &teacher, &scope, &id, refreshed.revision)
        .await
        .unwrap();
    // A later encoding failure withdraws local publication. Merely recovering
    // Ready through a missed-webhook refresh cannot silently republish it.
    fixture
        .remote
        .lock()
        .unwrap()
        .videos
        .get_mut(video.as_str())
        .unwrap()["status"] = serde_json::json!(5);
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    fixture.ready(&video);
    let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert!(
        !app.refresh(&auth, &teacher, &scope, &id, current.revision)
            .await
            .unwrap()
            .published
    );
    app.close().await;
}
