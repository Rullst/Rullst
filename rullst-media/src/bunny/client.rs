use super::{BunnyConfig, signatures::sha, wire};
use crate::{
    MediaError as Error, Metadata, PlaybackGrant, PlaybackKind, Processing, ProviderBinding,
    ProviderMode, Reference, RemoteVideo, UploadGrant, VideoId, VideoProvider,
};
use reqwest::{
    Client, Method, Url,
    header::{ACCEPT, CONTENT_TYPE, HeaderValue},
};
use serde::de::DeserializeOwned;
use std::{collections::BTreeMap, sync::Mutex, time::Duration};

pub struct BunnyStream {
    pub(super) config: BunnyConfig,
    pub(super) origin: String,
    binding: ProviderBinding,
    client: Client,
    offline: Mutex<BTreeMap<VideoId, RemoteVideo>>,
}
impl std::fmt::Debug for BunnyStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BunnyStream")
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}
impl BunnyStream {
    /// Fixed provider origin; empty/mock keys select offline behavior.
    pub fn new(config: BunnyConfig) -> Result<Self, Error> {
        if config.credentials.mode == ProviderMode::ProtocolFixture {
            return Err(Error::Configuration);
        }
        Self::build(config, "https://video.bunnycdn.com".into())
    }

    /// Explicit local protocol fixture. Accepts only fixture_* keys and a literal
    /// loopback origin; receipts retain ProtocolFixture and production rejects it.
    pub fn for_protocol_tests(
        config: BunnyConfig,
        origin: impl Into<String>,
    ) -> Result<Self, Error> {
        let origin = origin.into();
        let url = Url::parse(&origin).map_err(|_| Error::Configuration)?;
        let loopback = url
            .host_str()
            .and_then(|host| {
                host.trim_start_matches('[')
                    .trim_end_matches(']')
                    .parse::<std::net::IpAddr>()
                    .ok()
            })
            .is_some_and(|ip| ip.is_loopback());
        if config.credentials.mode != ProviderMode::ProtocolFixture
            || !loopback
            || !["http", "https"].contains(&url.scheme())
            || url.path() != "/"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.port() == Some(0)
        {
            return Err(Error::Configuration);
        }
        Self::build(config, url.as_str().trim_end_matches('/').to_owned())
    }

    fn build(config: BunnyConfig, origin: String) -> Result<Self, Error> {
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent("rullst-media/13")
            .build()
            .map_err(|_| Error::Configuration)?;
        let fingerprint = sha(&[
            config.environment.as_str(),
            "|",
            &origin,
            "|",
            &config.cdn_host,
        ]);
        let binding = ProviderBinding {
            library: config.library,
            mode: config.credentials.mode,
            environment: Reference::new(fingerprint)?,
        };
        Ok(Self {
            config,
            origin,
            binding,
            client,
            offline: Mutex::new(BTreeMap::new()),
        })
    }

    fn path(&self, video: Option<&VideoId>) -> String {
        let mut path = format!("/library/{}/videos", self.config.library.value());
        if let Some(video) = video {
            path.push('/');
            path.push_str(video.as_str());
        }
        path
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, Error> {
        let mut key = HeaderValue::from_str(&self.config.credentials.api)
            .map_err(|_| Error::Configuration)?;
        key.set_sensitive(true);
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.origin))
            .header("AccessKey", key)
            .header(ACCEPT, "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let mut response = request.send().await.map_err(|_| Error::Unavailable)?;
        match response.status().as_u16() {
            200 => (),
            404 => return Err(Error::NotFound),
            429 | 500..=599 => return Err(Error::Unavailable),
            400..=499 => return Err(Error::Rejected),
            _ => return Err(Error::Protocol),
        }
        if !response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| {
                h.split(';')
                    .next()
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("application/json"))
            })
            || response.content_length().is_some_and(|n| n > 1_048_576)
        {
            return Err(Error::Protocol);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if chunk.len() > 1_048_576usize.saturating_sub(bytes.len()) {
                return Err(Error::Protocol);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::Protocol)
    }

    async fn read<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        // Only safe reads retry, once. Bound both attempts and their bodies together.
        tokio::time::timeout(Duration::from_secs(12), async {
            match self.request(Method::GET, path, None).await {
                Err(Error::Unavailable) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    self.request(Method::GET, path, None).await
                }
                result => result,
            }
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }

    /// Explicit offline fixture control; never emits a real provider observation.
    pub fn simulate_processing(
        &self,
        video: &VideoId,
        processing: Processing,
    ) -> Result<(), Error> {
        if self.binding.mode != ProviderMode::Offline {
            return Err(Error::Unsupported);
        }
        let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
        let found = map.get_mut(video).ok_or(Error::NotFound)?;
        found.processing = processing;
        Ok(())
    }
}

