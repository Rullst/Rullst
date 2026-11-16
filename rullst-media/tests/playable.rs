//! Bunny's just-in-time encoding: a video whose playlists exist (API status 8)
//! is `Playable` in Bunny's player before encoding finishes; `Ready` still
//! means finished.
#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use serde_json::json;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use support::*;

async fn service(
    fixture: &Fixture,
    dir: &tempfile::TempDir,
) -> MediaService<BunnyStream, TestClock> {
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        TestClock::new(),
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}

fn set_status(fixture: &Fixture, video: &VideoId, status: u8, mp4: bool) {
    let mut remote = fixture.remote.lock().unwrap();
    let stored = remote.videos.get_mut(video.as_str()).unwrap();
    stored["status"] = json!(status);
    stored["hasMP4Fallback"] = json!(mp4);
}

#[test]
fn playable_allows_only_the_providers_player() {
    for kind in [
        PlaybackKind::Embed,
        PlaybackKind::Hls,
        PlaybackKind::Mp4_720p,
        PlaybackKind::Original,
    ] {
        assert!(Processing::Ready.allows(kind), "{kind:?}");
        assert_eq!(
            Processing::Playable.allows(kind),
            kind == PlaybackKind::Embed,
            "{kind:?}"
        );
        for state in [
            Processing::AwaitingUpload,
            Processing::Processing,
            Processing::Failed,
            Processing::Missing,
        ] {
            assert!(!state.allows(kind), "{state:?} {kind:?}");
            assert!(!state.is_playable(), "{state:?}");
        }
    }
    assert!(Processing::Ready.is_playable());
    assert!(Processing::Playable.is_playable());
    assert_eq!(
        serde_json::to_string(&Processing::Playable).unwrap(),
        "\"Playable\""
    );
}

#[tokio::test]
async fn a_playable_video_is_published_and_embedded_until_it_finishes() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir).await;
    let auth = Auth::new();
    let (teacher, learner, id, scope) = (
        reference("teacher"),
        reference("learner"),
        reference("lesson"),
        scope(),
    );
    let play = |kind| app.playback(&auth, &learner, &scope, &id, 60, kind);
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.clone().unwrap();

    // Segmenting (7) is still processing: nothing to publish yet.
    set_status(&fixture, &video, 7, false);
    assert_eq!(
        app.publish(&auth, &teacher, &scope, &id, created.revision)
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    let segmenting = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(segmenting.processing, Processing::Processing);
    assert!(!segmenting.published);

    // Playlists created (8): publishable, and embeddable in Bunny's player.
    set_status(&fixture, &video, 8, true);
    let published = app
        .publish(&auth, &teacher, &scope, &id, segmenting.revision)
        .await
        .unwrap();
    assert_eq!(published.processing, Processing::Playable);
    assert!(published.published);
    assert!(play(PlaybackKind::Embed).await.is_ok());
    for kind in [PlaybackKind::Hls, PlaybackKind::Mp4_720p] {
        assert_eq!(play(kind).await.unwrap_err(), MediaError::Unsupported);
    }
    assert!(
        app.get(&auth, &teacher, &scope, &id)
            .await
            .unwrap()
            .published
    );

    // Finished (4): every kind, and the MP4 fallback the API reports.
    set_status(&fixture, &video, 4, true);
    for kind in [
        PlaybackKind::Embed,
        PlaybackKind::Hls,
        PlaybackKind::Mp4_720p,
    ] {
        assert!(play(kind).await.is_ok(), "{kind:?}");
    }
    let ready = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(ready.processing, Processing::Ready);
    assert!(ready.published);
}

#[tokio::test]
async fn a_playable_video_that_stops_being_watchable_is_withdrawn() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir).await;
    let auth = Auth::new();
    let (teacher, id, scope) = (reference("teacher"), reference("lesson"), scope());
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let video = created.video.clone().unwrap();
    set_status(&fixture, &video, 8, false);
    let published = app
        .publish(&auth, &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    assert!(published.published);

    set_status(&fixture, &video, 3, false);
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
        MediaError::Denied
    );
    let withdrawn = app.get(&auth, &teacher, &scope, &id).await.unwrap();
    assert_eq!(withdrawn.processing, Processing::Processing);
    assert!(!withdrawn.published);

    // A refresh that reads a playable state again keeps it unpublished
    // until the instructor publishes it.
    set_status(&fixture, &video, 8, false);
    let refreshed = app
        .refresh(&auth, &teacher, &scope, &id, withdrawn.revision)
        .await
        .unwrap();
    assert_eq!(refreshed.processing, Processing::Playable);
    assert!(!refreshed.published);
}

/// Sets `$.asset.processing` of the only stored record to `processing`,
/// starting from `body`.
async fn store_processing(db: &mut SqliteConnection, body: &str, processing: &str) {
    sqlx::query("UPDATE media_assets SET body=json_set(?, '$.asset.processing', ?)")
        .bind(body)
        .bind(processing)
        .execute(db)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_stored_record_is_published_only_while_playable_and_a_tombstone_stays_missing() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let app = service(&fixture, &dir).await;
    let auth = Auth::new();
    let (teacher, id, scope) = (reference("teacher"), reference("lesson"), scope());
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    set_status(&fixture, created.video.as_ref().unwrap(), 4, false);
    let published = app
        .publish(&auth, &teacher, &scope, &id, created.revision)
        .await
        .unwrap();
    let mut db = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(dir.path().join("video.sqlite")),
    )
    .await
    .unwrap();
    let body: String = sqlx::query_scalar("SELECT body FROM media_assets")
        .fetch_one(&mut db)
        .await
        .unwrap();
    // A record read back must keep publication on a playable state.
    for (processing, valid) in [
        ("Playable", true),
        ("Processing", false),
        ("AwaitingUpload", false),
        ("Failed", false),
        ("Missing", false),
        ("Ready", true),
    ] {
        store_processing(&mut db, &body, processing).await;
        let read = app.get(&auth, &teacher, &scope, &id).await;
        match valid {
            true => assert!(read.unwrap().published, "{processing}"),
            false => assert_eq!(read.unwrap_err(), MediaError::Configuration, "{processing}"),
        }
    }
    let withdrawn = app
        .withdraw(&auth, &teacher, &scope, &id, published.revision)
        .await
        .unwrap();
    let deleted = app
        .delete(&auth, &teacher, &scope, &id, withdrawn.revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    let tombstone: String = sqlx::query_scalar("SELECT body FROM media_assets")
        .fetch_one(&mut db)
        .await
        .unwrap();
    // A tombstone is never playable, ready or processing again.
    for processing in ["Ready", "Playable", "Processing"] {
        store_processing(&mut db, &tombstone, processing).await;
        assert_eq!(
            app.get(&auth, &teacher, &scope, &id).await.unwrap_err(),
            MediaError::Configuration,
            "{processing}"
        );
    }
    store_processing(&mut db, &tombstone, "Missing").await;
    assert_eq!(
        app.get(&auth, &teacher, &scope, &id)
            .await
            .unwrap()
            .lifecycle,
        Lifecycle::Deleted
    );
    db.close().await.unwrap();
}
