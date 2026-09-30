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
