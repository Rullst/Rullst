#![cfg(all(feature = "bunny", feature = "sqlite"))]
//! Service guards around the offline Bunny adapter, wrapped by a provider
//! that breaks (or relaxes) one rule at a time.
mod support;
use rullst_media::{bunny::BunnyStream, sqlite::*, *};
use std::sync::Mutex;
use support::*;

const OTHER_VIDEO: &str = "12345678-1234-4234-8234-0000000000ff";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Twist {
    None,
    UploadMode,
    UploadLibrary,
    UploadVideo,
    UploadExpiry,
    DeclaredType,
    DeclaredLength,
    /// Accepts any requested upload lifetime, clamped to Bunny's bounds.
    LenientTtl,
    PlaybackMode,
    FailUpdate,
    /// Applies the title of an update but keeps another description.
    TitleOnly,
}

struct Guarded {
    inner: BunnyStream,
    twist: Mutex<Twist>,
}
impl Guarded {
    fn set(&self, twist: Twist) {
        *self.twist.lock().unwrap() = twist;
    }
    fn twist(&self) -> Twist {
        *self.twist.lock().unwrap()
    }
}
impl VideoProvider for Guarded {
    fn binding(&self) -> ProviderBinding {
        self.inner.binding()
    }
    async fn create(&self, marker: &str) -> Result<RemoteVideo, MediaError> {
        self.inner.create(marker).await
    }
    async fn find_created(&self, marker: &str) -> Result<Option<RemoteVideo>, MediaError> {
        self.inner.find_created(marker).await
    }
    async fn get(&self, video: &VideoId) -> Result<Option<RemoteVideo>, MediaError> {
        self.inner.get(video).await
    }
    async fn update(&self, video: &VideoId, metadata: &Metadata) -> Result<(), MediaError> {
        match self.twist() {
            Twist::FailUpdate => Err(MediaError::Unavailable),
            Twist::TitleOnly => {
                let kept = Metadata::new(metadata.title(), "Provider summary").unwrap();
                self.inner.update(video, &kept).await
            }
            _ => self.inner.update(video, metadata).await,
        }
    }
    async fn delete(&self, video: &VideoId) -> Result<(), MediaError> {
        self.inner.delete(video).await
    }
    fn upload(&self, video: &VideoId, now: i64, ttl: u32) -> Result<UploadGrant, MediaError> {
        let twist = self.twist();
        let ttl = if twist == Twist::LenientTtl {
            ttl.clamp(1, 3600)
        } else {
            ttl
        };
        let mut grant = self.inner.upload(video, now, ttl)?;
        match twist {
            Twist::UploadMode => grant.mode = ProviderMode::ProtocolFixture,
            Twist::UploadLibrary => grant.library = LibraryId::new(8).unwrap(),
            Twist::UploadVideo => grant.video = VideoId::new(OTHER_VIDEO).unwrap(),
            Twist::UploadExpiry => grant.expires_at += 10_000,
            _ => {}
        }
        Ok(grant)
    }
    fn upload_declared(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        declaration: &UploadDeclaration,
    ) -> Result<UploadGrant, MediaError> {
        let mut grant = self.upload(video, now, ttl)?;
        grant.protocol = UploadProtocol::PresignedPut;
        grant.content_type = Some(declaration.content_type().to_owned());
        grant.content_length = Some(declaration.length());
        match self.twist() {
            Twist::DeclaredType => grant.content_type = Some("video/webm".into()),
            Twist::DeclaredLength => grant.content_length = Some(declaration.length() + 1),
            _ => {}
        }
        Ok(grant)
    }
    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, MediaError> {
        let mut grant = self.inner.playback(video, now, ttl, kind)?;
        if self.twist() == Twist::PlaybackMode {
            grant.mode = ProviderMode::ProtocolFixture;
        }
        Ok(grant)
    }
}

async fn service(dir: &tempfile::TempDir, clock: TestClock) -> MediaService<Guarded, TestClock> {
    let provider = Guarded {
        inner: BunnyStream::new(config("offline")).unwrap(),
        twist: Mutex::new(Twist::None),
    };
    let store = SqliteMedia::initialize(
        dir.path().join("video.sqlite"),
        StoreConfig::testing(provider.binding(), 32).unwrap(),
        clock,
    )
    .await
    .unwrap();
    MediaService::new(provider, store).unwrap()
}

