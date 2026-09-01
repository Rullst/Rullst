//! Bounded single-process durable spool for local security events.

use crate::{
    digest::sha256_hex,
    telemetry::{LiveSecurityEvent, SECURITY_EVENT_SCHEMA_VERSION},
};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use subtle::ConstantTimeEq;

/// Maximum supported file size for one durable SIEM spool (16 MiB).
pub const MAX_SIEM_SPOOL_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of framed events retained in one spool.
pub const MAX_SIEM_SPOOL_RECORDS: usize = 4_096;

const MIN_SIEM_SPOOL_BYTES: u64 = 512;
const MAX_SIEM_RECORD_BYTES: usize = 16 * 1024;
const SPOOL_MAGIC: &[u8] = b"RULLST-SIEM-SPOOL-V1\n";
const LENGTH_HEX_BYTES: usize = 8;
const DIGEST_HEX_BYTES: usize = 64;
const FRAME_PREFIX_BYTES: usize = LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES + 1;

/// Typed local-spool failures. Paths and event bodies are deliberately omitted.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum SiemSpoolError {
    /// The requested quota is zero, too small, or above the crate ceiling.
    #[error("SIEM spool capacity must be between 512 bytes and 16 MiB")]
    InvalidCapacity,
    /// The target path is empty.
    #[error("SIEM spool path cannot be empty")]
    InvalidPath,
    /// The target exists but is a symlink, directory, or another non-file type.
    #[error("SIEM spool target must be a regular non-symlink file")]
    UnsafeFileType,
    /// The configured byte quota would be exceeded.
    #[error("SIEM spool byte capacity is exhausted")]
    CapacityExceeded,
    /// The configured record-count quota would be exceeded.
    #[error("SIEM spool record capacity is exhausted")]
    RecordCapacityExceeded,
    /// One normalized event cannot fit in the bounded record format.
    #[error("SIEM spool record exceeds the per-record encoding limit")]
    RecordTooLarge,
    /// The existing file does not contain a valid v1 spool.
    #[error("SIEM spool record {record} is corrupt: {reason}")]
    CorruptRecord {
        /// One-based record number; zero identifies the file header.
        record: usize,
        /// Bounded static diagnostic without event or path data.
        reason: &'static str,
    },
    /// The file changed outside this spool handle.
    #[error("SIEM spool changed outside the active writer")]
    ExternalModification,
    /// A prior partial-write recovery or durability operation was inconclusive.
    #[error("SIEM spool writer is unhealthy and must be reopened")]
    UnhealthyWriter,
    /// A completed write could not be confirmed durable.
    #[error("SIEM spool write completed but durability could not be confirmed")]
    DurabilityUncertain,
    /// A partial write could not be rolled back to the previous file boundary.
    #[error("SIEM spool partial-write recovery failed")]
    RecoveryFailed,
    /// The in-process serialization lock was poisoned.
    #[error("SIEM spool lock is unavailable")]
    LockUnavailable,
    /// A filesystem operation failed. Only the operation and error kind are exposed.
    #[error("SIEM spool {operation} failed: {kind:?}")]
    Io {
        /// Stable operation label.
        operation: &'static str,
        /// Portable I/O error category without a path or event body.
        kind: io::ErrorKind,
    },
    /// JSON encoding failed without exposing event data.
    #[error("SIEM spool event encoding failed")]
    Encoding,
}

/// Durable append receipt for one local event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SiemSpoolReceipt {
    sequence: u64,
    end_offset: u64,
}

impl SiemSpoolReceipt {
    /// One-based sequence within this spool file.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// File offset immediately after the synchronized frame.
    #[must_use]
    pub const fn end_offset(self) -> u64 {
        self.end_offset
    }
}

/// Current bounded spool state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SiemSpoolSnapshot {
    records: usize,
    bytes: u64,
    max_bytes: u64,
}

impl SiemSpoolSnapshot {
    /// Number of validated local records.
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

struct SpoolState {
    file: File,
    bytes: u64,
    records: usize,
    healthy: bool,
}

/// Single-process durable local event spool.
///
/// Each JSON event is length-framed and protected by an unkeyed SHA-256 digest
/// so restart validation can detect truncation and accidental modification.
/// The digest is not an authenticity proof. The caller owns file permissions,
/// directory trust, rotation, backup, delivery, acknowledgement and retention.
pub struct DurableSiemSpool {
    state: Mutex<SpoolState>,
    max_bytes: u64,
}

impl DurableSiemSpool {
    /// Opens or creates a spool with the crate's 16 MiB ceiling.
    pub fn try_open(path: impl Into<PathBuf>) -> Result<Self, SiemSpoolError> {
        Self::try_open_with_max_bytes(path, MAX_SIEM_SPOOL_BYTES)
    }