fn marker_valid(marker: &str) -> bool {
    marker.len() == 45
        && marker.starts_with("rullst-video-")
        && marker[13..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl VideoProvider for BunnyStream {
    fn binding(&self) -> ProviderBinding {
        self.binding.clone()
    }

    async fn create(&self, marker: &str) -> Result<RemoteVideo, Error> {
        if !marker_valid(marker) {
            return Err(Error::InvalidInput);
        }
        if self.binding.mode == ProviderMode::Offline {
            let hash = sha(&[marker]);
            let id = VideoId::new(format!(
                "{}-{}-{}-{}-{}",
                &hash[..8],
                &hash[8..12],
                &hash[12..16],
                &hash[16..20],
                &hash[20..32]
            ))?;
            let video = RemoteVideo {
                id: id.clone(),
                library: self.binding.library,
                title: marker.into(),
                description: String::new(),
                processing: Processing::AwaitingUpload,
                length_seconds: 0,
                mp4_720p: false,
            };
            let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
            if map.len() >= 10_000 {
                return Err(Error::Capacity);
            }
            map.entry(id).or_insert_with(|| video.clone());
            return Ok(video);
        }
        let response: wire::Video = self
            .request(
                Method::POST,
                &self.path(None),
                Some(serde_json::json!({"title":marker})),
            )
            .await?;
        let video = response.checked(self.binding.library)?;
        if video.title != marker || video.processing != Processing::AwaitingUpload {
            return Err(Error::Protocol);
        }
        Ok(video)
    }

    async fn find_created(&self, marker: &str) -> Result<Option<RemoteVideo>, Error> {
        if !marker_valid(marker) {
            return Err(Error::InvalidInput);
        }
        if self.binding.mode == ProviderMode::Offline {
            return Ok(self
                .offline
                .lock()
                .map_err(|_| Error::Storage)?
                .values()
                .find(|v| v.title == marker)
                .cloned());
        }
        let page: wire::Page = self
            .read(&format!(
                "{}?page=1&itemsPerPage=100&search={marker}",
                self.path(None)
            ))
            .await?;
        if page.current_page != 1
            || page.items_per_page != 100
            || page.total_items > 100
            || page.items.len() > 100
            || page.total_items as usize != page.items.len()
        {
            return Err(Error::Protocol);
        }
        let mut found = None;
        for video in page.items {
            let video = video.checked(self.binding.library)?;
            if video.title == marker {
                if found.is_some() {
                    return Err(Error::Conflict);
                }
                found = Some(video);
            }
        }
        Ok(found)
    }

    async fn get(&self, video: &VideoId) -> Result<Option<RemoteVideo>, Error> {
        if self.binding.mode == ProviderMode::Offline {
            return Ok(self
                .offline
                .lock()
                .map_err(|_| Error::Storage)?
                .get(video)
                .cloned());
        }
        let wire: wire::Video = match self.read(&self.path(Some(video))).await {
            Err(Error::NotFound) => return Ok(None),
            value => value?,
        };
        let result = wire.checked(self.binding.library)?;
        if &result.id != video {
            return Err(Error::Protocol);
        }
        Ok(Some(result))
    }

    async fn update(&self, video: &VideoId, metadata: &Metadata) -> Result<(), Error> {
        if self.binding.mode == ProviderMode::Offline {
            let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
            let video = map.get_mut(video).ok_or(Error::NotFound)?;
            video.title = metadata.title().into();
            video.description = metadata.description().into();
            return Ok(());
        }
        // Bunny replaces the complete supplied tag list. Preserve unrelated
        // tags from a bounded current read; callers still own single-writer
        // coordination with out-of-band tools (the API has no compare-and-swap).
        let mut wire: wire::Video = self.read(&self.path(Some(video))).await?;
        let tags = wire.update_tags(metadata.description())?;
        if &wire.checked(self.binding.library)?.id != video {
            return Err(Error::Protocol);
        }
        let status: wire::Status = self
            .request(
                Method::POST,
                &self.path(Some(video)),
                Some(serde_json::json!({
                    "title":metadata.title(), "metaTags":tags
                })),
            )
            .await?;
        if !status.success || status.status_code != 200 {
            return Err(Error::Protocol);
        }
        Ok(())
    }

    async fn delete(&self, video: &VideoId) -> Result<(), Error> {
        if self.binding.mode == ProviderMode::Offline {
            self.offline
                .lock()
                .map_err(|_| Error::Storage)?
                .remove(video);
            return Ok(());
        }
        let status: wire::Status = match self
            .request(Method::DELETE, &self.path(Some(video)), None)
            .await
        {
            Err(Error::NotFound) => return Ok(()),
            value => value?,
        };
        if !status.success || status.status_code != 200 {
            return Err(Error::Protocol);
        }
        Ok(())
    }

    fn upload(&self, video: &VideoId, now: i64, ttl: u32) -> Result<UploadGrant, Error> {
        self.upload_grant(video, now, ttl)
    }
    fn playback(
        &self,
        video: &VideoId,
        now: i64,
        ttl: u32,
        kind: PlaybackKind,
    ) -> Result<PlaybackGrant, Error> {
        self.playback_grant(video, now, ttl, kind)
    }
}
