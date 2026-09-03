//! HMAC-authenticated, bounded, single-writer security-event spool.

use crate::telemetry::LiveSecurityEvent;
use std::{
    collections::BTreeMap,
    fmt,
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use super::{MAX_SIEM_SPOOL_BYTES, MAX_SIEM_SPOOL_RECORDS};

mod codec;
use codec::{DecodedSpool, SPOOL_MAGIC, encode_frame, read_and_verify};

/// Maximum number of active plus historical verification keys accepted.
pub const MAX_SIEM_INTEGRITY_KEYS: usize = 8;
const MIN_SIEM_SPOOL_BYTES: u64 = 512;
const MIN_SIEM_KEY_BYTES: usize = 32;
const MAX_SIEM_KEY_BYTES: usize = 64;

/// Typed failures that never include event bodies, paths, or key material.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum AuthenticatedSiemSpoolError {
    #[error("authenticated SIEM spool capacity must be between 512 bytes and 16 MiB")]
    InvalidCapacity,
    #[error("authenticated SIEM spool path cannot be empty")]
    InvalidPath,
    #[error("authenticated SIEM spool target must be a regular non-symlink file")]
    UnsafeFileType,
    #[error("SIEM integrity key identifier is invalid")]
    InvalidKeyId,
    #[error("SIEM integrity key must contain 32 to 64 diverse bytes")]
    InvalidKeyMaterial,
    #[error("SIEM integrity key identifiers must be unique")]
    DuplicateKeyId,
    #[error("SIEM integrity key ring exceeds the eight-key limit")]
    TooManyKeys,
    #[error("authenticated SIEM spool byte capacity is exhausted")]
    CapacityExceeded,
    #[error("authenticated SIEM spool record capacity is exhausted")]
    RecordCapacityExceeded,
    #[error("authenticated SIEM spool record exceeds the encoding limit")]
    RecordTooLarge,
    #[error("authenticated SIEM spool record {record} is corrupt: {reason}")]
    CorruptRecord { record: usize, reason: &'static str },
    #[error("authenticated SIEM spool record {record} references an unavailable key")]
    UnknownKey { record: usize },
    #[error("authenticated SIEM spool record {record} failed HMAC verification")]
    AuthenticationFailed { record: usize },
    #[error("authenticated SIEM spool changed outside the active writer")]
    ExternalModification,
    #[error("authenticated SIEM spool writer is unhealthy and must be reopened")]
    UnhealthyWriter,
    #[error("authenticated SIEM spool write completed but durability is uncertain")]
    DurabilityUncertain,
    #[error("authenticated SIEM spool partial-write recovery failed")]
    RecoveryFailed,
    #[error("authenticated SIEM spool lock is unavailable")]
    LockUnavailable,
    #[error("authenticated SIEM spool {operation} failed: {kind:?}")]
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    #[error("authenticated SIEM spool event encoding failed")]
    Encoding,
}

/// Named HMAC-SHA256 key whose material is zeroized and omitted from `Debug`.
pub struct SiemIntegrityKey {
    id: String,
    material: Zeroizing<Vec<u8>>,
}

impl SiemIntegrityKey {
    /// Consumes and validates one named key.
    ///
    /// Identifiers contain 1–32 ASCII alphanumeric, `.`, `_`, or `-` bytes.
    /// Material contains 32–64 bytes and is retained in zeroizing storage.
    pub fn try_new(
        id: impl Into<String>,
        material: impl Into<Vec<u8>>,
    ) -> Result<Self, AuthenticatedSiemSpoolError> {
        let id = id.into();
        let material = Zeroizing::new(material.into());
        if !valid_key_id(&id) {
            return Err(AuthenticatedSiemSpoolError::InvalidKeyId);
        }
        if !valid_key_material(&material) {
            return Err(AuthenticatedSiemSpoolError::InvalidKeyMaterial);
        }
        Ok(Self { id, material })
    }

    /// Stable non-secret identifier stored in authenticated frame headers.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Debug for SiemIntegrityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SiemIntegrityKey")
            .field("id", &self.id)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

/// Bounded rotation ring. New records use `active`; historical keys only verify.
pub struct SiemKeyRing {
    active: String,
    keys: BTreeMap<String, Zeroizing<Vec<u8>>>,
}

impl SiemKeyRing {
    /// Builds a ring with one write key and at most seven historical read keys.
    pub fn try_new(
        active: SiemIntegrityKey,
        historical: impl IntoIterator<Item = SiemIntegrityKey>,
    ) -> Result<Self, AuthenticatedSiemSpoolError> {
        let active_id = active.id.clone();
        let mut keys = BTreeMap::new();
        keys.insert(active.id, active.material);
        for key in historical {
            if keys.len() >= MAX_SIEM_INTEGRITY_KEYS {
                return Err(AuthenticatedSiemSpoolError::TooManyKeys);
            }
            if keys.insert(key.id, key.material).is_some() {
                return Err(AuthenticatedSiemSpoolError::DuplicateKeyId);
            }
        }
        Ok(Self {
            active: active_id,
            keys,
        })
    }

    fn active(&self) -> Option<(&str, &[u8])> {
        self.keys
            .get(&self.active)
            .map(|material| (self.active.as_str(), material.as_slice()))
    }

    fn get(&self, id: &str) -> Option<&[u8]> {
        self.keys.get(id).map(|value| value.as_slice())
    }
}

impl fmt::Debug for SiemKeyRing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SiemKeyRing")
            .field("active", &self.active)
            .field("key_count", &self.keys.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedSiemSpoolReceipt {
    sequence: u64,
    end_offset: u64,
}

impl AuthenticatedSiemSpoolReceipt {
    /// One-based authenticated record sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// File offset after the synchronized record.
    #[must_use]
    pub const fn end_offset(self) -> u64 {
        self.end_offset
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedSiemSpoolSnapshot {
    records: usize,
    bytes: u64,
    max_bytes: u64,
}

impl AuthenticatedSiemSpoolSnapshot {
    /// Number of authenticated records in the active file.
    #[must_use]
    pub const fn records(self) -> usize {
        self.records
    }

    /// Current file size including its header.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// Configured file-size ceiling.
    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

struct AuthenticatedSpoolState {
    file: File,
    bytes: u64,
    records: usize,
    last_tag: [u8; 32],
    healthy: bool,
}

/// Authenticated local journal for normalized [`LiveSecurityEvent`] values.
///
/// HMAC chaining detects forged, reordered, or removed interior records. A
/// trusted external checkpoint is still required to detect removal of a whole
/// valid tail. The caller owns key custody, path permissions, single-writer
/// operation, retention, delivery, acknowledgement, and backup.
pub struct AuthenticatedSiemSpool {
    state: Mutex<AuthenticatedSpoolState>,
    keys: SiemKeyRing,
    max_bytes: u64,
}

impl AuthenticatedSiemSpool {
    /// Opens or creates an authenticated spool with the 16 MiB crate ceiling.
    pub fn try_open(
        path: impl Into<PathBuf>,
        keys: SiemKeyRing,
    ) -> Result<Self, AuthenticatedSiemSpoolError> {
        Self::try_open_with_max_bytes(path, keys, MAX_SIEM_SPOOL_BYTES)
    }

    /// Opens or creates an authenticated spool with a smaller explicit quota.
    pub fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        keys: SiemKeyRing,
        max_bytes: u64,
    ) -> Result<Self, AuthenticatedSiemSpoolError> {
        if !(MIN_SIEM_SPOOL_BYTES..=MAX_SIEM_SPOOL_BYTES).contains(&max_bytes) {
            return Err(AuthenticatedSiemSpoolError::InvalidCapacity);
        }
        let path = path.into();
        validate_target(&path)?;
        let mut file = open_spool_file(&path)?;
        let metadata = file
            .metadata()
            .map_err(|error| io_failure("metadata", &error))?;
        if !metadata.is_file() {
            return Err(AuthenticatedSiemSpoolError::UnsafeFileType);
        }
        if metadata.len() > max_bytes {
            return Err(AuthenticatedSiemSpoolError::CapacityExceeded);
        }

        let decoded = if metadata.len() == 0 {
            file.write_all(SPOOL_MAGIC)
                .map_err(|error| io_failure("initialize", &error))?;
            file.sync_data()
                .map_err(|error| io_failure("initialize sync", &error))?;
            DecodedSpool::empty()
        } else {
            read_and_verify(&mut file, max_bytes, &keys)?
        };

        Ok(Self {
            state: Mutex::new(AuthenticatedSpoolState {
                file,
                bytes: decoded.bytes,
                records: decoded.events.len(),
                last_tag: decoded.last_tag,
                healthy: true,
            }),
            keys,
            max_bytes,
        })
    }

    /// Normalizes and durably appends one local event using the active key.
    pub fn append_local(
        &self,
        mut event: LiveSecurityEvent,
    ) -> Result<AuthenticatedSiemSpoolReceipt, AuthenticatedSiemSpoolError> {
        event.verified_hmac = false;
        let event = event.normalized();
        let payload =
            serde_json::to_vec(&event).map_err(|_| AuthenticatedSiemSpoolError::Encoding)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AuthenticatedSiemSpoolError::LockUnavailable)?;
        ensure_unchanged(&state)?;
        if !state.healthy {
            return Err(AuthenticatedSiemSpoolError::UnhealthyWriter);
        }
        if state.records >= MAX_SIEM_SPOOL_RECORDS {
            return Err(AuthenticatedSiemSpoolError::RecordCapacityExceeded);
        }
        let sequence = u64::try_from(state.records)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(AuthenticatedSiemSpoolError::RecordCapacityExceeded)?;
        let (key_id, key) = self
            .keys
            .active()
            .ok_or(AuthenticatedSiemSpoolError::InvalidKeyMaterial)?;
        let (frame, tag) = encode_frame(sequence, key_id, key, state.last_tag, &payload)?;
        let final_bytes = state
            .bytes
            .checked_add(frame.len() as u64)
            .ok_or(AuthenticatedSiemSpoolError::CapacityExceeded)?;
        if final_bytes > self.max_bytes {
            return Err(AuthenticatedSiemSpoolError::CapacityExceeded);
        }

        let previous_bytes = state.bytes;
        if let Err(write_error) = state.file.write_all(&frame) {
            return recover_partial_write(&mut state, previous_bytes, &write_error);
        }
        state.bytes = final_bytes;
        state.records = state.records.saturating_add(1);
        state.last_tag = tag;
        if state.file.sync_data().is_err() {
            state.healthy = false;
            return Err(AuthenticatedSiemSpoolError::DurabilityUncertain);
        }
        Ok(AuthenticatedSiemSpoolReceipt {
            sequence,
            end_offset: final_bytes,
        })
    }

    /// Re-reads every frame, validates its HMAC chain, and marks results verified.
    pub fn read_verified(&self) -> Result<Vec<LiveSecurityEvent>, AuthenticatedSiemSpoolError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AuthenticatedSiemSpoolError::LockUnavailable)?;
        if !state.healthy {
            return Err(AuthenticatedSiemSpoolError::UnhealthyWriter);
        }
        ensure_unchanged(&state)?;
        let decoded = read_and_verify(&mut state.file, self.max_bytes, &self.keys)?;
        state.bytes = decoded.bytes;
        state.records = decoded.events.len();
        state.last_tag = decoded.last_tag;
        Ok(decoded.events)
    }

    /// Returns bounded metadata after checking for external file growth/shrinkage.
    pub fn snapshot(&self) -> Result<AuthenticatedSiemSpoolSnapshot, AuthenticatedSiemSpoolError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AuthenticatedSiemSpoolError::LockUnavailable)?;
        if !state.healthy {
            return Err(AuthenticatedSiemSpoolError::UnhealthyWriter);
        }
        ensure_unchanged(&state)?;
        Ok(AuthenticatedSiemSpoolSnapshot {
            records: state.records,
            bytes: state.bytes,
            max_bytes: self.max_bytes,
        })
    }
}

