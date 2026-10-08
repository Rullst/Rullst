use super::{
    S3Config,
    config::endpoint_loopback,
    signing::{authorize, object_url},
};
use crate::{
    MediaError as Error, Processing, ProviderBinding, ProviderMode, Reference, RemoteVideo, VideoId,
};
use reqwest::{
    Client, Method, Response,
    header::{CONTENT_LENGTH, CONTENT_TYPE, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Mutex, time::Duration};

pub(super) const ORIGINAL: &str = "original";
const METADATA: &str = "metadata.json";
const METADATA_LIMIT: usize = 16 * 1024;

/// One uploaded object as observed by `HEAD`: length and media type.
pub(super) type Stored = (u64, String);

pub(super) struct OfflineObject {
    pub(super) title: String,
    pub(super) description: String,
    pub(super) stored: Option<Stored>,
}

/// Metadata sidecar written beside the original; never sent to browsers.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sidecar {
    pub(super) version: u8,
    pub(super) video: VideoId,
    pub(super) title: String,
    pub(super) description: String,
}

/// S3-compatible private object storage: AWS S3, Cloudflare R2 or MinIO.
/// It stores and serves the uploaded original; there is no transcoding.
pub struct S3Storage {
    pub(super) config: S3Config,
    pub(super) binding: ProviderBinding,
    client: Client,
    pub(super) offline: Mutex<BTreeMap<VideoId, OfflineObject>>,
}
impl std::fmt::Debug for S3Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Storage")
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}
impl S3Storage {
    /// Empty or `mock_*` credentials select the offline mock; `fixture_*`
    /// credentials are rejected here (see [`S3Storage::for_protocol_tests`]).
    pub fn new(config: S3Config) -> Result<Self, Error> {
        if config.credentials.mode == ProviderMode::ProtocolFixture {
            return Err(Error::Configuration);
        }
        Self::build(config)
    }

    /// Explicit local protocol fixture: `fixture_*` credentials and a literal
    /// loopback endpoint. Grants keep `ProtocolFixture`; production rejects it.
    pub fn for_protocol_tests(config: S3Config) -> Result<Self, Error> {
        if config.credentials.mode != ProviderMode::ProtocolFixture
            || !endpoint_loopback(&config.endpoint)
        {
            return Err(Error::Configuration);
        }
        Self::build(config)
    }

    fn build(config: S3Config) -> Result<Self, Error> {
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent("rullst-media/13")
            .build()
            .map_err(|_| Error::Configuration)?;
        let fingerprint = sha256_hex(&[
            "s3|",
            config.endpoint.as_str(),
            "|",
            &config.bucket,
            "|",
            &config.region,
            "|",
            &config.prefix,
            "|",
            if config.path_style { "path" } else { "virtual" },
        ]);
        let binding = ProviderBinding {
            library: config.library,
            mode: config.credentials.mode,
            environment: Reference::new(fingerprint)?,
        };
        Ok(Self {
            config,
            binding,
            client,
            offline: Mutex::new(BTreeMap::new()),
        })
    }

    /// Explicit offline fixture control: records an uploaded object as `HEAD`
    /// would report it. Never available against a remote bucket.
    pub fn simulate_upload(
        &self,
        video: &VideoId,
        content_type: impl Into<String>,
        length: u64,
    ) -> Result<(), Error> {
        if self.binding.mode != ProviderMode::Offline {
            return Err(Error::Unsupported);
        }
        let mut map = self.offline.lock().map_err(|_| Error::Storage)?;
        let found = map.get_mut(video).ok_or(Error::NotFound)?;
        found.stored = Some((length, content_type.into()));
        Ok(())
    }

    pub(super) fn key(&self, video: &VideoId, leaf: &str) -> String {
        format!("{}{}/{leaf}", self.config.prefix, video.as_str())
    }

    /// Ready only for a non-empty object within the size limit whose stored
    /// type is accepted. Anything else is `Failed`; a missing object awaits upload.
    pub(super) fn processing(&self, stored: Option<&Stored>) -> Processing {
        match stored {
            None => Processing::AwaitingUpload,
            Some((length, content_type))
                if *length > 0
                    && *length <= self.config.max_object_bytes
                    && self.config.content_types.contains(content_type) =>
            {
                Processing::Ready
            }
            Some(_) => Processing::Failed,
        }
    }

