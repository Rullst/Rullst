use super::{
    S3Storage,
    client::{ORIGINAL, OfflineObject, Sidecar, sha256_hex},
    signing::{object_url, presign, system_time},
};
use crate::{
    MediaError as Error, Metadata, PlaybackGrant, PlaybackKind, Processing, ProviderBinding,
    ProviderMode, RemoteVideo, UploadDeclaration, UploadGrant, UploadProtocol, VideoId,
    VideoProvider,
    contracts::checked_time,
    provider::{marker_valid, validate_remote},
};
use std::time::Duration;

/// Upper bound for presigned upload and playback grants.
pub const MAX_GRANT_SECONDS: u32 = 900;

fn expiration(now: i64, ttl: u32) -> Result<i64, Error> {
    checked_time(now)?;
    if ttl == 0 || ttl > MAX_GRANT_SECONDS {
        return Err(Error::InvalidInput);
    }
    checked_time(now.checked_add(i64::from(ttl)).ok_or(Error::Clock)?)
}

/// The adapter chooses the identity: a UUID-shaped digest of the local
/// creation marker, so a retried create addresses the same objects.
fn derive_id(marker: &str) -> Result<VideoId, Error> {
    let hash = sha256_hex(&["rullst-media-s3|", marker]);
    let part = |range: std::ops::Range<usize>| hash.get(range).ok_or(Error::Configuration);
    VideoId::new(format!(
        "{}-{}-{}-{}-{}",
        part(0..8)?,
        part(8..12)?,
        part(12..16)?,
        part(16..20)?,
        part(20..32)?
    ))
}

impl S3Storage {
    fn offline(&self) -> bool {
        self.binding.mode == ProviderMode::Offline
    }

    fn checked(&self, video: RemoteVideo) -> Result<RemoteVideo, Error> {
        validate_remote(&video, self.binding.library)?;
        Ok(video)
    }

    async fn current(&self, video: &VideoId) -> Result<Option<RemoteVideo>, Error> {
        if self.offline() {
            let map = self.offline.lock().map_err(|_| Error::Storage)?;
            return match map.get(video) {
                None => Ok(None),
                Some(object) => {
                    let sidecar = Sidecar {
                        version: 1,
                        video: video.clone(),
                        title: object.title.clone(),
                        description: object.description.clone(),
                    };
                    let processing = self.processing(object.stored.as_ref());
                    Ok(Some(self.checked(self.remote(sidecar, processing))?))
                }
            };
        }
        let Some(sidecar) = self.read_sidecar(video).await? else {
            return Ok(None);
        };
        let stored = self.head(video).await?;
        let processing = self.processing(stored.as_ref());
        Ok(Some(self.checked(self.remote(sidecar, processing))?))
    }
}

impl VideoProvider for S3Storage {
    fn binding(&self) -> ProviderBinding {
        self.binding.clone()
    }

    async fn create(&self, marker: &str) -> Result<RemoteVideo, Error> {
        if !marker_valid(marker) {
            return Err(Error::InvalidInput);
        }
        let sidecar = Sidecar {
            version: 1,
            video: derive_id(marker)?,
            title: marker.into(),
            description: String::new(),
        };
        if self.offline() {
            let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
            if map.len() >= 10_000 && !map.contains_key(&sidecar.video) {
                return Err(Error::Capacity);
            }
            map.entry(sidecar.video.clone())
                .or_insert_with(|| OfflineObject {
                    title: marker.into(),
                    description: String::new(),
                    stored: None,
                });
        } else {
            // Idempotent: the same marker always writes the same key and body.
            self.write_sidecar(&sidecar).await?;
        }
        self.checked(self.remote(sidecar, Processing::AwaitingUpload))
    }

    async fn find_created(&self, marker: &str) -> Result<Option<RemoteVideo>, Error> {
        if !marker_valid(marker) {
            return Err(Error::InvalidInput);
        }
        match self.current(&derive_id(marker)?).await? {
            Some(video) if video.title == marker => Ok(Some(video)),
            Some(_) => Err(Error::Conflict),
            None => Ok(None),
        }
    }

    async fn get(&self, video: &VideoId) -> Result<Option<RemoteVideo>, Error> {
        self.current(video).await
    }

