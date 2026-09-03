use crate::fiscal::NfseEnvironment;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::{fmt, io};
use zeroize::Zeroizing;

/// Maximum supported size for one local NFS-e command journal (16 MiB).
pub const MAX_FISCAL_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of events retained in one local NFS-e command journal.
pub const MAX_FISCAL_JOURNAL_RECORDS: usize = 4_096;

/// Typed journal failures that never include paths, command IDs, XML or authority bodies.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum FiscalJournalError {
    #[error("fiscal journal key must contain a portable ID and exactly 32 bytes")]
    InvalidKey,
    #[error("fiscal journal capacity must be between 512 bytes and 16 MiB")]
    InvalidCapacity,
    #[error("fiscal journal path cannot be empty")]
    InvalidPath,
    #[error("fiscal journal target must be a regular non-symlink file")]
    UnsafeFileType,
    #[error("fiscal journal key does not match the authenticated file header")]
    KeyMismatch,
    #[error("fiscal command ID must be a portable opaque value of at most 128 bytes")]
    InvalidCommandId,
    #[error("offline mock commands cannot enter the official NFS-e command journal")]
    InvalidEnvironment,
    #[error("fiscal command request could not be encoded")]
    RequestEncoding,
    #[error("fiscal command response does not match its prepared request")]
    ResponseMismatch,
    #[error("fiscal command was not prepared in this journal")]
    MissingCommand,
    #[error("fiscal command conflicts with previously synchronized evidence")]
    CommandConflict,
    #[error("fiscal journal byte capacity is exhausted")]
    CapacityExceeded,
    #[error("fiscal journal record capacity is exhausted")]
    RecordCapacityExceeded,
    #[error("fiscal journal record exceeds the per-record encoding limit")]
    RecordTooLarge,
    #[error("fiscal journal record {record} is corrupt: {reason}")]
    CorruptRecord { record: usize, reason: &'static str },
    #[error("fiscal journal changed outside the active writer")]
    ExternalModification,
    #[error("fiscal journal writer is unhealthy and must be reopened")]
    UnhealthyWriter,
    #[error("fiscal journal write completed but durability could not be confirmed")]
    DurabilityUncertain,
    #[error("fiscal journal partial-write recovery failed")]
    RecoveryFailed,
    #[error("fiscal journal lock is unavailable")]
    LockUnavailable,
    #[error("fiscal journal clock is outside the supported Unix range")]
    ClockUnavailable,
    #[error("fiscal journal checkpoint does not match the current synchronized tip")]
    CheckpointMismatch,
    #[error("fiscal journal {operation} failed: {kind:?}")]
    Io {
        operation: &'static str,
        kind: io::ErrorKind,
    },
    #[error("fiscal journal event encoding failed")]
    Encoding,
}

/// A named 256-bit HMAC key used to authenticate a local fiscal journal.
pub struct FiscalJournalKey {
    key_id: String,
    bytes: Zeroizing<[u8; 32]>,
}

impl FiscalJournalKey {
    /// Constructs a journal key from a portable rotation ID and exactly 32 random bytes.
    pub fn try_new(
        key_id: impl Into<String>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, FiscalJournalError> {
        let key_id = key_id.into();
        if !valid_key_id(&key_id) {
            return Err(FiscalJournalError::InvalidKey);
        }
        let bytes: [u8; 32] = bytes
            .as_ref()
            .try_into()
            .map_err(|_| FiscalJournalError::InvalidKey)?;
        Ok(Self {
            key_id,
            bytes: Zeroizing::new(bytes),
        })
    }

    /// Returns the non-secret key-rotation identifier.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(super) fn sign(&self, parts: &[&[u8]]) -> [u8; 32] {
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.bytes.as_ref());
        let mut context = hmac::Context::with_key(&key);
        for part in parts {
            context.update(part);
        }
        let mut output = [0_u8; 32];
        for (target, source) in output.iter_mut().zip(context.sign().as_ref()) {
            *target = *source;
        }
        output
    }
}

