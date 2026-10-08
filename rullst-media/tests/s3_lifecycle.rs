#![cfg(all(feature = "s3", feature = "sqlite"))]
//! The S3 adapter behind the shared-local service, using the offline mock.
use rullst_media::{s3::*, sqlite::*, *};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI64, Ordering},
};

const NOW: i64 = 1_800_000_000;

#[derive(Clone)]
struct TestClock(Arc<AtomicI64>);
impl Clock for TestClock {
    fn now(&self) -> Result<i64, MediaError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}

struct Auth {
    until: AtomicI64,
    revoked: AtomicBool,
}
impl Authorization for Auth {
    async fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> Result<Permission, MediaError> {
        if self.revoked.load(Ordering::SeqCst)
            || scope != &course()
            || (actor.as_str() != "teacher"
                && (actor.as_str() != "learner" || action == Action::Manage))
        {
            return Err(MediaError::Denied);
        }
        Permission::until(self.until.load(Ordering::SeqCst))
    }
}
fn auth() -> Auth {
    Auth {
        until: AtomicI64::new(NOW + 10_000),
        revoked: AtomicBool::new(false),
    }
}
fn course() -> Scope {
    Scope::new("school-a", "rust-course").unwrap()
}
fn reference(value: &str) -> Reference {
    Reference::new(value).unwrap()
}
fn storage(mock: bool) -> S3Storage {
    let credentials = if mock {
        S3Credentials::new("mock_access", "mock_secret").unwrap()
    } else {
        S3Credentials::new(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        )
        .unwrap()
    };
    let config = S3Config::r2(
        LibraryId::new(3).unwrap(),
        "0123abcd",
        "videos",
        credentials,
    )
    .unwrap()
    .with_key_prefix("lessons/")
    .unwrap()
    .with_max_object_bytes(10_000_000)
    .unwrap();
    S3Storage::new(config).unwrap()
}
async fn service(directory: &tempfile::TempDir) -> MediaService<S3Storage, TestClock> {
    let provider = storage(true);
    let store = SqliteMedia::initialize(
        directory.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 16).unwrap(),
        TestClock(Arc::new(AtomicI64::new(NOW))),
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}

#[tokio::test]
async fn upload_confirmation_publication_playback_and_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let app = service(&dir).await;
    let auth = auth();
    let (teacher, learner, id, scope) = (
        reference("teacher"),
        reference("learner"),
        reference("lesson-1"),
        course(),
    );
    let metadata = Metadata::new("Ownership", "Transcript.").unwrap();
    let created = app
        .create(&auth, &teacher, &scope, &id, metadata)
        .await
        .unwrap();
    assert_eq!(created.lifecycle, Lifecycle::Active);
    assert_eq!(created.processing, Processing::AwaitingUpload);
    assert_eq!(created.metadata.title(), "Ownership");
    let video = created.video.clone().unwrap();
    let declared = UploadDeclaration::new("video/mp4", 4_096).unwrap();
    let grant = app
        .upload_declared(&auth, &teacher, &scope, &id, 300, &declared)
        .await
        .unwrap();
    assert_eq!(grant.video, video);
    assert_eq!(grant.protocol, UploadProtocol::PresignedPut);
    assert_eq!(grant.content_length, Some(4_096));
    // Object storage needs a declaration to bind.
    assert_eq!(
        app.upload(&auth, &teacher, &scope, &id, 300)
            .await
            .unwrap_err(),
        MediaError::Unsupported
    );
    // HEAD confirms size and type before the asset is ready.
    app.provider()
        .simulate_upload(&video, "video/mp4", 4_096)
        .unwrap();
    let revision = app
        .get(&auth, &teacher, &scope, &id)
        .await
        .unwrap()
        .revision;
    let published = app
        .publish(&auth, &teacher, &scope, &id, revision)
        .await
        .unwrap();
    assert_eq!(published.processing, Processing::Ready);
    assert!(published.published);
    let playback = app
        .playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Original)
        .await
        .unwrap();
    assert_eq!(playback.expires_at, NOW + 60);
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Embed)
            .await
            .unwrap_err(),
        MediaError::Unsupported
    );
    // A ready original is not replaced through a new grant.
    assert_eq!(
        app.upload_declared(&auth, &teacher, &scope, &id, 300, &declared)
            .await
            .unwrap_err(),
        MediaError::Conflict
    );
    // That refused request still refreshed the asset, so reload its revision.
    let revision = app
        .get(&auth, &teacher, &scope, &id)
        .await
        .unwrap()
        .revision;
    let deleted = app
        .delete(&auth, &teacher, &scope, &id, revision)
        .await
        .unwrap();
    assert_eq!(deleted.lifecycle, Lifecycle::Deleted);
    assert!(app.provider().get(&video).await.unwrap().is_none());
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Original)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    app.close().await;
}

