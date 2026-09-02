//! Shared bounded file format for local AI audit evidence.

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Mutex,
};

/// Maximum supported size for one durable local AI audit file (16 MiB).
pub const MAX_AI_AUDIT_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of events retained in one durable local AI audit file.
pub const MAX_AI_AUDIT_RECORDS: usize = 4_096;

const MIN_AI_AUDIT_BYTES: u64 = 512;
const MAX_AI_AUDIT_RECORD_BYTES: usize = 16 * 1024;
const LENGTH_HEX_BYTES: usize = 8;
const DIGEST_HEX_BYTES: usize = 64;
const FRAME_PREFIX_BYTES: usize = LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES + 1;

/// Typed local AI audit failures. Paths and event bodies are deliberately omitted.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum DurableAuditError {
    /// The byte quota is zero, too small, or above the crate ceiling.
    #[error("AI audit capacity must be between 512 bytes and 16 MiB")]
    InvalidCapacity,
    /// The target path is empty.
    #[error("AI audit path cannot be empty")]
    InvalidPath,
    /// The target exists but is a symlink, directory, or another non-file type.
    #[error("AI audit target must be a regular non-symlink file")]
    UnsafeFileType,
    /// The configured byte quota would be exceeded.
    #[error("AI audit byte capacity is exhausted")]
    CapacityExceeded,
    /// The fixed record-count quota would be exceeded.
    #[error("AI audit record capacity is exhausted")]
    RecordCapacityExceeded,
    /// One event cannot fit in the bounded frame format.
    #[error("AI audit record exceeds the per-record encoding limit")]
    RecordTooLarge,
    /// A new event violates the semantic contract for its audit stream.
    #[error("AI audit event is invalid: {reason}")]
    InvalidEvent {
        /// Bounded static diagnostic without event data.
        reason: &'static str,
    },
    /// The existing file does not contain a valid event stream.
    #[error("AI audit record {record} is corrupt: {reason}")]
    CorruptRecord {
        /// One-based record number; zero identifies the file header.
        record: usize,
        /// Bounded static diagnostic without event or path data.
        reason: &'static str,
    },
    /// The file length changed outside this writer.
    #[error("AI audit file changed outside the active writer")]
    ExternalModification,
    /// A prior durability or partial-write recovery operation was inconclusive.
    #[error("AI audit writer is unhealthy and must be reopened")]
    UnhealthyWriter,
    /// A completed write could not be confirmed durable.
    #[error("AI audit write completed but durability could not be confirmed")]
    DurabilityUncertain,
    /// A partial write could not be rolled back to the previous boundary.
    #[error("AI audit partial-write recovery failed")]
    RecoveryFailed,
    /// The in-process serialization lock was poisoned.
    #[error("AI audit lock is unavailable")]
    LockUnavailable,
    /// A filesystem operation failed. Only the operation and error kind are exposed.
    #[error("AI audit {operation} failed: {kind:?}")]
    Io {
        /// Stable operation label.
        operation: &'static str,
        /// Portable I/O category without a path or event body.
        kind: io::ErrorKind,
    },
    /// JSON encoding failed without exposing event data.
    #[error("AI audit event encoding failed")]
    Encoding,
}

/// Current bounded state of a durable local AI audit file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurableAuditSnapshot {
    records: usize,
    bytes: u64,
    max_bytes: u64,
}

impl DurableAuditSnapshot {
    /// Number of validated records in the file.
    #[must_use]
    pub const fn records(self) -> usize {
        self.records
    }

    /// Current file length, including the version header and frame metadata.
    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.bytes
    }

    /// Configured hard file-size ceiling.
    #[must_use]
    pub const fn max_bytes(self) -> u64 {
        self.max_bytes
    }
}

pub(crate) trait DurableAuditRecord: Serialize + DeserializeOwned {
    const MAGIC: &'static [u8];

    fn validate(&self) -> Result<(), &'static str>;
}

struct AuditState {
    file: File,
    bytes: u64,
    records: usize,
    healthy: bool,
}

pub(crate) struct DurableAuditLog<E> {
    state: Mutex<AuditState>,
    max_bytes: u64,
    event: PhantomData<E>,
}

