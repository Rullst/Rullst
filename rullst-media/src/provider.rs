use crate::{
    ListQuery, MediaError, Metadata, PlaybackGrant, PlaybackKind, ProviderBinding, RemoteVideo,
    UploadDeclaration, UploadGrant, VideoId, VideoPage,
};
use std::future::Future;

/// One configured provider library. Implementations must bound I/O, validate
/// response identity, preserve mode and never retry an ambiguous create.
/// Trusted adapter contract, also usable for instrumentation wrappers. This
/// candidate supplies Bunny Stream and S3-compatible object storage; the
/// grant/notification wire formats are not a promise of compatibility with
/// arbitrary video providers.
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
    /// v13: an upload grant bound to a host-validated declaration. The grant
    /// must carry the declared type and length. Adapters that cannot bind them
    /// keep this default and return `Unsupported`.
    fn upload_declared(
        &self,
        video: &VideoId,
        now: i64,
        ttl_seconds: u32,
        declaration: &UploadDeclaration,
    ) -> Result<UploadGrant, MediaError> {
        let _ = (video, now, ttl_seconds, declaration);
        Err(MediaError::Unsupported)
    }
    /// v13: [`create`](Self::create) followed by [`update`](Self::update), so
    /// the provider shows the real title. The marker stays the identity of
    /// the creation: after `Unavailable`, `Protocol` or `Uncertain`, call
    /// [`find_created`](Self::find_created) with it, never `create` again.
    /// When applying `metadata` fails after the video was created, the video
    /// is still returned, with its marker title; retry `update` with its id.
    fn create_with_metadata(
        &self,
        marker: &str,
        metadata: &Metadata,
    ) -> impl Future<Output = Result<RemoteVideo, MediaError>> + Send {
        async move {
            let mut video = self.create(marker).await?;
            if self.update(&video.id, metadata).await.is_ok() {
                video.title = metadata.title().to_owned();
                video.description = metadata.description().to_owned();
            }
            Ok(video)
        }
    }
    /// v13: one page of the provider library, including videos created
    /// outside this application. Adapters without a listing keep this
    /// default and return `Unsupported`.
    fn list(
        &self,
        query: &ListQuery,
    ) -> impl Future<Output = Result<VideoPage, MediaError>> + Send {
        let _ = query;
        std::future::ready(Err(MediaError::Unsupported))
    }
}

/// The local creation marker `rullst-video-` plus 32 lower-case hex digits.
#[cfg(any(feature = "bunny", feature = "s3"))]
pub(crate) fn marker_valid(marker: &str) -> bool {
    marker.len() == 45
        && marker.starts_with("rullst-video-")
        && marker[13..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(any(feature = "bunny", feature = "s3", feature = "sqlite"))]
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
