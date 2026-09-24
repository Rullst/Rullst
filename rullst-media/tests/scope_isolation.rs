#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{sqlite::*, *};
use support::*;

#[tokio::test]
async fn identical_asset_names_in_authorized_tenants_and_courses_never_share_lifecycle() {
    let fixture = Fixture::new().await;
    let directory = tempfile::tempdir().unwrap();
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        directory.path().join("isolation.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        TestClock::new(),
    )
    .await
    .unwrap();
    let app = MediaService::new(provider, store).unwrap();
    let scopes = [
        scope(),
        Scope::new("school-b", "rust-course").unwrap(),
        Scope::new("school-a", "other-course").unwrap(),
    ];
    let teacher = reference("teacher");
    let id = reference("same-lesson");
    let mut originals = Vec::new();
    for scope in &scopes {
        let mut auth = Auth::new();
        auth.scope = scope.clone();
        originals.push(
            app.create(&auth, &teacher, scope, &id, metadata())
                .await
                .unwrap(),
        );
    }
    for left in 0..3 {
        for right in left + 1..3 {
            assert_ne!(originals[left].video, originals[right].video);
        }
    }
    let video = originals[0].video.as_ref().unwrap();
    fixture.ready(video);
    assert!(
        app.notification(&notification(app.provider(), video, 3))
            .await
            .unwrap()
    );
    let auth = Auth::new();
    let ready = app.get(&auth, &teacher, &scopes[0], &id).await.unwrap();
    assert_eq!(ready.processing, Processing::Ready);
    let published = app
        .publish(&auth, &teacher, &scopes[0], &id, ready.revision)
        .await
        .unwrap();
    assert!(published.published);
    let deleted = app
        .delete(&auth, &teacher, &scopes[0], &id, published.revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    for (scope, original) in scopes.iter().zip(&originals).skip(1) {
        let mut auth = Auth::new();
        auth.scope = scope.clone();
        assert_eq!(
            app.get(&auth, &teacher, scope, &id).await.unwrap(),
            *original
        );
        assert!(
            fixture
                .remote
                .lock()
                .unwrap()
                .videos
                .contains_key(original.video.as_ref().unwrap().as_str())
        );
    }
    app.close().await;
}
