#![cfg(all(feature = "bunny", feature = "sqlite"))]
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::sync::atomic::Ordering;
use support::*;

async fn service(
    fixture: &Fixture,
    dir: &tempfile::TempDir,
    clock: TestClock,
    capacity: u32,
) -> MediaService<BunnyStream, TestClock> {
    let provider = fixture.provider();
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), capacity).unwrap(),
        clock,
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}
async fn connection(dir: &tempfile::TempDir) -> SqliteConnection {
    SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(dir.path().join("video.sqlite")),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn retention_purges_only_old_confirmed_deletions_and_restores_capacity() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone(), 2).await;
    let auth = Auth::new();
    auth.until.store(NOW + 200_000, Ordering::SeqCst);
    let teacher = reference("teacher");
    let scope = scope();
    let first = reference("retired-lesson");
    let second = reference("pending-deletion");
    let a = app
        .create(&auth, &teacher, &scope, &first, metadata())
        .await
        .unwrap();
    let b = app
        .create(&auth, &teacher, &scope, &second, metadata())
        .await
        .unwrap();
    assert_eq!(
        app.create(&auth, &teacher, &scope, &reference("new"), metadata())
            .await
            .unwrap_err(),
        MediaError::Capacity
    );
    app.delete(&auth, &teacher, &scope, &first, a.revision)
        .await
        .unwrap();
    fixture.remote.lock().unwrap().fail_deletes = true;
    assert!(
        app.delete(&auth, &teacher, &scope, &second, b.revision)
            .await
            .is_err()
    );
    assert_eq!(
        app.purge_deleted(&auth, &teacher, &scope, NOW, 10)
            .await
            .unwrap_err(),
        MediaError::InvalidInput
    );
    clock.advance(86_400);
    assert_eq!(
        app.purge_deleted(&auth, &reference("learner"), &scope, NOW, 10)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    assert_eq!(
        app.purge_deleted(&auth, &teacher, &scope, NOW, 0)
            .await
            .unwrap_err(),
        MediaError::InvalidInput
    );
    assert_eq!(
        app.purge_deleted(&auth, &teacher, &scope, NOW, 10)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        app.get(&auth, &teacher, &scope, &first).await.unwrap_err(),
        MediaError::NotFound
    );
    assert_eq!(
        app.get(&auth, &teacher, &scope, &second)
            .await
            .unwrap()
            .lifecycle,
        Lifecycle::Deleting
    );
    assert_eq!(
        app.purge_deleted(&auth, &teacher, &scope, NOW, 10)
            .await
            .unwrap(),
        0
    );
    app.create(
        &auth,
        &teacher,
        &scope,
        &reference("new-creation-id"),
        metadata(),
    )
    .await
    .unwrap();
    app.close().await;
}

#[tokio::test]
async fn malformed_records_and_schema_are_rejected_without_repair() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone(), 2).await;
    let auth = Auth::new();
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    app.create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    let mut db = connection(&dir).await;
    // A valid JSON blob cannot bypass agreement with indexed ownership columns.
    sqlx::query(
        "UPDATE media_assets SET body=json_set(body,'$.asset.scope.tenant','another-school')",
    )
    .execute(&mut db)
    .await
    .unwrap();
    assert_eq!(
        app.get(&auth, &teacher, &scope, &id).await.unwrap_err(),
        MediaError::Configuration
    );
    app.close().await;
    sqlx::query("CREATE TRIGGER unrelated AFTER UPDATE ON media_meta BEGIN SELECT 1; END")
        .execute(&mut db)
        .await
        .unwrap();
    let configuration = StoreConfig::testing(fixture.provider().binding(), 2).unwrap();
    assert!(matches!(
        SqliteMedia::open(dir.path().join("video.sqlite"), configuration, clock).await,
        Err(MediaError::Configuration)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sqlite_schema WHERE type='trigger'")
            .fetch_one(&mut db)
            .await
            .unwrap(),
        1
    );
    db.close().await.unwrap();
}

#[tokio::test]
async fn sqlite_lock_does_not_extend_permission_or_send_a_remote_mutation() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone(), 2).await;
    let auth = Auth::new();
    auth.until.store(NOW + 1, Ordering::SeqCst);
    let teacher = reference("teacher");
    let id = reference("lesson");
    let scope = scope();
    let mut db = connection(&dir).await;
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut db)
        .await
        .unwrap();
    let operation = app.create(&auth, &teacher, &scope, &id, metadata());
    tokio::pin!(operation);
    // Drive until the operation yields with the write lock held, then expire it.
    std::future::poll_fn(|cx| {
        use std::future::Future;
        assert!(operation.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    clock.advance(2);
    sqlx::query("COMMIT").execute(&mut db).await.unwrap();
    assert!(matches!(
        operation.await,
        Err(MediaError::Expired | MediaError::Denied)
    ));
    assert!(fixture.remote.lock().unwrap().calls.is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM media_assets")
            .fetch_one(&mut db)
            .await
            .unwrap(),
        0
    );
    db.close().await.unwrap();
    app.close().await;
}

#[cfg(unix)]
#[tokio::test]
async fn reopening_a_symlink_is_rejected_and_initialization_never_overwrites() {
    let fixture = Fixture::new().await;
    let dir = tempfile::tempdir().unwrap();
    let clock = TestClock::new();
    let app = service(&fixture, &dir, clock.clone(), 2).await;
    let config = StoreConfig::testing(fixture.provider().binding(), 2).unwrap();
    assert!(matches!(
        SqliteMedia::initialize(
            dir.path().join("video.sqlite"),
            config.clone(),
            clock.clone()
        )
        .await,
        Err(MediaError::Configuration)
    ));
    app.close().await;
    std::os::unix::fs::symlink(
        dir.path().join("video.sqlite"),
        dir.path().join("alias.sqlite"),
    )
    .unwrap();
    assert!(matches!(
        SqliteMedia::open(dir.path().join("alias.sqlite"), config, clock).await,
        Err(MediaError::Configuration)
    ));
}
