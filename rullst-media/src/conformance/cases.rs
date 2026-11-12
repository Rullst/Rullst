//! The cases of the video conformance kit, one method each.

use super::VideoExpectations;
use super::report::Check;
use crate::{
    ListQuery, MediaError, Metadata, PlaybackKind, Processing, ProviderMode, RemoteVideo,
    UploadDeclaration, VideoId, VideoProvider,
};
use std::sync::atomic::{AtomicU64, Ordering};

const UPLOAD_TTL: u32 = 600;
const PLAYBACK_TTL: u32 = 300;
const UNKNOWN_VIDEO: &str = "c0f0c0f0-0000-4000-8000-00000000c0f0";
const KINDS: [PlaybackKind; 4] = [
    PlaybackKind::Embed,
    PlaybackKind::Hls,
    PlaybackKind::Mp4_720p,
    PlaybackKind::Original,
];

static MARKERS: AtomicU64 = AtomicU64::new(0);

/// The provider under test and what it is expected to offer.
pub(super) struct Run<'a, P> {
    pub(super) provider: &'a P,
    pub(super) expect: VideoExpectations,
}

/// HTTPS, except for the explicit loopback protocol fixture.
fn secure(url: &str, mode: ProviderMode) -> bool {
    mode == ProviderMode::ProtocolFixture || url.starts_with("https://")
}