#[tokio::test]
async fn wrong_type_or_size_fails_confirmation_and_allows_a_new_grant() {
    let dir = tempfile::tempdir().unwrap();
    let app = service(&dir).await;
    let auth = auth();
    let (teacher, id, scope) = (reference("teacher"), reference("lesson-2"), course());
    let created = app
        .create(
            &auth,
            &teacher,
            &scope,
            &id,
            Metadata::new("Traits", "").unwrap(),
        )
        .await
        .unwrap();
    let video = created.video.unwrap();
    for (content_type, length) in [
        ("text/html", 10),
        ("video/mp4", 10_000_001),
        ("video/mp4", 0),
    ] {
        app.provider()
            .simulate_upload(&video, content_type, length)
            .unwrap();
        let current = app.get(&auth, &teacher, &scope, &id).await.unwrap();
        let refreshed = app
            .refresh(&auth, &teacher, &scope, &id, current.revision)
            .await
            .unwrap();
        assert_eq!(refreshed.processing, Processing::Failed);
        assert_eq!(
            app.publish(&auth, &teacher, &scope, &id, refreshed.revision)
                .await
                .unwrap_err(),
            MediaError::Conflict
        );
    }
    let declared = UploadDeclaration::new("video/webm", 2_048).unwrap();
    assert!(
        app.upload_declared(&auth, &teacher, &scope, &id, 300, &declared)
            .await
            .is_ok()
    );
    app.close().await;
}

#[tokio::test]
async fn grants_are_checked_before_signing() {
    let dir = tempfile::tempdir().unwrap();
    let app = service(&dir).await;
    let auth = auth();
    let (teacher, learner, id, scope) = (
        reference("teacher"),
        reference("learner"),
        reference("lesson-3"),
        course(),
    );
    app.create(
        &auth,
        &teacher,
        &scope,
        &id,
        Metadata::new("Lifetimes", "").unwrap(),
    )
    .await
    .unwrap();
    let declared = UploadDeclaration::new("video/mp4", 1_024).unwrap();
    // Learners cannot upload; other scopes and revoked hosts are denied.
    assert_eq!(
        app.upload_declared(&auth, &learner, &scope, &id, 300, &declared)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    let other = Scope::new("school-b", "rust-course").unwrap();
    assert_eq!(
        app.upload_declared(&auth, &teacher, &other, &id, 300, &declared)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    auth.revoked.store(true, Ordering::SeqCst);
    assert_eq!(
        app.upload_declared(&auth, &teacher, &scope, &id, 300, &declared)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    auth.revoked.store(false, Ordering::SeqCst);
    // The host's permission bounds the grant lifetime.
    auth.until.store(NOW + 45, Ordering::SeqCst);
    let grant = app
        .upload_declared(&auth, &teacher, &scope, &id, 300, &declared)
        .await
        .unwrap();
    assert_eq!(grant.expires_at, NOW + 45);
    // Unpublished assets never receive playback grants.
    assert_eq!(
        app.playback(&auth, &learner, &scope, &id, 60, PlaybackKind::Original)
            .await
            .unwrap_err(),
        MediaError::Denied
    );
    app.close().await;
}

#[test]
fn production_stores_reject_the_offline_mock() {
    assert!(StoreConfig::production(storage(true).binding(), 16).is_err());
    assert!(StoreConfig::production(storage(false).binding(), 16).is_ok());
}