impl fmt::Debug for FiscalJournalKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FiscalJournalKey")
            .field("key_id", &self.key_id)
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

/// Durable lifecycle state for one prepared fiscal command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FiscalCommandStatus {
    Prepared,
    Authorized,
    Rejected,
}

/// Whether an API call appended evidence or returned an exact idempotent replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FiscalJournalDisposition {
    Recorded,
    Replay,
}

/// Result of one synchronized command-journal operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiscalCommandReceipt {
    pub(super) disposition: FiscalJournalDisposition,
    pub(super) status: FiscalCommandStatus,
    pub(super) sequence: u64,
}

impl FiscalCommandReceipt {
    /// Returns whether durable evidence was newly recorded or exactly replayed.
    #[must_use]
    pub const fn disposition(self) -> FiscalJournalDisposition {
        self.disposition
    }

    /// Returns the resulting command lifecycle state.
    #[must_use]
    pub const fn status(self) -> FiscalCommandStatus {
        self.status
    }

    /// Returns the one-based sequence of the command's latest journal event.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
}

/// Secret-minimized recovery descriptor for one unresolved prepared command.
#[derive(Clone, Eq, PartialEq)]
pub struct FiscalPendingCommand {
    pub(super) command_id: String,
    pub(super) environment: NfseEnvironment,
    pub(super) request_digest: String,
    pub(super) prepared_at_unix_ms: i64,
    pub(super) sequence: u64,
}

impl FiscalPendingCommand {
    /// Returns the caller-owned opaque identifier used to locate protected request storage.
    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    /// Returns the official environment bound inside the signed DPS.
    pub const fn environment(&self) -> NfseEnvironment {
        self.environment
    }

    /// Returns the lowercase SHA-256 digest of the deterministic signed request envelope.
    pub fn request_digest(&self) -> &str {
        &self.request_digest
    }

    /// Returns when the local writer observed preparation, in Unix milliseconds.
    pub const fn prepared_at_unix_ms(&self) -> i64 {
        self.prepared_at_unix_ms
    }

    /// Returns the one-based sequence of the preparation event.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

impl fmt::Debug for FiscalPendingCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FiscalPendingCommand")
            .field("command_id", &"[REDACTED]")
            .field("environment", &self.environment)
            .field("request_digest", &"[REDACTED]")
            .field("prepared_at_unix_ms", &self.prepared_at_unix_ms)
            .field("sequence", &self.sequence)
            .finish()
    }
}

/// Externally retainable exact-tip checkpoint for truncation detection after restart.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct FiscalJournalCheckpoint {
    pub(super) sequence: u64,
    pub(super) end_offset: u64,
    pub(super) commitment: String,
}

impl FiscalJournalCheckpoint {
    /// Returns the exact number of events at this checkpoint.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Returns the exact synchronized file length at this checkpoint.
    pub const fn end_offset(&self) -> u64 {
        self.end_offset
    }

    /// Returns the authenticated tip commitment for independent persistence.
    pub fn commitment(&self) -> &str {
        &self.commitment
    }
}

impl fmt::Debug for FiscalJournalCheckpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FiscalJournalCheckpoint")
            .field("sequence", &self.sequence)
            .field("end_offset", &self.end_offset)
            .field("commitment", &"[REDACTED]")
            .finish()
    }
}

/// Bounded counters for the current authenticated journal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiscalJournalSnapshot {
    pub(super) records: usize,
    pub(super) pending: usize,
    pub(super) terminal: usize,
    pub(super) bytes: u64,
    pub(super) max_bytes: u64,
}

impl FiscalJournalSnapshot {
    /// Returns the total count of prepared and terminal events.
    pub const fn records(self) -> usize {
        self.records
    }

    /// Returns the number of commands still requiring reconciliation.
    pub const fn pending(self) -> usize {
        self.pending
    }

    /// Returns the number of commands with authorized or rejected evidence.
    pub const fn terminal(self) -> usize {
        self.terminal
    }

    /// Returns the current journal size including framing and header bytes.
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// Returns the configured hard byte ceiling.
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
