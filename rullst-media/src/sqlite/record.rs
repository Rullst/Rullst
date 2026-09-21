use crate::{MediaError as Error, Metadata, Processing, Reference, Scope, VideoId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lifecycle {
    Creating,
    Active,
    Deleting,
    Deleted,
}

/// Local ownership and desired metadata; pending work is explicit. Provider IDs
/// are displayed only to authorized callers and never accepted as ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: Reference,
    pub scope: Scope,
    pub owner: Reference,
    pub metadata: Metadata,
    pub video: Option<VideoId>,
    pub lifecycle: Lifecycle,
    pub processing: Processing,
    pub published: bool,
    pub pending: bool,
    pub revision: i64,
    pub updated_at: i64,
    pub length_seconds: u32,
    pub mp4_720p: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    Create,
    Update,
    Refresh,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Pending {
    pub kind: Kind,
    pub dispatched: bool,
    pub nonce: Option<String>,
    pub until: i64,
    pub revision: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub asset: Asset,
    pub marker: String,
    pub create_digest: String,
    pub pending: Option<Pending>,
    pub notifications: Vec<String>,
    pub last_notification: i64,
}
impl Record {
    pub fn validate(&self) -> Result<(), Error> {
        let a = &self.asset;
        if a.revision <= 0
            || crate::contracts::checked_time(a.updated_at).is_err()
            || a.length_seconds > 604_800
            || self.notifications.len() > 32
            || self.notifications.iter().any(|s| !digest(s))
            || self.last_notification < 0
            || self.last_notification > a.updated_at
            || !digest(&self.create_digest)
            || self.marker.len() != 45
            || !self.marker.starts_with("rullst-video-")
            || !self.marker[13..]
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || a.pending != self.pending.is_some()
            || (a.lifecycle == Lifecycle::Creating) != a.video.is_none()
            || (matches!(a.lifecycle, Lifecycle::Creating | Lifecycle::Deleting)
                && self.pending.is_none())
            || (a.lifecycle == Lifecycle::Deleted
                && (a.published || a.pending || a.processing != Processing::Missing))
            || (a.published
                && (a.lifecycle != Lifecycle::Active || a.processing != Processing::Ready))
        {
            return Err(Error::Configuration);
        }
        if let Some(p) = &self.pending
            && (p.revision < 0
                || p.revision > a.revision
                || p.until < 0
                || (p.nonce.is_some() && (p.revision == 0 || p.until == 0))
                || p.nonce.as_ref().is_some_and(|n| {
                    n.len() != 32
                        || !n
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                })
                || (p.kind == Kind::Create) != (a.lifecycle == Lifecycle::Creating)
                || (p.kind == Kind::Delete) != (a.lifecycle == Lifecycle::Deleting))
        {
            return Err(Error::Configuration);
        }
        Ok(())
    }
    pub fn next(&mut self, now: i64) -> Result<(), Error> {
        self.asset.revision = self.asset.revision.checked_add(1).ok_or(Error::Capacity)?;
        self.asset.updated_at = now;
        self.asset.pending = self.pending.is_some();
        Ok(())
    }
    pub fn plan(&mut self, kind: Kind) {
        self.pending = Some(Pending {
            kind,
            dispatched: false,
            nonce: None,
            until: 0,
            revision: 0,
        });
        self.asset.pending = true;
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(super) fn random_hex() -> Result<String, Error> {
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 16];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(super) fn create_digest(metadata: &Metadata) -> Result<String, Error> {
    let bytes = serde_json::to_vec(metadata).map_err(|_| Error::InvalidInput)?;
    Ok(ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
