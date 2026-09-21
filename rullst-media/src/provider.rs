use crate::{
    MediaError, Metadata, PlaybackGrant, PlaybackKind, ProviderBinding, RemoteVideo, UploadGrant,
    VideoId,
};
use std::future::Future;

/// One configured provider library. Implementations must bound I/O, validate
/// response identity, preserve mode and never retry an ambiguous create.
/// Trusted adapter contract, also usable for instrumentation wrappers. This
/// candidate supplies only Bunny Stream; the grant/notification wire formats
/// are not a promise of compatibility with arbitrary video providers.
pub trait VideoProvider: Send + Sync {
    fn binding(&self) -> ProviderBinding;
    fn create(&self, marker: &str) -> impl Future<Output = Result<RemoteVideo, MediaError>> + Send;
    fn find_created(
        &self,
        marker: &str,
    ) -> impl Future<Output = Result<Option<RemoteVideo>, MediaError>> + Send;
    fn get(
        &self,
        video: &VideoId,
    ) -> impl Future<Output = Result<Option<RemoteVideo>, MediaError>> + Send;
    fn update(
        &self,
        video: &VideoId,
        metadata: &Metadata,
    ) -> impl Future<Output = Result<(), MediaError>> + Send;
    fn delete(&self, video: &VideoId) -> impl Future<Output = Result<(), MediaError>> + Send;
    fn upload(
        &self,
        video: &VideoId,
        now: i64,
        ttl_seconds: u32,
    ) -> Result<UploadGrant, MediaError>;
    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl_seconds: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, MediaError>;
}

#[cfg(any(feature = "bunny", feature = "sqlite"))]
pub(crate) fn validate_remote(
    video: &RemoteVideo,
    library: crate::LibraryId,
) -> Result<(), MediaError> {
    if video.library != library
        || video.title.is_empty()
        || video.title.len() > 256
        || video.description.len() > 4096
        || video.length_seconds > 604_800
    {
        return Err(MediaError::Protocol);
    }
    Ok(())
}