fn open_spool_file(path: &Path) -> Result<File, AuthenticatedSiemSpoolError> {
    let mut options = OpenOptions::new();
    options.read(true).append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        // Close the validate/open race for the final path component and make a
        // newly created journal owner-only. Existing permissions are preserved.
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| io_failure("open", &error))
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_key_material(value: &[u8]) -> bool {
    if !(MIN_SIEM_KEY_BYTES..=MAX_SIEM_KEY_BYTES).contains(&value.len()) {
        return false;
    }
    let mut observed = [false; 256];
    for byte in value {
        observed[usize::from(*byte)] = true;
    }
    observed.into_iter().filter(|seen| *seen).count() >= 8
}

fn validate_target(path: &Path) -> Result<(), AuthenticatedSiemSpoolError> {
    if path.as_os_str().is_empty() {
        return Err(AuthenticatedSiemSpoolError::InvalidPath);
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(AuthenticatedSiemSpoolError::UnsafeFileType)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure("metadata", &error)),
    }
}

fn ensure_unchanged(state: &AuthenticatedSpoolState) -> Result<(), AuthenticatedSiemSpoolError> {
    let length = state
        .file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length.ct_eq(&state.bytes).unwrap_u8() != 1 {
        return Err(AuthenticatedSiemSpoolError::ExternalModification);
    }
    Ok(())
}

fn recover_partial_write(
    state: &mut AuthenticatedSpoolState,
    previous_bytes: u64,
    write_error: &io::Error,
) -> Result<AuthenticatedSiemSpoolReceipt, AuthenticatedSiemSpoolError> {
    if state.file.set_len(previous_bytes).is_err() || state.file.sync_data().is_err() {
        state.healthy = false;
        return Err(AuthenticatedSiemSpoolError::RecoveryFailed);
    }
    Err(io_failure("append", write_error))
}

fn io_failure(operation: &'static str, error: &io::Error) -> AuthenticatedSiemSpoolError {
    AuthenticatedSiemSpoolError::Io {
        operation,
        kind: error.kind(),
    }
}

#[cfg(test)]
#[path = "authenticated/tests.rs"]
mod tests;
