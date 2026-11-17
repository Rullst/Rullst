#![cfg(all(feature = "bunny", feature = "sqlite"))]
//! Notification binding and rate, and tombstone purge windows, against the
//! offline Bunny adapter.
mod support;
use rullst_media::{bunny::*, sqlite::*, *};
use std::sync::atomic::Ordering;
use support::*;

struct Course {
    app: MediaService<BunnyStream, TestClock>,
    auth: Auth,
    teacher: Reference,
    scope: Scope,
    id: Reference,
    created: Asset,
    _dir: tempfile::TempDir,
}

async fn course(clock: TestClock) -> Course {
    let dir = tempfile::tempdir().unwrap();
    let provider = BunnyStream::new(config("offline")).unwrap();
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    let app = MediaService::new(provider, store).unwrap();
    let (auth, teacher, scope, id) = (
        Auth::new(),
        reference("teacher"),
        scope(),
        reference("lesson"),
    );
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata())
        .await
        .unwrap();
    Course {
        app,
        auth,
        teacher,
        scope,
        id,
        created,
        _dir: dir,
    }
}

/// A notification about `video` signed for `provider` with `webhook`; the
/// spaces make otherwise equal bodies distinct.
fn signed(
    provider: &BunnyStream,
    webhook: &str,
    library: i64,
    video: &VideoId,
    spaces: usize,
) -> VerifiedNotification {
    let body = format!(
        r#"{{{}"VideoLibraryId":{library},"VideoGuid":"{}","Status":3}}"#,
        " ".repeat(spaces),
        video.as_str()
    );
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, webhook.as_bytes());
    let signature: String = ring::hmac::sign(&key, body.as_bytes())
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let headers = WebhookHeaders::new("v1", "hmac-sha256", &signature);
    provider
        .verify_notification(headers, body.as_bytes())
        .unwrap()
}

#[tokio::test]
async fn notifications_verified_for_another_binding_are_denied() {
    let course = course(TestClock::new()).await;
    let video = course.created.video.clone().unwrap();
    let other_library = BunnyStream::new(
        BunnyConfig::new(
            LibraryId::new(8).unwrap(),
            reference("acceptance"),
            "test-library.b-cdn.net",
            BunnyCredentials::new("mock_api", "mock_webhook", "mock_embed", "mock_cdn").unwrap(),
            PrivateDelivery::configured(true, true, true).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let event = signed(&other_library, "mock_webhook", 8, &video, 0);
    assert_eq!(event.mode(), ProviderMode::Offline);
    assert_eq!(
        course.app.notification(&event).await,
        Err(MediaError::Denied)
    );
    // Same library, protocol-fixture mode.
    let fixture = BunnyStream::for_protocol_tests(config("fixture"), "http://127.0.0.1:9").unwrap();
    let event = notification(&fixture, &video, 3);
    assert_eq!(event.library(), LibraryId::new(7).unwrap());
    assert_eq!(
        course.app.notification(&event).await,
        Err(MediaError::Denied)
    );
    let own = signed(course.app.provider(), "mock_webhook", 7, &video, 0);
    assert_eq!(course.app.notification(&own).await, Ok(true));
}

#[tokio::test]
async fn a_new_notification_waits_two_seconds_after_the_last_one() {
    let clock = TestClock::new();
    let course = course(clock.clone()).await;
    let video = course.created.video.clone().unwrap();
    let event = |spaces| signed(course.app.provider(), "mock_webhook", 7, &video, spaces);
    assert_eq!(course.app.notification(&event(0)).await, Ok(true));
    for step in [0, 1] {
        clock.advance(step);
        assert_eq!(
            course.app.notification(&event(1)).await,
            Err(MediaError::Busy),
            "{step}"
        );
    }
    clock.advance(1);
    assert_eq!(course.app.notification(&event(1)).await, Ok(true));
}

#[tokio::test]
async fn purge_takes_tombstones_older_than_the_cutoff_in_pages_of_at_most_100() {
    let clock = TestClock::new();
    let course = course(clock.clone()).await;
    let (app, auth, teacher, scope) = (&course.app, &course.auth, &course.teacher, &course.scope);
    let deleted = app
        .delete(auth, teacher, scope, &course.id, course.created.revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    assert_eq!(deleted.updated_at, NOW);
    auth.until.store(NOW + 200_000, Ordering::SeqCst);
    clock.set(NOW + 86_410);
    let cutoff = NOW + 5;
    assert_eq!(
        app.purge_deleted(auth, teacher, scope, cutoff, 101).await,
        Err(MediaError::InvalidInput)
    );
    assert_eq!(
        app.purge_deleted(auth, teacher, scope, cutoff, 100).await,
        Ok(1)
    );
    let gone = app.get(auth, teacher, scope, &course.id).await;
    assert_eq!(gone.err(), Some(MediaError::NotFound));
}