    /// Opens or creates a spool with a smaller explicit byte quota.
    pub fn try_open_with_max_bytes(
        path: impl Into<PathBuf>,
        max_bytes: u64,
    ) -> Result<Self, SiemSpoolError> {
        if !(MIN_SIEM_SPOOL_BYTES..=MAX_SIEM_SPOOL_BYTES).contains(&max_bytes) {
            return Err(SiemSpoolError::InvalidCapacity);
        }
        let path = path.into();
        validate_target(&path)?;
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|error| io_failure("open", &error))?;
        let metadata = file
            .metadata()
            .map_err(|error| io_failure("metadata", &error))?;
        if !metadata.is_file() {
            return Err(SiemSpoolError::UnsafeFileType);
        }
        if metadata.len() > max_bytes {
            return Err(SiemSpoolError::CapacityExceeded);
        }

        let (bytes, records) = if metadata.len() == 0 {
            file.write_all(SPOOL_MAGIC)
                .map_err(|error| io_failure("initialize", &error))?;
            file.sync_data()
                .map_err(|error| io_failure("initialize sync", &error))?;
            (SPOOL_MAGIC.len() as u64, 0)
        } else {
            let (bytes, events) = decode_file(&mut file, max_bytes)?;
            (bytes, events.len())
        };

        Ok(Self {
            state: Mutex::new(SpoolState {
                file,
                bytes,
                records,
                healthy: true,
            }),
            max_bytes,
        })
    }

    /// Normalizes and synchronously appends one unsigned local event.
    pub fn append_local(
        &self,
        mut event: LiveSecurityEvent,
    ) -> Result<SiemSpoolReceipt, SiemSpoolError> {
        event.verified_hmac = false;
        let event = event.normalized();
        let payload = serde_json::to_vec(&event).map_err(|_| SiemSpoolError::Encoding)?;
        let frame = encode_frame(&payload)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| SiemSpoolError::LockUnavailable)?;
        ensure_writer_state(&state, self.max_bytes, frame.len())?;

        let previous_bytes = state.bytes;
        if let Err(write_error) = state.file.write_all(&frame) {
            return recover_partial_write(&mut state, previous_bytes, &write_error);
        }
        state.bytes = state
            .bytes
            .checked_add(frame.len() as u64)
            .ok_or(SiemSpoolError::CapacityExceeded)?;
        state.records = state.records.saturating_add(1);
        if state.file.sync_data().is_err() {
            state.healthy = false;
            return Err(SiemSpoolError::DurabilityUncertain);
        }

        Ok(SiemSpoolReceipt {
            sequence: state.records as u64,
            end_offset: state.bytes,
        })
    }

    /// Re-reads and validates every frame from the active file handle.
    pub fn read_local(&self) -> Result<Vec<LiveSecurityEvent>, SiemSpoolError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SiemSpoolError::LockUnavailable)?;
        if !state.healthy {
            return Err(SiemSpoolError::UnhealthyWriter);
        }
        verify_unchanged(&state)?;
        let (bytes, events) = decode_file(&mut state.file, self.max_bytes)?;
        state.bytes = bytes;
        state.records = events.len();
        Ok(events)
    }

    /// Returns the in-process counters without reading event bodies.
    pub fn snapshot(&self) -> Result<SiemSpoolSnapshot, SiemSpoolError> {
        let state = self
            .state
            .lock()
            .map_err(|_| SiemSpoolError::LockUnavailable)?;
        if !state.healthy {
            return Err(SiemSpoolError::UnhealthyWriter);
        }
        verify_unchanged(&state)?;
        Ok(SiemSpoolSnapshot {
            records: state.records,
            bytes: state.bytes,
            max_bytes: self.max_bytes,
        })
    }
}

fn validate_target(path: &Path) -> Result<(), SiemSpoolError> {
    if path.as_os_str().is_empty() {
        return Err(SiemSpoolError::InvalidPath);
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(SiemSpoolError::UnsafeFileType)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure("metadata", &error)),
    }
}