struct Course {
    app: MediaService<Guarded, TestClock>,
    auth: Auth,
    teacher: Reference,
    learner: Reference,
    scope: Scope,
    id: Reference,
    _dir: tempfile::TempDir,
}
impl Course {
    async fn new(clock: TestClock) -> (Self, Asset) {
        let dir = tempfile::tempdir().unwrap();
        let course = Self {
            app: service(&dir, clock).await,
            auth: Auth::new(),
            teacher: reference("teacher"),
            learner: reference("learner"),
            scope: scope(),
            id: reference("lesson"),
            _dir: dir,
        };
        let created = course.create("lesson").await;
        (course, created)
    }
    async fn create(&self, id: &str) -> Asset {
        let (auth, teacher, scope) = (&self.auth, &self.teacher, &self.scope);
        self.app
            .create(auth, teacher, scope, &reference(id), metadata())
            .await
            .unwrap()
    }
    async fn get(&self) -> Asset {
        let (auth, teacher, scope) = (&self.auth, &self.teacher, &self.scope);
        self.app.get(auth, teacher, scope, &self.id).await.unwrap()
    }
    async fn upload(&self, ttl: u32) -> Result<UploadGrant, MediaError> {
        let (auth, teacher, scope) = (&self.auth, &self.teacher, &self.scope);
        self.app.upload(auth, teacher, scope, &self.id, ttl).await
    }
    async fn playback(&self) -> Result<PlaybackGrant, MediaError> {
        let (auth, learner, scope) = (&self.auth, &self.learner, &self.scope);
        let embed = PlaybackKind::Embed;
        self.app
            .playback(auth, learner, scope, &self.id, 60, embed)
            .await
    }
    async fn update(&self, revision: i64, metadata: Metadata) -> Result<Asset, MediaError> {
        let (auth, teacher, scope) = (&self.auth, &self.teacher, &self.scope);
        self.app
            .update(auth, teacher, scope, &self.id, revision, metadata)
            .await
    }
    fn process(&self, created: &Asset, processing: Processing) {
        let video = created.video.as_ref().unwrap();
        let provider = self.app.provider();
        provider
            .inner
            .simulate_processing(video, processing)
            .unwrap();
    }
    async fn publish(&self, created: &Asset) -> Asset {
        self.process(created, Processing::Ready);
        let (auth, teacher, scope) = (&self.auth, &self.teacher, &self.scope);
        let revision = created.revision;
        self.app
            .publish(auth, teacher, scope, &self.id, revision)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn course_inventory_pages_hold_one_to_one_hundred_assets() {
    let (course, _) = Course::new(TestClock::new()).await;
    course.create("next-lesson").await;
    let (app, auth, teacher, scope) = (&course.app, &course.auth, &course.teacher, &course.scope);
    let ids = |assets: Vec<Asset>| -> Vec<String> {
        assets
            .into_iter()
            .map(|a| a.id.as_str().to_owned())
            .collect()
    };
    let first = app.list(auth, teacher, scope, None, 1).await.unwrap();
    assert_eq!(ids(first), ["lesson"]);
    let after = reference("lesson");
    let rest = app.list(auth, teacher, scope, Some(&after), 100).await;
    assert_eq!(ids(rest.unwrap()), ["next-lesson"]);
    for limit in [0, 101] {
        assert_eq!(
            app.list(auth, teacher, scope, None, limit).await,
            Err(MediaError::InvalidInput),
            "{limit}"
        );
    }
}

#[tokio::test]
async fn upload_needs_a_video_still_awaiting_its_upload() {
    let (course, created) = Course::new(TestClock::new()).await;
    course.process(&created, Processing::Ready);
    assert_eq!(course.upload(300).await.err(), Some(MediaError::Conflict));
    let current = course.get().await;
    assert_eq!(current.processing, Processing::Ready);
    assert!(!current.pending);
}

#[tokio::test]
async fn upload_grants_must_match_the_binding_video_and_permission() {
    let (course, created) = Course::new(TestClock::new()).await;
    let provider = course.app.provider();
    for twist in [
        Twist::UploadMode,
        Twist::UploadLibrary,
        Twist::UploadVideo,
        Twist::UploadExpiry,
    ] {
        provider.set(twist);
        let refused = course.upload(300).await;
        assert_eq!(refused.err(), Some(MediaError::Protocol), "{twist:?}");
    }
    provider.set(Twist::None);
    let grant = course.upload(300).await.unwrap();
    assert_eq!(grant.expires_at, NOW + 300);
    assert_eq!(Some(&grant.video), created.video.as_ref());
}

#[tokio::test]
async fn declared_uploads_must_carry_the_declared_type_and_length() {
    let (course, _) = Course::new(TestClock::new()).await;
    let (app, auth, teacher, scope) = (&course.app, &course.auth, &course.teacher, &course.scope);
    let declared = UploadDeclaration::new("video/mp4", 1024).unwrap();
    let id = &course.id;
    let grant = app
        .upload_declared(auth, teacher, scope, id, 300, &declared)
        .await
        .unwrap();
    assert_eq!(grant.content_type.as_deref(), Some("video/mp4"));
    assert_eq!(grant.content_length, Some(1024));
    for twist in [Twist::DeclaredType, Twist::DeclaredLength] {
        app.provider().set(twist);
        let refused = app
            .upload_declared(auth, teacher, scope, id, 300, &declared)
            .await;
        assert_eq!(refused.err(), Some(MediaError::Protocol), "{twist:?}");
    }
}

#[tokio::test]
async fn requested_upload_lifetimes_are_one_second_to_one_hour() {
    let (course, _) = Course::new(TestClock::new()).await;
    // A provider that accepts any lifetime must not widen the service's bounds.
    course.app.provider().set(Twist::LenientTtl);
    for ttl in [0, 3601] {
        let refused = course.upload(ttl).await;
        assert_eq!(refused.err(), Some(MediaError::InvalidInput), "{ttl}");
    }
    assert_eq!(course.upload(3600).await.unwrap().expires_at, NOW + 3600);
}

#[tokio::test]
async fn playback_grants_must_keep_the_binding_mode() {
    let (course, created) = Course::new(TestClock::new()).await;
    course.publish(&created).await;
    course.app.provider().set(Twist::PlaybackMode);
    assert_eq!(course.playback().await.err(), Some(MediaError::Protocol));
    course.app.provider().set(Twist::None);
    assert_eq!(course.playback().await.unwrap().expires_at, NOW + 60);
}

#[tokio::test]
async fn a_live_lease_defers_playback_observations_until_its_last_second() {
    let clock = TestClock::new();
    let (course, created) = Course::new(clock.clone()).await;
    let published = course.publish(&created).await;
    // The update's lease is taken at NOW and ends at NOW + 45.
    course.app.provider().set(Twist::FailUpdate);
    let revised = Metadata::new("Revised lesson", "New transcript").unwrap();
    let failed = course.update(published.revision, revised.clone()).await;
    assert_eq!(failed.err(), Some(MediaError::Unavailable));
    let leased = course.get().await;
    assert!(leased.pending && leased.published);
    assert_eq!(leased.failure, None);
    // A stale revision conflicts even while that intent is pending.
    let stale = course.update(leased.revision - 1, metadata()).await;
    assert_eq!(stale.err(), Some(MediaError::Conflict));
    course.process(&created, Processing::Playable);
    clock.set(NOW + 44);
    course.playback().await.unwrap();
    let held = course.get().await;
    assert_eq!(held.revision, leased.revision);
    assert_eq!(held.processing, Processing::Ready);
    clock.set(NOW + 45);
    course.playback().await.unwrap();
    let observed = course.get().await;
    assert_eq!(observed.revision, leased.revision + 1);
    assert_eq!(observed.processing, Processing::Playable);
    assert!(observed.published);
    // The expired lease can be taken again in its last second.
    course.app.provider().set(Twist::None);
    let (auth, teacher, scope) = (&course.auth, &course.teacher, &course.scope);
    let reconciled = course
        .app
        .reconcile(auth, teacher, scope, &course.id)
        .await
        .unwrap();
    assert!(!reconciled.pending);
    assert_eq!(reconciled.metadata, revised);
}

#[tokio::test]
async fn metadata_verification_compares_the_description_too() {
    let (course, created) = Course::new(TestClock::new()).await;
    course.app.provider().set(Twist::TitleOnly);
    let revised = Metadata::new("Revised lesson", "New transcript").unwrap();
    let failed = course.update(created.revision, revised).await;
    assert_eq!(failed.err(), Some(MediaError::Uncertain));
    let stopped = course.get().await;
    assert_eq!(
        stopped.failure,
        Some(OperationFailure::VerificationMismatch)
    );
}

#[tokio::test]
async fn the_newest_32_notification_digests_are_retained() {
    let clock = TestClock::new();
    let (course, created) = Course::new(clock.clone()).await;
    let video = created.video.unwrap();
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, b"mock_webhook");
    // Distinct bytes (and digests) for one notification about one video.
    let event = |spaces: usize| {
        let body = format!(
            r#"{{{}"VideoLibraryId":7,"VideoGuid":"{}","Status":3}}"#,
            " ".repeat(spaces),
            video.as_str()
        );
        let signature: String = ring::hmac::sign(&key, body.as_bytes())
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let headers = WebhookHeaders::new("v1", "hmac-sha256", &signature);
        let provider = &course.app.provider().inner;
        provider
            .verify_notification(headers, body.as_bytes())
            .unwrap()
    };
    for spaces in 0..33 {
        clock.advance(2);
        let fresh = course.app.notification(&event(spaces)).await;
        assert_eq!(fresh, Ok(true), "{spaces}");
    }
    clock.advance(2);
    assert_eq!(course.app.notification(&event(1)).await, Ok(false));
    assert_eq!(course.app.notification(&event(0)).await, Ok(true));
}

#[tokio::test]
async fn closing_the_service_closes_its_store() {
    let (course, _) = Course::new(TestClock::new()).await;
    course.app.close().await;
    let (auth, teacher, scope) = (&course.auth, &course.teacher, &course.scope);
    let closed = course.app.get(auth, teacher, scope, &course.id).await;
    assert_eq!(closed.err(), Some(MediaError::Storage));
}

#[test]
fn a_tenant_quota_may_equal_the_store_capacity() {
    let binding = BunnyStream::new(config("offline")).unwrap().binding();
    let store = StoreConfig::testing(binding, 4).unwrap();
    assert!(store.clone().tenant_assets(4).is_ok());
    assert_eq!(
        store.tenant_assets(5).unwrap_err(),
        MediaError::Configuration
    );
}