impl<P: VideoProvider> Run<'_, P> {
    /// A fresh creation marker: `rullst-video-` and 32 lower-case hex digits.
    fn marker(&self) -> String {
        let count = MARKERS.fetch_add(1, Ordering::Relaxed);
        format!(
            "rullst-video-{:016x}{count:016x}",
            self.expect.now.unsigned_abs()
        )
    }

    /// Creates a video for a case, recording why when it cannot.
    async fn created(&self, check: &mut Check) -> Option<RemoteVideo> {
        match self.provider.create(&self.marker()).await {
            Ok(video) => Some(video),
            Err(error) => {
                check.expect(false, format!("create makes a video: {error}"));
                None
            }
        }
    }

    pub(super) async fn binding_stable(&self) -> Check {
        let mut check = Check::default();
        check.expect(
            self.provider.binding() == self.provider.binding(),
            "binding() returns the same binding every time",
        );
        check
    }

    pub(super) async fn marker_validated(&self) -> Check {
        let mut check = Check::default();
        for marker in [
            "",
            "lesson-1",
            "rullst-video-0123",
            "rullst-video-0123456789ABCDEF0123456789ABCDEF",
        ] {
            let created = self.provider.create(marker).await;
            check.expect(
                matches!(created, Err(MediaError::InvalidInput)),
                format!("create refuses the marker {marker:?} with InvalidInput"),
            );
            let found = self.provider.find_created(marker).await;
            check.expect(
                matches!(found, Err(MediaError::InvalidInput)),
                format!("find_created refuses the marker {marker:?} with InvalidInput"),
            );
        }
        check
    }

    pub(super) async fn lifecycle(&self) -> Check {
        let mut check = Check::default();
        let provider = self.provider;
        let marker = self.marker();
        let video = match provider.create(&marker).await {
            Ok(video) => video,
            Err(error) => {
                check.expect(false, format!("create makes a video: {error}"));
                return check;
            }
        };
        let id = &video.id;
        check.expect(
            video.title == marker,
            "create titles the video with its marker",
        );
        check.expect(
            video.library == provider.binding().library,
            "the video belongs to the binding's library",
        );
        check.expect(
            video.processing == Processing::AwaitingUpload,
            "a new video awaits its upload",
        );
        let found = provider.find_created(&marker).await;
        check.expect(
            found.map(|video| video.map(|video| video.id)) == Ok(Some(id.clone())),
            "find_created finds the video by its marker",
        );
        let read = provider.get(id).await;
        check.expect(
            read.map(|video| video.map(|video| video.id)) == Ok(Some(id.clone())),
            "get reads the video",
        );
        let metadata = match Metadata::new("Conformance lesson", "Checked by the conformance kit.")
        {
            Ok(metadata) => metadata,
            Err(error) => {
                check.expect(false, format!("the kit's metadata is valid: {error}"));
                return check;
            }
        };
        let updated = provider.update(id, &metadata).await;
        check.expect(
            updated.is_ok(),
            format!("update applies metadata, not {updated:?}"),
        );
        let renamed = provider.get(id).await.ok().flatten();
        check.expect(
            renamed.as_ref().map(|video| video.title.as_str()) == Some(metadata.title()),
            "get returns the updated title",
        );
        check.expect(
            renamed.as_ref().map(|video| video.description.as_str())
                == Some(metadata.description()),
            "get returns the updated description",
        );
        for attempt in ["delete removes the video", "deleting it again succeeds"] {
            let deleted = provider.delete(id).await;
            check.expect(deleted.is_ok(), format!("{attempt}, not {deleted:?}"));
        }
        check.expect(
            matches!(provider.get(id).await, Ok(None)),
            "get finds nothing after delete",
        );
        check.expect(
            matches!(provider.find_created(&marker).await, Ok(None)),
            "find_created finds nothing after delete",
        );
        check.expect(
            matches!(
                provider.update(id, &metadata).await,
                Err(MediaError::NotFound)
            ),
            "update of a deleted video is NotFound",
        );
        check
    }

    pub(super) async fn upload_grant(&self) -> Check {
        let mut check = Check::default();
        let Some(video) = self.created(&mut check).await else {
            return check;
        };
        let (provider, now, binding) = (self.provider, self.expect.now, self.provider.binding());
        if self.expect.upload {
            match provider.upload(&video.id, now, UPLOAD_TTL) {
                Ok(grant) => {
                    check.expect(grant.video == video.id, "the upload grant names the video");
                    check.expect(
                        grant.library == binding.library,
                        "the upload grant names the binding's library",
                    );
                    check.expect(
                        grant.mode == binding.mode,
                        "the upload grant keeps the provider mode",
                    );
                    check.expect(
                        grant.expires_at == now + i64::from(UPLOAD_TTL),
                        "the upload grant expires after its TTL",
                    );
                    check.expect(
                        !grant.expose_signature().is_empty(),
                        "the upload grant is signed",
                    );
                    check.expect(
                        secure(&grant.endpoint, binding.mode),
                        "the upload endpoint uses HTTPS",
                    );
                }
                Err(error) => check.expect(false, format!("upload grants an upload: {error}")),
            }
            for (label, at, ttl) in [
                ("a zero TTL", now, 0),
                ("a TTL of 136 years", now, u32::MAX),
                ("a clock before 1970", -1, UPLOAD_TTL),
                ("an unset clock", 0, UPLOAD_TTL),
            ] {
                check.expect(
                    provider.upload(&video.id, at, ttl).is_err(),
                    format!("upload refuses {label}"),
                );
            }
        } else {
            check.expect(
                matches!(
                    provider.upload(&video.id, now, UPLOAD_TTL),
                    Err(MediaError::Unsupported)
                ),
                "without upload support, upload is Unsupported",
            );
        }
        match UploadDeclaration::new("video/mp4", 1_024) {
            Ok(declaration) => {
                match provider.upload_declared(&video.id, now, UPLOAD_TTL, &declaration) {
                    Ok(grant) => {
                        check.expect(
                            grant.content_type.as_deref() == Some("video/mp4"),
                            "a declared upload grant binds the content type",
                        );
                        check.expect(
                            grant.content_length == Some(1_024),
                            "a declared upload grant binds the length",
                        );
                    }
                    Err(error) => check.expect(
                        error == MediaError::Unsupported,
                        format!(
                            "upload_declared binds the declaration or is Unsupported, not {error}"
                        ),
                    ),
                }
            }
            Err(error) => check.expect(false, format!("the kit's declaration is valid: {error}")),
        }
        let _ = provider.delete(&video.id).await;
        check
    }

    pub(super) async fn playback_grant(&self) -> Check {
        let mut check = Check::default();
        let Some(video) = self.created(&mut check).await else {
            return check;
        };
        let (provider, now, mode) = (self.provider, self.expect.now, self.provider.binding().mode);
        let mut granted = 0;
        for kind in KINDS {
            match provider.playback(&video.id, now, PLAYBACK_TTL, kind) {
                Ok(grant) => {
                    granted += 1;
                    check.expect(
                        grant.expires_at == now + i64::from(PLAYBACK_TTL),
                        format!("the {kind:?} grant expires after its TTL"),
                    );
                    check.expect(
                        grant.mode == mode,
                        format!("the {kind:?} grant keeps the provider mode"),
                    );
                    check.expect(
                        secure(grant.expose_url(), mode),
                        format!("the {kind:?} grant uses HTTPS"),
                    );
                    check.expect(
                        provider.playback(&video.id, now, 0, kind).is_err(),
                        format!("{kind:?} playback refuses a zero TTL"),
                    );
                }
                Err(error) => check.expect(
                    error == MediaError::Unsupported,
                    format!("{kind:?} playback is granted or Unsupported, not {error}"),
                ),
            }
        }
        if self.expect.playback {
            check.expect(granted > 0, "at least one playback kind is granted");
        } else {
            check.expect(
                granted == 0,
                "without playback support (upload-only credentials), every playback kind is Unsupported",
            );
        }
        let _ = provider.delete(&video.id).await;
        check
    }

    pub(super) async fn list_bounded(&self) -> Check {
        let mut check = Check::default();
        for page in [1, 2] {
            let query = match ListQuery::new(page, ListQuery::MIN_PER_PAGE) {
                Ok(query) => query,
                Err(error) => {
                    check.expect(false, format!("the kit's query is valid: {error}"));
                    return check;
                }
            };
            match self.provider.list(&query).await {
                Ok(listed) if self.expect.listing => {
                    check.expect(
                        listed.page == page,
                        format!("page {page} echoes its number"),
                    );
                    check.expect(
                        listed.per_page == query.per_page(),
                        format!("page {page} echoes its size"),
                    );
                    check.expect(
                        listed.items.len() <= query.per_page() as usize,
                        format!("page {page} holds at most per_page videos"),
                    );
                    check.expect(
                        listed.total >= listed.items.len() as u64,
                        format!("page {page}'s total counts at least its videos"),
                    );
                }
                Ok(_) => check.expect(false, "without listing support, list is Unsupported"),
                Err(MediaError::Unsupported) if !self.expect.listing => {}
                Err(error) => check.expect(false, format!("list reads page {page}: {error}")),
            }
        }
        check
    }

    pub(super) async fn deterministic(&self) -> Check {
        let mut check = Check::default();
        if self.provider.binding().mode != ProviderMode::Offline {
            check.skip("the provider is not in offline mode");
            return check;
        }
        let Some(video) = self.created(&mut check).await else {
            return check;
        };
        let (provider, now) = (self.provider, self.expect.now);
        let upload = || {
            provider
                .upload(&video.id, now, UPLOAD_TTL)
                .map(|grant| serde_json::to_string(&grant).ok())
        };
        check.expect(
            upload() == upload(),
            "the same upload request gives the same grant",
        );
        let playback = || {
            provider
                .playback(&video.id, now, PLAYBACK_TTL, PlaybackKind::Hls)
                .map(|grant| serde_json::to_string(&grant).ok())
        };
        check.expect(
            playback() == playback(),
            "the same playback request gives the same grant",
        );
        match ListQuery::new(1, ListQuery::MIN_PER_PAGE) {
            Ok(query) => check.expect(
                provider.list(&query).await == provider.list(&query).await,
                "the same listing request gives the same page",
            ),
            Err(error) => check.expect(false, format!("the kit's query is valid: {error}")),
        }
        let _ = provider.delete(&video.id).await;
        check
    }

    pub(super) async fn typed_errors(&self) -> Check {
        let mut check = Check::default();
        let provider = self.provider;
        let (unknown, metadata) = match (
            VideoId::new(UNKNOWN_VIDEO),
            Metadata::new("Conformance lesson", ""),
        ) {
            (Ok(unknown), Ok(metadata)) => (unknown, metadata),
            _ => {
                check.expect(false, "the kit's identifiers are valid");
                return check;
            }
        };
        check.expect(
            matches!(provider.get(&unknown).await, Ok(None)),
            "get of an unknown video is Ok(None)",
        );
        let deleted = provider.delete(&unknown).await;
        check.expect(
            deleted.is_ok(),
            format!("delete of an unknown video succeeds, not {deleted:?}"),
        );
        check.expect(
            matches!(
                provider.update(&unknown, &metadata).await,
                Err(MediaError::NotFound)
            ),
            "update of an unknown video is NotFound",
        );
        check.expect(
            provider.upload(&unknown, i64::MAX, u32::MAX).is_err(),
            "upload refuses a clock at the end of time",
        );
        for kind in KINDS {
            check.expect(
                provider.playback(&unknown, i64::MIN, 1, kind).is_err(),
                format!("{kind:?} playback refuses a clock before 1970"),
            );
        }
        if let Ok(query) = ListQuery::new(ListQuery::MAX_PAGE, ListQuery::MAX_PER_PAGE) {
            // The last page is empty or Unsupported, never a panic.
            let _ = provider.list(&query).await;
        }
        check
    }
}
