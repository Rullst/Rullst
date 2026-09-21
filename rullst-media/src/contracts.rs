use crate::MediaError as Error;
use serde::{Deserialize, Serialize};
use std::future::Future;

/// Opaque application identifier. Contains no path/URL delimiters or whitespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Reference(String);

impl Reference {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 96
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Reference {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Error> {
        Self::new(value)
    }
}
impl From<Reference> for String {
    fn from(value: Reference) -> Self {
        value.0
    }
}

/// Canonical lower-case provider GUID, never an authorization assertion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VideoId(String);
impl VideoId {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.len() != 36
            || !value.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                }
            })
            || value.bytes().all(|b| b == b'0' || b == b'-')
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for VideoId {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Error> {
        Self::new(value)
    }
}
impl From<VideoId> for String {
    fn from(value: VideoId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct LibraryId(i64);
impl LibraryId {
    pub fn new(value: i64) -> Result<Self, Error> {
        if value <= 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn value(self) -> i64 {
        self.0
    }
}
impl TryFrom<i64> for LibraryId {
    type Error = Error;
    fn try_from(v: i64) -> Result<Self, Error> {
        Self::new(v)
    }
}
impl From<LibraryId> for i64 {
    fn from(v: LibraryId) -> Self {
        v.0
    }
}

/// Construct from server-authenticated context, never directly from form claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub tenant: Reference,
    pub course: Reference,
}
impl Scope {
    pub fn new(tenant: impl Into<String>, course: impl Into<String>) -> Result<Self, Error> {
        Ok(Self {
            tenant: Reference::new(tenant)?,
            course: Reference::new(course)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Manage,
    Play,
}

/// Upper validity bound supplied by the authoritative application policy.
#[derive(Debug, Clone, Copy)]
pub struct Permission {
    until: i64,
}
impl Permission {
    pub fn until(expires_at: i64) -> Result<Self, Error> {
        Ok(Self {
            until: checked_time(expires_at)?,
        })
    }
    pub fn expires_at(self) -> i64 {
        self.until
    }
}

/// Refresh authenticated membership/role/entitlement from the authoritative host.
/// Implementations must return Denied on revoked/expired or cross-scope access.
pub trait Authorization: Send + Sync {
    fn check(
        &self,
        actor: &Reference,
        scope: &Scope,
        action: Action,
    ) -> impl Future<Output = Result<Permission, Error>> + Send;
}

/// Application-owned trusted UTC seconds. No caller/browser timestamp authority.
pub trait Clock: Send + Sync {
    fn now(&self) -> Result<i64, Error>;
}
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Result<i64, Error> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Clock)?;
        checked_time(i64::try_from(time.as_secs()).map_err(|_| Error::Clock)?)
    }
}
pub(crate) fn checked_time(time: i64) -> Result<i64, Error> {
    if !(1..=253_402_300_799).contains(&time) {
        return Err(Error::Clock);
    }
    Ok(time)
}

/// Plain-text metadata; no generated HTML, secret fields or arbitrary tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MetadataWire", into = "MetadataWire")]
pub struct Metadata {
    title: String,
    description: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataWire {
    title: String,
    description: String,
}
impl Metadata {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Result<Self, Error> {
        let title = title.into();
        let description = description.into();
        if title.trim().is_empty()
            || title.len() > 200
            || description.len() > 4096
            || title
                .chars()
                .any(|c| c.is_control() || c == '<' || c == '>')
            || description
                .chars()
                .any(|c| (c.is_control() && c != '\n') || c == '<' || c == '>')
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self { title, description })
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn description(&self) -> &str {
        &self.description
    }
}
impl TryFrom<MetadataWire> for Metadata {
    type Error = Error;
    fn try_from(v: MetadataWire) -> Result<Self, Error> {
        Self::new(v.title, v.description)
    }
}
impl From<Metadata> for MetadataWire {
    fn from(v: Metadata) -> Self {
        Self {
            title: v.title,
            description: v.description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderMode {
    Offline,
    ProtocolFixture,
    RemoteUnvalidated,
}

/// Identifies a configured provider environment, not a user's authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderBinding {
    pub library: LibraryId,
    pub mode: ProviderMode,
    /// Opaque server configuration identity; changing environment requires a new store.
    pub environment: Reference,
}

/// Only Finished from the authoritative API maps to Ready. Webhook codes differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Processing {
    AwaitingUpload,
    Processing,
    Ready,
    Failed,
    Missing,
}

/// Minimized provider response after identity/status/size validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteVideo {
    pub id: VideoId,
    pub library: LibraryId,
    pub title: String,
    pub description: String,
    pub processing: Processing,
    pub length_seconds: u32,
    pub mp4_720p: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackKind {
    Embed,
    Hls,
    Mp4_720p,
}

/// Explicitly exposed bearer data; its Debug representation is redacted.
#[derive(Clone, Serialize)]
pub struct PlaybackGrant {
    pub(crate) url: String,
    pub expires_at: i64,
    pub mode: ProviderMode,
}
impl PlaybackGrant {
    pub fn expose_url(&self) -> &str {
        &self.url
    }
}
impl std::fmt::Debug for PlaybackGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlaybackGrant")
            .field("url", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("mode", &self.mode)
            .finish()
    }
}

/// Expiring upload capability, not a proof of a byte cap or single use at Bunny.
#[derive(Clone, Serialize)]
pub struct UploadGrant {
    pub endpoint: String,
    pub library: LibraryId,
    pub video: VideoId,
    pub expires_at: i64,
    pub(crate) signature: String,
    pub mode: ProviderMode,
}
impl UploadGrant {
    pub fn expose_signature(&self) -> &str {
        &self.signature
    }
}
impl std::fmt::Debug for UploadGrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadGrant")
            .field("signature", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("mode", &self.mode)
            .finish()
    }
}

/// All headers must occur exactly once at ingress; duplicate header handling is
/// owned by the HTTP adapter, before constructing this borrowed value.
pub struct WebhookHeaders<'a> {
    pub version: &'a str,
    pub algorithm: &'a str,
    pub signature: &'a str,
}

/// Authenticated notification, never a publication/ownership or fresh-state grant.
#[derive(Debug, Clone)]
pub struct VerifiedNotification {
    pub(crate) library: LibraryId,
    pub(crate) video: VideoId,
    pub(crate) digest: String,
    pub(crate) mode: ProviderMode,
}
impl VerifiedNotification {
    pub fn video(&self) -> &VideoId {
        &self.video
    }
    pub fn library(&self) -> LibraryId {
        self.library
    }
    pub fn body_digest(&self) -> &str {
        &self.digest
    }
    pub fn mode(&self) -> ProviderMode {
        self.mode
    }
}
