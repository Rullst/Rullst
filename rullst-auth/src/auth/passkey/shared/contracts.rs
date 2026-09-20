//! Bounded trusted-store contracts; constructing references does not authenticate them.
use std::{
    fmt,
    future::Future,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) const MAX_TIME: i64 = 253_402_300_799;
pub(super) const MAX_CREDENTIALS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PasskeyCeremonyError {
    #[error("invalid passkey ceremony input")]
    InvalidInput,
    #[error("invalid passkey ceremony store configuration")]
    Configuration,
    #[error("passkey ceremony rejected")]
    Rejected,
    #[error("passkey ceremony capacity exhausted")]
    Capacity,
    #[error("passkey ceremony already exists")]
    Conflict,
    #[error("passkey ceremony expired")]
    Expired,
    #[error("passkey ceremony storage unavailable")]
    Unavailable,
    #[error("passkey ceremony storage is inconsistent")]
    Corrupt,
    #[error("passkey ceremony commit outcome is uncertain")]
    UncertainCommit,
}
use PasskeyCeremonyError as Error;

/// Trusted server time, never a timestamp supplied by a browser.
pub trait CeremonyClock: Clone + Send + Sync + 'static {
    fn now(&self) -> Result<i64, Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct SystemCeremonyClock;
impl CeremonyClock for SystemCeremonyClock {
    fn now(&self) -> Result<i64, Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Unavailable)?;
        i64::try_from(now.as_secs())
            .ok()
            .filter(|v| *v <= MAX_TIME)
            .ok_or(Error::Unavailable)
    }
}

/// Immutable configuration retained independently of a restored database.
#[derive(Clone, Eq, PartialEq)]
pub struct CeremonyStoreConfig {
    epoch: String,
    capacity: u32,
    lifetime_seconds: u32,
}
impl CeremonyStoreConfig {
    pub fn new(
        epoch: impl Into<String>,
        capacity: u32,
        lifetime_seconds: u32,
    ) -> Result<Self, Error> {
        let epoch = epoch.into();
        opaque(&epoch)?;
        if !(1..=100_000).contains(&capacity) || !(1..=600).contains(&lifetime_seconds) {
            return Err(Error::Configuration);
        }
        Ok(Self {
            epoch,
            capacity,
            lifetime_seconds,
        })
    }
    pub fn epoch(&self) -> &str {
        &self.epoch
    }
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }
    pub const fn lifetime_seconds(&self) -> u32 {
        self.lifetime_seconds
    }
}
impl fmt::Debug for CeremonyStoreConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CeremonyStoreConfig")
            .field("epoch", &"[redacted]")
            .field("capacity", &self.capacity)
            .field("lifetime_seconds", &self.lifetime_seconds)
            .finish()
    }
}

/// Host-resolved account and session binding; these references are not identity proofs.
#[derive(Clone)]
pub struct PasskeyBinding {
    pub(super) tenant: String,
    pub(super) subject: String,
    pub(super) session: String,
    pub(super) user_handle: Vec<u8>,
}
impl PasskeyBinding {
    pub fn new(
        tenant: impl Into<String>,
        subject: impl Into<String>,
        session: impl Into<String>,
        user_handle: impl Into<Vec<u8>>,
    ) -> Result<Self, Error> {
        let value = Self {
            tenant: tenant.into(),
            subject: subject.into(),
            session: session.into(),
            user_handle: user_handle.into(),
        };
        for reference in [&value.tenant, &value.subject, &value.session] {
            opaque(reference)?;
        }
        if value.user_handle.is_empty()
            || value.user_handle.len() > 64
            || value.user_handle.iter().all(|byte| *byte == 0)
        {
            return Err(Error::InvalidInput);
        }
        Ok(value)
    }
    pub fn tenant(&self) -> &str {
        &self.tenant
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
}
impl fmt::Debug for PasskeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasskeyBinding([redacted])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CeremonyKind {
    Registration,
    Authentication,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CeremonyDurability {
    ProcessLocal,
    SharedDurable,
}

/// Digest-only state. Custom stores must retain exactly this intent until consumption.
#[derive(Clone, Eq, PartialEq)]
pub struct CeremonyIntent {
    challenge: [u8; 32],
    binding: [u8; 32],
    kind: CeremonyKind,
    credentials: Vec<[u8; 32]>,
}
impl CeremonyIntent {
    pub fn new(
        challenge: [u8; 32],
        binding: [u8; 32],
        kind: CeremonyKind,
        credentials: Vec<[u8; 32]>,
    ) -> Result<Self, Error> {
        if credentials.len() > MAX_CREDENTIALS
            || (kind == CeremonyKind::Registration && !credentials.is_empty())
            || (kind == CeremonyKind::Authentication && credentials.is_empty())
            || credentials
                .iter()
                .enumerate()
                .any(|(index, value)| credentials[..index].contains(value))
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            challenge,
            binding,
            kind,
            credentials,
        })
    }
    pub const fn challenge(&self) -> &[u8; 32] {
        &self.challenge
    }
    pub const fn binding(&self) -> &[u8; 32] {
        &self.binding
    }
    pub const fn kind(&self) -> CeremonyKind {
        self.kind
    }
    pub fn credentials(&self) -> &[[u8; 32]] {
        &self.credentials
    }
}
impl fmt::Debug for CeremonyIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CeremonyIntent")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// A committed one-time consumption. No Clone/serialization or public proof acceptance.
pub struct ConsumedCeremony {
    intent: CeremonyIntent,
    issued_at: i64,
    expires_at: i64,
}
impl ConsumedCeremony {
    /// For trusted store implementers decoding validated durable state.
    pub fn from_stored(
        intent: CeremonyIntent,
        issued_at: i64,
        expires_at: i64,
    ) -> Result<Self, Error> {
        if issued_at < 0
            || expires_at <= issued_at
            || expires_at > MAX_TIME
            || expires_at - issued_at > 600
        {
            return Err(Error::Corrupt);
        }
        Ok(Self {
            intent,
            issued_at,
            expires_at,
        })
    }
    pub const fn intent(&self) -> &CeremonyIntent {
        &self.intent
    }
    pub const fn issued_at(&self) -> i64 {
        self.issued_at
    }
    pub const fn expires_at(&self) -> i64 {
        self.expires_at
    }
}
impl fmt::Debug for ConsumedCeremony {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConsumedCeremony([redacted])")
    }
}

/// Trusted adapter contract. Single-use consumption and quota checks must be atomic
/// across all participating hosts; outages must never fall back to local memory.
pub trait PasskeyCeremonyStore: Send + Sync {
    fn config(&self) -> &CeremonyStoreConfig;
    fn durability(&self) -> CeremonyDurability;
    fn issue(&self, intent: &CeremonyIntent) -> impl Future<Output = Result<(), Error>> + Send;
    fn consume(
        &self,
        challenge: [u8; 32],
        binding: [u8; 32],
        kind: CeremonyKind,
    ) -> impl Future<Output = Result<ConsumedCeremony, Error>> + Send;
    /// Recheck persisted configuration, trusted time and expiry after cryptographic work.
    fn confirm(
        &self,
        consumed: &ConsumedCeremony,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

fn opaque(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