fn ensure_writer_state(
    state: &SpoolState,
    max_bytes: u64,
    frame_bytes: usize,
) -> Result<(), SiemSpoolError> {
    if !state.healthy {
        return Err(SiemSpoolError::UnhealthyWriter);
    }
    verify_unchanged(state)?;
    if state.records >= MAX_SIEM_SPOOL_RECORDS {
        return Err(SiemSpoolError::RecordCapacityExceeded);
    }
    let final_bytes = state
        .bytes
        .checked_add(frame_bytes as u64)
        .ok_or(SiemSpoolError::CapacityExceeded)?;
    if final_bytes > max_bytes {
        return Err(SiemSpoolError::CapacityExceeded);
    }
    Ok(())
}

fn verify_unchanged(state: &SpoolState) -> Result<(), SiemSpoolError> {
    let length = state
        .file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length != state.bytes {
        return Err(SiemSpoolError::ExternalModification);
    }
    Ok(())
}

fn recover_partial_write(
    state: &mut SpoolState,
    previous_bytes: u64,
    write_error: &io::Error,
) -> Result<SiemSpoolReceipt, SiemSpoolError> {
    if state.file.set_len(previous_bytes).is_err() || state.file.sync_data().is_err() {
        state.healthy = false;
        return Err(SiemSpoolError::RecoveryFailed);
    }
    Err(io_failure("append", write_error))
}

fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, SiemSpoolError> {
    if payload.len() > MAX_SIEM_RECORD_BYTES {
        return Err(SiemSpoolError::RecordTooLarge);
    }
    let digest = sha256_hex(payload);
    let prefix = format!("{:08x}:{digest}:", payload.len());
    let capacity = prefix
        .len()
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(SiemSpoolError::RecordTooLarge)?;
    let mut frame = Vec::with_capacity(capacity);
    frame.extend_from_slice(prefix.as_bytes());
    frame.extend_from_slice(payload);
    frame.push(b'\n');
    Ok(frame)
}

fn decode_file(
    file: &mut File,
    max_bytes: u64,
) -> Result<(u64, Vec<LiveSecurityEvent>), SiemSpoolError> {
    let length = file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length > max_bytes {
        return Err(SiemSpoolError::CapacityExceeded);
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
        return Err(SiemSpoolError::ExternalModification);
    }
    if !bytes.starts_with(SPOOL_MAGIC) {
        return Err(corrupt(0, "unsupported or missing header"));
    }

    let mut cursor = SPOOL_MAGIC.len();
    let mut events = Vec::new();
    while cursor < bytes.len() {
        if events.len() >= MAX_SIEM_SPOOL_RECORDS {
            return Err(SiemSpoolError::RecordCapacityExceeded);
        }
        let relative_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| corrupt(events.len() + 1, "truncated frame"))?;
        let end = cursor
            .checked_add(relative_end)
            .ok_or_else(|| corrupt(events.len() + 1, "invalid frame offset"))?;
        events.push(decode_frame(&bytes[cursor..end], events.len() + 1)?);
        cursor = end
            .checked_add(1)
            .ok_or_else(|| corrupt(events.len(), "invalid frame offset"))?;
    }
    Ok((length, events))
}

fn decode_frame(frame: &[u8], record: usize) -> Result<LiveSecurityEvent, SiemSpoolError> {
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
    if payload_length > MAX_SIEM_RECORD_BYTES {
        return Err(corrupt(record, "record length exceeds limit"));
    }
    let payload = &frame[FRAME_PREFIX_BYTES..];
    if payload.len() != payload_length {
        return Err(corrupt(record, "record length mismatch"));
    }
    let expected_digest = sha256_hex(payload);
    let recorded_digest = &frame[LENGTH_HEX_BYTES + 1..LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES];
    if expected_digest
        .as_bytes()
        .ct_eq(recorded_digest)
        .unwrap_u8()
        != 1
    {
        return Err(corrupt(record, "digest mismatch"));
    }
    let event: LiveSecurityEvent =
        serde_json::from_slice(payload).map_err(|_| corrupt(record, "invalid event JSON"))?;
    let mut normalized = event.clone();
    normalized.verified_hmac = false;
    normalized = normalized.normalized();
    if event.schema_version != SECURITY_EVENT_SCHEMA_VERSION || event != normalized {
        return Err(corrupt(record, "event violates the local v1 contract"));
    }
    Ok(event)
}

const fn corrupt(record: usize, reason: &'static str) -> SiemSpoolError {
    SiemSpoolError::CorruptRecord { record, reason }
}

fn io_failure(operation: &'static str, error: &io::Error) -> SiemSpoolError {
    SiemSpoolError::Io {
        operation,
        kind: error.kind(),
    }
}

#[cfg(test)]
#[path = "spool_tests.rs"]
mod tests;