    pub(super) fn remote(&self, sidecar: Sidecar, processing: Processing) -> RemoteVideo {
        RemoteVideo {
            id: sidecar.video,
            library: self.binding.library,
            title: sidecar.title,
            description: sidecar.description,
            processing,
            length_seconds: 0,
            mp4_720p: false,
        }
    }

    async fn request(
        &self,
        method: Method,
        key: &str,
        body: Option<Vec<u8>>,
    ) -> Result<Response, Error> {
        let url = object_url(&self.config, key)?;
        let body = body.unwrap_or_default();
        let digest = ring::digest::digest(&ring::digest::SHA256, &body);
        let signed: &[(&str, &str)] = if method == Method::PUT {
            &[("content-type", "application/json")]
        } else {
            &[]
        };
        let authorization = authorize(
            &self.config,
            method.as_str(),
            &url,
            signed,
            &hex(digest.as_ref()),
            std::time::SystemTime::now(),
        )?;
        let mut request = self.client.request(method.clone(), url);
        for (name, value) in signed {
            request = request.header(*name, *value);
        }
        for (name, value) in authorization {
            let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| Error::Configuration)?;
            let mut value = HeaderValue::from_str(&value).map_err(|_| Error::Configuration)?;
            value.set_sensitive(true);
            request = request.header(name, value);
        }
        if method == Method::PUT {
            request = request.body(body);
        }
        let response = request.send().await.map_err(|_| Error::Unavailable)?;
        match response.status().as_u16() {
            200 | 204 => Ok(response),
            404 => Err(Error::NotFound),
            429 | 500..=599 => Err(Error::Unavailable),
            400..=499 => Err(Error::Rejected),
            _ => Err(Error::Protocol),
        }
    }

    /// The uploaded original as `HEAD` reports it, or `None` when absent.
    pub(super) async fn head(&self, video: &VideoId) -> Result<Option<Stored>, Error> {
        let response = match self
            .request(Method::HEAD, &self.key(video, ORIGINAL), None)
            .await
        {
            Err(Error::NotFound) => return Ok(None),
            result => result?,
        };
        let header = |name| {
            response
                .headers()
                .get(name)
                .and_then(|value: &HeaderValue| value.to_str().ok())
        };
        let length = header(CONTENT_LENGTH)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(Error::Protocol)?;
        let content_type = header(CONTENT_TYPE)
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        Ok(Some((length, content_type)))
    }

    pub(super) async fn read_sidecar(&self, video: &VideoId) -> Result<Option<Sidecar>, Error> {
        let mut response = match self
            .request(Method::GET, &self.key(video, METADATA), None)
            .await
        {
            Err(Error::NotFound) => return Ok(None),
            result => result?,
        };
        if response
            .content_length()
            .is_some_and(|length| length > METADATA_LIMIT as u64)
        {
            return Err(Error::Protocol);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Unavailable)? {
            if chunk.len() > METADATA_LIMIT.saturating_sub(bytes.len()) {
                return Err(Error::Protocol);
            }
            bytes.extend_from_slice(&chunk);
        }
        let sidecar: Sidecar = serde_json::from_slice(&bytes).map_err(|_| Error::Protocol)?;
        if sidecar.version != 1 || &sidecar.video != video {
            return Err(Error::Protocol);
        }
        Ok(Some(sidecar))
    }

    pub(super) async fn write_sidecar(&self, sidecar: &Sidecar) -> Result<(), Error> {
        let body = serde_json::to_vec(sidecar).map_err(|_| Error::Configuration)?;
        self.request(Method::PUT, &self.key(&sidecar.video, METADATA), Some(body))
            .await
            .map(drop)
    }

    /// Deletes the original, then the sidecar. A missing object is not an error.
    pub(super) async fn delete_objects(&self, video: &VideoId) -> Result<(), Error> {
        for leaf in [ORIGINAL, METADATA] {
            match self
                .request(Method::DELETE, &self.key(video, leaf), None)
                .await
            {
                Ok(_) | Err(Error::NotFound) => (),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

pub(super) fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        value.push(char::from(DIGITS[usize::from(b >> 4)]));
        value.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    value
}

pub(super) fn sha256_hex(parts: &[&str]) -> String {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    for part in parts {
        hash.update(part.as_bytes());
    }
    hex(hash.finish().as_ref())
}