impl<E> DurableAuditLog<E>
where
    E: DurableAuditRecord,
{
    pub(crate) fn try_open(path: impl Into<PathBuf>) -> Result<Self, DurableAuditError> {
        Self::try_open_with_max_bytes(path, MAX_AI_AUDIT_BYTES)
    }

    pub(crate) fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        max_bytes: u64,
    ) -> Result<Self, DurableAuditError> {
        if !(MIN_AI_AUDIT_BYTES..=MAX_AI_AUDIT_BYTES).contains(&max_bytes) {
            return Err(DurableAuditError::InvalidCapacity);
        }
        let path = path.into();
        validate_target(&path)?;
        let mut file = open_audit_file(&path)?;
        let metadata = file
            .metadata()
            .map_err(|error| io_failure("metadata", &error))?;
        if !metadata.is_file() {
            return Err(DurableAuditError::UnsafeFileType);
        }
        if metadata.len() > max_bytes {
            return Err(DurableAuditError::CapacityExceeded);
        }

        let (bytes, records) = if metadata.len() == 0 {
            file.write_all(E::MAGIC)
                .map_err(|error| io_failure("initialize", &error))?;
            file.sync_data()
                .map_err(|error| io_failure("initialize sync", &error))?;
            (E::MAGIC.len() as u64, 0)
        } else {
            let (bytes, events) = decode_file::<E>(&mut file, max_bytes)?;
            (bytes, events.len())
        };

        Ok(Self {
            state: Mutex::new(AuditState {
                file,
                bytes,
                records,
                healthy: true,
            }),
            max_bytes,
            event: PhantomData,
        })
    }

    pub(crate) fn append(&self, event: E) -> Result<(), DurableAuditError> {
        event
            .validate()
            .map_err(|reason| DurableAuditError::InvalidEvent { reason })?;
        let payload = serde_json::to_vec(&event).map_err(|_| DurableAuditError::Encoding)?;
        let frame = encode_frame(&payload)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| DurableAuditError::LockUnavailable)?;
        ensure_writer_state(&state, self.max_bytes, frame.len())?;

        let previous_bytes = state.bytes;
        if let Err(write_error) = state.file.write_all(&frame) {
            return recover_partial_write(&mut state, previous_bytes, &write_error);
        }
        state.bytes = state
            .bytes
            .checked_add(frame.len() as u64)
            .ok_or(DurableAuditError::CapacityExceeded)?;
        state.records = state.records.saturating_add(1);
        if state.file.sync_data().is_err() {
            state.healthy = false;
            return Err(DurableAuditError::DurabilityUncertain);
        }
        Ok(())
    }

    pub(crate) fn entries(&self) -> Result<Vec<E>, DurableAuditError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| DurableAuditError::LockUnavailable)?;
        if !state.healthy {
            return Err(DurableAuditError::UnhealthyWriter);
        }
        verify_unchanged(&state)?;
        let (bytes, events) = decode_file::<E>(&mut state.file, self.max_bytes)?;
        state.bytes = bytes;
        state.records = events.len();
        Ok(events)
    }

    pub(crate) fn snapshot(&self) -> Result<DurableAuditSnapshot, DurableAuditError> {
        let state = self
            .state
            .lock()
            .map_err(|_| DurableAuditError::LockUnavailable)?;
        if !state.healthy {
            return Err(DurableAuditError::UnhealthyWriter);
        }
        verify_unchanged(&state)?;
        Ok(DurableAuditSnapshot {
            records: state.records,
            bytes: state.bytes,
            max_bytes: self.max_bytes,
        })
    }
}

fn open_audit_file(path: &Path) -> Result<File, DurableAuditError> {
    let mut options = OpenOptions::new();
    options.read(true).append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        // Close the validate/open race: even if the path is exchanged after
        // `symlink_metadata`, the kernel must reject a final-component link.
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| io_failure("open", &error))
}

fn validate_target(path: &Path) -> Result<(), DurableAuditError> {
    if path.as_os_str().is_empty() {
        return Err(DurableAuditError::InvalidPath);
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(DurableAuditError::UnsafeFileType)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure("metadata", &error)),
    }
}

fn ensure_writer_state(
    state: &AuditState,
    max_bytes: u64,
    frame_bytes: usize,
) -> Result<(), DurableAuditError> {
    if !state.healthy {
        return Err(DurableAuditError::UnhealthyWriter);
    }
    verify_unchanged(state)?;
    if state.records >= MAX_AI_AUDIT_RECORDS {
        return Err(DurableAuditError::RecordCapacityExceeded);
    }
    let final_bytes = state
        .bytes
        .checked_add(frame_bytes as u64)
        .ok_or(DurableAuditError::CapacityExceeded)?;
    if final_bytes > max_bytes {
        return Err(DurableAuditError::CapacityExceeded);
    }
    Ok(())
}

