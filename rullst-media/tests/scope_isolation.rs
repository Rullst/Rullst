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

#[tokio::test]
async fn an_optional_tenant_quota_keeps_one_tenant_from_filling_the_store() {
    let fixture = Fixture::new().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("quota.sqlite");
    let clock = TestClock::new();
    let configuration = || StoreConfig::testing(fixture.provider().binding(), 4).unwrap();
    assert!(configuration().tenant_assets(0).is_err());
    assert!(configuration().tenant_assets(5).is_err());
    let store = SqliteMedia::initialize(
        &path,
        configuration().tenant_assets(2).unwrap(),
        clock.clone(),
    )
    .await
    .unwrap();
    let app = MediaService::new(fixture.provider(), store).unwrap();
    let teacher = reference("teacher");
    let flooded = [scope(), Scope::new("school-a", "other-course").unwrap()];
    for (index, scope) in flooded.iter().enumerate() {
        let mut auth = Auth::new();
        auth.scope = scope.clone();
        let id = reference(&format!("lesson-{index}"));
        app.create(&auth, &teacher, scope, &id, metadata())
            .await
            .unwrap();
    }
    let mut auth = Auth::new();
    auth.scope = flooded[1].clone();
    assert_eq!(
        app.create(&auth, &teacher, &flooded[1], &reference("more"), metadata())
            .await
            .unwrap_err(),
        MediaError::Capacity
    );
    // Another tenant keeps its share of the store-wide capacity.
    let other = Scope::new("school-b", "rust-course").unwrap();
    auth.scope = other.clone();
    app.create(&auth, &teacher, &other, &reference("lesson"), metadata())
        .await
        .unwrap();
    app.close().await;
    // The quota is part of the persisted configuration.
    assert!(matches!(
        SqliteMedia::open(&path, configuration(), clock.clone()).await,
        Err(MediaError::Configuration)
    ));
    SqliteMedia::open(&path, configuration().tenant_assets(2).unwrap(), clock)
        .await
        .unwrap()
        .close()
        .await;
}
