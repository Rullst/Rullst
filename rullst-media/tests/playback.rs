#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use std::{sync::Arc, time::Duration};
use support::*;

type Gate = Arc<(tokio::sync::Notify, tokio::sync::Notify)>;

async fn published(
    fixture: &Fixture,
    dir: &tempfile::TempDir,
) -> (Arc<MediaService<BunnyStream, TestClock>>, Arc<Auth>, Asset) {
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        TestClock::new(),
    )
    .await
    .unwrap();
    let app = Arc::new(MediaService::new(provider, store).unwrap());
    let auth = Arc::new(Auth::new());
    let teacher = reference("teacher");
    let created = app
        .create(
            auth.as_ref(),
            &teacher,
            &scope(),
            &reference("lesson"),
            metadata(),
        )
        .await
        .unwrap();
    fixture.ready(created.video.as_ref().unwrap());
    let asset = app
        .publish(
            auth.as_ref(),
            &teacher,
            &scope(),
            &reference("lesson"),
            created.revision,
        )
        .await
        .unwrap();
    (app, auth, asset)
}

/// Starts one learner playback and waits until its provider read is in flight.
async fn gated(
    fixture: &Fixture,
    app: &Arc<MediaService<BunnyStream, TestClock>>,
    auth: &Arc<Auth>,
) -> (
    Gate,
    tokio::task::JoinHandle<Result<PlaybackGrant, MediaError>>,
) {
    let gate: Gate = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    fixture.remote.lock().unwrap().gate = Some(gate.clone());
    let running = {
        let (app, auth) = (app.clone(), auth.clone());
        tokio::spawn(async move {
            app.playback(
                auth.as_ref(),
                &reference("learner"),
                &scope(),
                &reference("lesson"),
                60,
                PlaybackKind::Embed,
            )
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(5), gate.0.notified())
        .await
        .unwrap();
    fixture.remote.lock().unwrap().gate = None;
    (gate, running)
}

#[tokio::test]
async fn concurrent_or_abandoned_viewers_never_lock_out_other_learners() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let (app, auth, asset) = published(&fixture, &dir).await;
    let (learner, teacher, id, scope) = (
        reference("learner"),
        reference("teacher"),
        reference("lesson"),
        scope(),
    );
    let play = || {
        app.playback(
            auth.as_ref(),
            &learner,
            &scope,
            &id,
            60,
            PlaybackKind::Embed,
        )
    };

    // One viewer's provider read is in flight: it holds no durable intent,
    // and a second viewer is served meanwhile instead of Busy/Conflict.
    let (gate, first) = gated(&fixture, &app, &auth).await;
    let during = app.get(auth.as_ref(), &teacher, &scope, &id).await.unwrap();
    assert!(!during.pending);
    play().await.unwrap();
    gate.1.notify_one();
    first.await.unwrap().unwrap();

    // A dropped request (client disconnect or tower timeout) leaves nothing
    // behind, so the next viewer needs no lease expiry.
    let (_gate, abandoned) = gated(&fixture, &app, &auth).await;
    abandoned.abort();
    assert!(abandoned.await.unwrap_err().is_cancelled());
    play().await.unwrap();

    // So does a failed provider read.
    fixture.remote.lock().unwrap().fail_reads = true;
    assert_eq!(play().await.unwrap_err(), MediaError::Unavailable);
    fixture.remote.lock().unwrap().fail_reads = false;
    play().await.unwrap();

    // Reads of an unchanged ready asset never mutate its revision.
    let after = app.get(auth.as_ref(), &teacher, &scope, &id).await.unwrap();
    assert_eq!(after, asset);
    app.close().await;
}