fn verify_unchanged(state: &AuditState) -> Result<(), DurableAuditError> {
    let length = state
        .file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length != state.bytes {
        return Err(DurableAuditError::ExternalModification);
    }
    Ok(())
}

fn recover_partial_write(
    state: &mut AuditState,
    previous_bytes: u64,
    write_error: &io::Error,
) -> Result<(), DurableAuditError> {
    if state.file.set_len(previous_bytes).is_err() || state.file.sync_data().is_err() {
        state.healthy = false;
        return Err(DurableAuditError::RecoveryFailed);
    }
    Err(io_failure("append", write_error))
}

fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, DurableAuditError> {
    if payload.len() > MAX_AI_AUDIT_RECORD_BYTES {
        return Err(DurableAuditError::RecordTooLarge);
    }
    let digest = sha256_hex(payload);
    let prefix = format!("{:08x}:{digest}:", payload.len());
    let capacity = prefix
        .len()
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(DurableAuditError::RecordTooLarge)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(prefix.as_bytes());
    frame.extend_from_slice(payload);
    frame.push(b'\n');
    Ok(frame)
}

fn decode_file<E>(file: &mut File, max_bytes: u64) -> Result<(u64, Vec<E>), DurableAuditError>
where
    E: DurableAuditRecord,
{
    let length = file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length > max_bytes {
        return Err(DurableAuditError::CapacityExceeded);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_failure("seek", &error))?;
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_failure("read", &error))?;
    file.seek(SeekFrom::End(0))
        .map_err(|error| io_failure("seek", &error))?;
    if bytes.len() as u64 != length {
        return Err(DurableAuditError::ExternalModification);
    }
    if !bytes.starts_with(E::MAGIC) {
        return Err(corrupt(0, "unsupported or missing header"));
    }

    let mut cursor = E::MAGIC.len();
    let mut events = Vec::new();
    while cursor < bytes.len() {
        if events.len() >= MAX_AI_AUDIT_RECORDS {
            return Err(DurableAuditError::RecordCapacityExceeded);
        }
        let relative_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| corrupt(events.len() + 1, "truncated frame"))?;
        let end = cursor
            .checked_add(relative_end)
            .ok_or_else(|| corrupt(events.len() + 1, "invalid frame offset"))?;
        events.push(decode_frame::<E>(&bytes[cursor..end], events.len() + 1)?);
        cursor = end
            .checked_add(1)
            .ok_or_else(|| corrupt(events.len(), "invalid frame offset"))?;
    }
    Ok((length, events))
}

fn decode_frame<E>(frame: &[u8], record: usize) -> Result<E, DurableAuditError>
where
    E: DurableAuditRecord,
{
    if frame.len() < FRAME_PREFIX_BYTES
        || frame.get(LENGTH_HEX_BYTES) != Some(&b':')
        || frame.get(LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES) != Some(&b':')
    {
        return Err(corrupt(record, "invalid frame header"));
    }
    let length_text = std::str::from_utf8(&frame[..LENGTH_HEX_BYTES])
        .map_err(|_| corrupt(record, "invalid length"))?;
    let payload_length =
        usize::from_str_radix(length_text, 16).map_err(|_| corrupt(record, "invalid length"))?;
    if payload_length > MAX_AI_AUDIT_RECORD_BYTES {
        return Err(corrupt(record, "record length exceeds limit"));
    }
    let payload = &frame[FRAME_PREFIX_BYTES..];
    if payload.len() != payload_length {
        return Err(corrupt(record, "record length mismatch"));
    }
    let expected_digest = sha256_hex(payload);
    let recorded_digest = &frame[LENGTH_HEX_BYTES + 1..LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES];
    if expected_digest.as_bytes() != recorded_digest {
        return Err(corrupt(record, "digest mismatch"));
    }
    let event: E =
        serde_json::from_slice(payload).map_err(|_| corrupt(record, "invalid event JSON"))?;
    event.validate().map_err(|reason| corrupt(record, reason))?;
    Ok(event)
}

fn sha256_hex(payload: &[u8]) -> String {
    let digest = Sha256::digest(payload);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

const fn corrupt(record: usize, reason: &'static str) -> DurableAuditError {
    DurableAuditError::CorruptRecord { record, reason }
}

fn io_failure(operation: &'static str, error: &io::Error) -> DurableAuditError {
    DurableAuditError::Io {
        operation,
        kind: error.kind(),
    }
}