    async fn update(&self, video: &VideoId, metadata: &Metadata) -> Result<(), Error> {
        if self.offline() {
            let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
            let object = map.get_mut(video).ok_or(Error::NotFound)?;
            object.title = metadata.title().into();
            object.description = metadata.description().into();
            return Ok(());
        }
        if self.read_sidecar(video).await?.is_none() {
            return Err(Error::NotFound);
        }
        self.write_sidecar(&Sidecar {
            version: 1,
            video: video.clone(),
            title: metadata.title().into(),
            description: metadata.description().into(),
        })
        .await
    }

    async fn delete(&self, video: &VideoId) -> Result<(), Error> {
        if self.offline() {
            self.offline
                .lock()
                .map_err(|_| Error::Storage)?
                .remove(video);
            return Ok(());
        }
        self.delete_objects(video).await
    }

    /// Object storage needs the declared type and length; use
    /// [`VideoProvider::upload_declared`].
    fn upload(&self, _video: &VideoId, _now: i64, _ttl: u32) -> Result<UploadGrant, Error> {
        Err(Error::Unsupported)
    }

    fn upload_declared(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        declaration: &UploadDeclaration,
    ) -> Result<UploadGrant, Error> {
        let expires_at = expiration(now, ttl)?;
        let content_type = declaration.content_type();
        if !self.config.content_types.iter().any(|t| t == content_type) {
            return Err(Error::InvalidInput);
        }
        if declaration.length() > self.config.max_object_bytes {
            return Err(Error::Capacity);
        }
        let key = self.key(video, ORIGINAL);
        let length = declaration.length().to_string();
        let (endpoint, signature) = if self.offline() {
            let signature = sha256_hex(&[
                "offline-put|",
                video.as_str(),
                "|",
                &expires_at.to_string(),
                "|",
                content_type,
                "|",
                &length,
            ]);
            (format!("https://rullst-media.invalid/s3/{key}"), signature)
        } else {
            let url = object_url(&self.config, &key)?;
            let signature = presign(
                &self.config,
                "PUT",
                &url,
                &[("content-length", &length), ("content-type", content_type)],
                system_time(now)?,
                Duration::from_secs(u64::from(ttl)),
            )?;
            (url.to_string(), signature)
        };
        Ok(UploadGrant {
            endpoint,
            library: self.binding.library,
            video: video.clone(),
            expires_at,
            signature,
            mode: self.binding.mode,
            protocol: UploadProtocol::PresignedPut,
            content_type: Some(content_type.into()),
            content_length: Some(declaration.length()),
        })
    }

    /// Presigned `GET` of the original; `Range` requests go to the bucket.
    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, Error> {
        if kind != PlaybackKind::Original {
            return Err(Error::Unsupported);
        }
        let expires_at = expiration(now, ttl)?;
        let key = self.key(video, ORIGINAL);
        let url = if self.offline() {
            format!("https://rullst-media.invalid/s3/{key}?expires={expires_at}")
        } else {
            let url = object_url(&self.config, &key)?;
            let query = presign(
                &self.config,
                "GET",
                &url,
                &[],
                system_time(now)?,
                Duration::from_secs(u64::from(ttl)),
            )?;
            format!("{url}?{query}")
        };
        Ok(PlaybackGrant {
            url,
            expires_at,
            mode: self.binding.mode,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn derived_identities_are_stable_and_marker_bound() {
        let first = derive_id("rullst-video-0123456789abcdef0123456789abcdef").unwrap();
        let again = derive_id("rullst-video-0123456789abcdef0123456789abcdef").unwrap();
        let other = derive_id("rullst-video-fedcba9876543210fedcba9876543210").unwrap();
        assert_eq!(first, again);
        assert_ne!(first, other);
    }

    #[test]
    fn grant_lifetimes_are_bounded() {
        assert_eq!(expiration(1_800_000_000, 0), Err(Error::InvalidInput));
        assert_eq!(expiration(1_800_000_000, 901), Err(Error::InvalidInput));
        assert_eq!(expiration(1_800_000_000, 900), Ok(1_800_000_900));
        assert_eq!(expiration(0, 60), Err(Error::Clock));
    }
}
