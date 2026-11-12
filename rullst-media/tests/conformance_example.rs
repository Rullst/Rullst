//! An application-owned video provider that creates its own grants and
//! listing pages and passes the video conformance kit, without a network.
//! `docs/src/provider-conformance.md` walks through it.
#![cfg(feature = "conformance")]

use rullst_media::conformance::{VideoExpectations, check_video_provider};
use rullst_media::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

// ANCHOR: provider
/// Longest upload grant, in seconds.
const MAX_UPLOAD_TTL: u32 = 3_600;
/// Longest playback grant, in seconds.
const MAX_PLAYBACK_TTL: u32 = 900;

/// Adapter for the fictitious ExampleVideo service, here with its offline
/// library kept in memory. ExampleVideo signs its own upload and HLS URLs and
/// lists its library.
pub struct ExampleVideo {
    binding: ProviderBinding,
    videos: Mutex<BTreeMap<VideoId, RemoteVideo>>,
}

impl ExampleVideo {
    pub fn offline(library: LibraryId) -> Result<Self, MediaError> {
        Ok(Self {
            binding: ProviderBinding {
                library,
                mode: ProviderMode::Offline,
                environment: Reference::new("example-video")?,
            },
            videos: Mutex::new(BTreeMap::new()),
        })
    }

    /// ExampleVideo's identifier for a creation marker (`rullst-video-` and
    /// 32 lower-case hex digits): the digits as a GUID.
    fn id_for(marker: &str) -> Result<VideoId, MediaError> {
        let hex = marker
            .strip_prefix("rullst-video-")
            .filter(|hex| hex.len() == 32)
            .filter(|hex| {
                hex.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .ok_or(MediaError::InvalidInput)?;
        let (a, b, c, d, e) = (
            &hex[..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..],
        );
        VideoId::new(format!("{a}-{b}-{c}-{d}-{e}"))
    }

    fn expiry(now: i64, ttl: u32, max_ttl: u32) -> Result<i64, MediaError> {
        if ttl == 0 || ttl > max_ttl {
            return Err(MediaError::InvalidInput);
        }
        if now <= 0 {
            return Err(MediaError::Clock);
        }
        now.checked_add(i64::from(ttl)).ok_or(MediaError::Clock)
    }

    /// Stands in for ExampleVideo's signature: a real adapter computes it
    /// with the service's key (an HMAC over the purpose, video and expiry).
    fn token(purpose: &str, video: &VideoId, expires_at: i64) -> String {
        format!("{purpose}.{}.{expires_at}", video.as_str())
    }

    fn videos(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<VideoId, RemoteVideo>>, MediaError> {
        self.videos.lock().map_err(|_| MediaError::Storage)
    }
}

impl VideoProvider for ExampleVideo {
    fn binding(&self) -> ProviderBinding {
        self.binding.clone()
    }

    async fn create(&self, marker: &str) -> Result<RemoteVideo, MediaError> {
        let video = RemoteVideo {
            id: Self::id_for(marker)?,
            library: self.binding.library,
            title: marker.to_owned(),
            description: String::new(),
            processing: Processing::AwaitingUpload,
            length_seconds: 0,
            mp4_720p: false,
        };
        self.videos()?.insert(video.id.clone(), video.clone());
        Ok(video)
    }

    async fn find_created(&self, marker: &str) -> Result<Option<RemoteVideo>, MediaError> {
        Self::id_for(marker)?;
        Ok(self
            .videos()?
            .values()
            .find(|video| video.title == marker)
            .cloned())
    }

    async fn get(&self, video: &VideoId) -> Result<Option<RemoteVideo>, MediaError> {
        Ok(self.videos()?.get(video).cloned())
    }

    async fn update(&self, video: &VideoId, metadata: &Metadata) -> Result<(), MediaError> {
        let mut videos = self.videos()?;
        let stored = videos.get_mut(video).ok_or(MediaError::NotFound)?;
        stored.title = metadata.title().to_owned();
        stored.description = metadata.description().to_owned();
        Ok(())
    }

    async fn delete(&self, video: &VideoId) -> Result<(), MediaError> {
        self.videos()?.remove(video);
        Ok(())
    }

    fn upload(&self, video: &VideoId, now: i64, ttl: u32) -> Result<UploadGrant, MediaError> {
        let expires_at = Self::expiry(now, ttl, MAX_UPLOAD_TTL)?;
        UploadGrant::new(
            &self.binding,
            video.clone(),
            UploadProtocol::Tus,
            "https://upload.example-video.invalid/tus",
            Self::token("upload", video, expires_at),
            expires_at,
        )
    }

    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, MediaError> {
        // ExampleVideo serves HLS only.
        if kind != PlaybackKind::Hls {
            return Err(MediaError::Unsupported);
        }
        let expires_at = Self::expiry(now, ttl, MAX_PLAYBACK_TTL)?;
        let token = Self::token("play", video, expires_at);
        let url = format!(
            "https://cdn.example-video.invalid/{}/playlist.m3u8?expires={expires_at}&token={token}",
            video.as_str()
        );
        PlaybackGrant::new(&self.binding, url, expires_at)
    }

    async fn list(&self, query: &ListQuery) -> Result<VideoPage, MediaError> {
        let term = query.search().map(str::to_lowercase);
        let mut matches = Vec::new();
        for video in self.videos()?.values() {
            if term
                .as_ref()
                .is_none_or(|term| video.title.to_lowercase().contains(term))
            {
                let listed = ListedVideo::new(
                    video.id.clone(),
                    &video.title,
                    video.length_seconds,
                    video.processing,
                )?;
                matches.push(listed);
            }
        }
        // ExampleVideo keeps no upload dates, so both orders list by title.
        matches.sort_by(|a, b| a.title.cmp(&b.title));
        let total = matches.len() as u64;
        let skip = (query.page() as usize - 1) * query.per_page() as usize;
        let items = matches
            .into_iter()
            .skip(skip)
            .take(query.per_page() as usize)
            .collect();
        VideoPage::new(query, items, total)
    }
}
// ANCHOR_END: provider

// ANCHOR: check
#[tokio::test]
async fn example_video_passes_the_video_conformance_kit() {
    let provider = ExampleVideo::offline(LibraryId::new(7).unwrap()).unwrap();
    // Upload grants, HLS playback and listing, checked at this UTC second.
    let expect = VideoExpectations::new(1_793_534_400);
    let report = check_video_provider("example-video", &provider, expect).await;
    println!("{report}");
    report.assert_passed();
}
// ANCHOR_END: check

#[tokio::test]
async fn example_video_lists_its_library_in_bounded_pages() {
    let provider = ExampleVideo::offline(LibraryId::new(7).unwrap()).unwrap();
    for (index, title) in ["Ownership", "Borrowing", "Lifetimes"].iter().enumerate() {
        let video = provider
            .create(&format!("rullst-video-{:032x}", index + 1))
            .await
            .unwrap();
        provider
            .update(&video.id, &Metadata::new(*title, "").unwrap())
            .await
            .unwrap();
    }
    let titles = |page: &VideoPage| {
        page.items
            .iter()
            .map(|video| video.title.clone())
            .collect::<Vec<_>>()
    };
    let first = provider
        .list(&ListQuery::new(1, 10).unwrap())
        .await
        .unwrap();
    assert_eq!(titles(&first), ["Borrowing", "Lifetimes", "Ownership"]);
    assert_eq!((first.page, first.per_page, first.total), (1, 10, 3));
    assert!(!first.has_more());
    let search = ListQuery::new(1, 10).unwrap().with_search("LIFE").unwrap();
    let found = provider.list(&search).await.unwrap();
    assert_eq!(
        (titles(&found), found.total),
        (vec!["Lifetimes".to_owned()], 1)
    );
    let second = provider
        .list(&ListQuery::new(2, 10).unwrap())
        .await
        .unwrap();
    assert!(second.items.is_empty());
    assert_eq!(second.total, 3);
}
