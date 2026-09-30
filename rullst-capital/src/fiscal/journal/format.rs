use super::{FiscalJournalError, FiscalJournalKey, JournalEvent};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};
use subtle::ConstantTimeEq as _;

const HEADER_PREFIX: &str = "RULLST-NFSE-COMMAND-JOURNAL-V1";
const HEADER_PROBE_DOMAIN: &[u8] = b"rullst.nfse.command-journal.header.v1";
const FRAME_DOMAIN: &[u8] = b"rullst.nfse.command-journal.frame.v1";
const LENGTH_HEX_BYTES: usize = 8;
const TAG_HEX_BYTES: usize = 64;
const FRAME_PREFIX_BYTES: usize = LENGTH_HEX_BYTES + 1 + TAG_HEX_BYTES + 1;
const MAX_HEADER_BYTES: usize = 192;
const MAX_RECORD_BYTES: usize = 8 * 1024;

pub(super) struct JournalFileState {
    file: File,
    pub(super) bytes: u64,
    pub(super) records: usize,
    pub(super) last_tag: [u8; 32],
    pub(super) healthy: bool,
}

pub(super) fn open(
    path: &Path,
    max_bytes: u64,
    key: &FiscalJournalKey,
) -> Result<(JournalFileState, Vec<JournalEvent>), FiscalJournalError> {
    validate_target(path)?;
    let mut file = open_file(path)?;
    let metadata = file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?;
    if !metadata.is_file() {
        return Err(FiscalJournalError::UnsafeFileType);
    }
    if metadata.len() > max_bytes {
        return Err(FiscalJournalError::CapacityExceeded);
    }

    if metadata.len() == 0 {
        let (header, probe) = encode_header(key);
        if header.len() as u64 > max_bytes {
            return Err(FiscalJournalError::InvalidCapacity);
        }
        file.write_all(&header)
            .map_err(|error| io_failure("initialize", &error))?;
        file.sync_data()
            .map_err(|error| io_failure("initialize sync", &error))?;
        sync_parent_directory(path)?;
        return Ok((
            JournalFileState {
                file,
                bytes: header.len() as u64,
                records: 0,
                last_tag: probe,
                healthy: true,
            },
            Vec::new(),
        ));
    }

    let (bytes, events, last_tag) = decode_file(&mut file, max_bytes, key)?;
    Ok((
        JournalFileState {
            file,
            bytes,
            records: events.len(),
            last_tag,
            healthy: true,
        },
        events,
    ))
}

pub(super) fn verify_and_read(
    state: &mut JournalFileState,
    max_bytes: u64,
    key: &FiscalJournalKey,
) -> Result<Vec<JournalEvent>, FiscalJournalError> {
    if !state.healthy {
        return Err(FiscalJournalError::UnhealthyWriter);
    }
    let length = state
        .file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length != state.bytes {
        return Err(FiscalJournalError::ExternalModification);
    }
    let (bytes, events, last_tag) = decode_file(&mut state.file, max_bytes, key)?;
    if bytes != state.bytes || events.len() != state.records || last_tag != state.last_tag {
        return Err(FiscalJournalError::ExternalModification);
    }
    Ok(events)
}

pub(super) fn append(
    state: &mut JournalFileState,
    max_bytes: u64,
    key: &FiscalJournalKey,
    event: &JournalEvent,
) -> Result<(), FiscalJournalError> {
    if !state.healthy {
        return Err(FiscalJournalError::UnhealthyWriter);
    }
    let payload = serde_json::to_vec(event).map_err(|_| FiscalJournalError::Encoding)?;
    if payload.len() > MAX_RECORD_BYTES {
        return Err(FiscalJournalError::RecordTooLarge);
    }
    let tag = key.sign(&[FRAME_DOMAIN, &state.last_tag, &payload]);
    let prefix = format!("{:08x}:{}:", payload.len(), hex::encode(tag));
    let frame_bytes = prefix
        .len()
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(1))
        .ok_or(FiscalJournalError::RecordTooLarge)?;
    let final_bytes = state
        .bytes
        .checked_add(frame_bytes as u64)
        .ok_or(FiscalJournalError::CapacityExceeded)?;
    if final_bytes > max_bytes {
        return Err(FiscalJournalError::CapacityExceeded);
    }

    let mut frame = Vec::with_capacity(frame_bytes);
    frame.extend_from_slice(prefix.as_bytes());
    frame.extend_from_slice(&payload);
    frame.push(b'\n');
    let previous_bytes = state.bytes;
    // Unix appends through O_APPEND; elsewhere the explicit offset places the
    // frame at the authenticated end, including after a recovery truncation.
    if let Err(error) = state
        .file
        .seek(SeekFrom::Start(previous_bytes))
        .and_then(|_| state.file.write_all(&frame))
    {
        return recover_partial_write(state, previous_bytes, &error);
    }
    state.bytes = final_bytes;
    state.records = state.records.saturating_add(1);
    state.last_tag = tag;
    if state.file.sync_data().is_err() {
        state.healthy = false;
        return Err(FiscalJournalError::DurabilityUncertain);
    }
    Ok(())
}

/// Returns the exact encoded size of one frame for this event.
pub(super) fn frame_bytes(event: &JournalEvent) -> Result<u64, FiscalJournalError> {
    let payload = serde_json::to_vec(event).map_err(|_| FiscalJournalError::Encoding)?;
    FRAME_PREFIX_BYTES
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(1))
        .and_then(|length| u64::try_from(length).ok())
        .ok_or(FiscalJournalError::RecordTooLarge)
}

fn decode_file(
    file: &mut File,
    max_bytes: u64,
    key: &FiscalJournalKey,
) -> Result<(u64, Vec<JournalEvent>, [u8; 32]), FiscalJournalError> {
    let length = file
        .metadata()
        .map_err(|error| io_failure("metadata", &error))?
        .len();
    if length > max_bytes {
        return Err(FiscalJournalError::CapacityExceeded);
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
        return Err(FiscalJournalError::ExternalModification);
    }

    let header_end = bytes
        .iter()
        .take(MAX_HEADER_BYTES)
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| corrupt(0, "missing or oversized header"))?;
    let mut previous_tag = decode_header(&bytes[..header_end], key)?;
    let mut cursor = header_end
        .checked_add(1)
        .ok_or_else(|| corrupt(0, "invalid header offset"))?;
    let mut events = Vec::new();
    while cursor < bytes.len() {
        if events.len() >= super::MAX_FISCAL_JOURNAL_RECORDS {
            return Err(FiscalJournalError::RecordCapacityExceeded);
        }
        let relative_end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| corrupt(events.len() + 1, "truncated frame"))?;
        let end = cursor
            .checked_add(relative_end)
            .ok_or_else(|| corrupt(events.len() + 1, "invalid frame offset"))?;
        let (event, tag) = decode_frame(&bytes[cursor..end], events.len() + 1, key, &previous_tag)?;
        events.push(event);
        previous_tag = tag;
        cursor = end
            .checked_add(1)
            .ok_or_else(|| corrupt(events.len(), "invalid frame offset"))?;
    }
    Ok((length, events, previous_tag))
}

fn decode_frame(
    frame: &[u8],
    record: usize,
    key: &FiscalJournalKey,
    previous_tag: &[u8; 32],
) -> Result<(JournalEvent, [u8; 32]), FiscalJournalError> {
    if frame.len() < FRAME_PREFIX_BYTES
        || frame.get(LENGTH_HEX_BYTES) != Some(&b':')
        || frame.get(LENGTH_HEX_BYTES + 1 + TAG_HEX_BYTES) != Some(&b':')
    {
        return Err(corrupt(record, "invalid frame header"));
    }
    let length_text = std::str::from_utf8(&frame[..LENGTH_HEX_BYTES])
        .map_err(|_| corrupt(record, "invalid record length"))?;
    let payload_length = usize::from_str_radix(length_text, 16)
        .map_err(|_| corrupt(record, "invalid record length"))?;
    if payload_length > MAX_RECORD_BYTES {
        return Err(corrupt(record, "record length exceeds limit"));
    }
    let payload = &frame[FRAME_PREFIX_BYTES..];
    if payload.len() != payload_length {
        return Err(corrupt(record, "record length mismatch"));
    }
    let recorded = decode_tag(
        &frame[LENGTH_HEX_BYTES + 1..LENGTH_HEX_BYTES + 1 + TAG_HEX_BYTES],
        record,
    )?;
    let expected = key.sign(&[FRAME_DOMAIN, previous_tag, payload]);
    if expected.ct_eq(&recorded).unwrap_u8() != 1 {
        return Err(corrupt(record, "authentication tag mismatch"));
    }
    let event =
        serde_json::from_slice(payload).map_err(|_| corrupt(record, "invalid event JSON"))?;
    Ok((event, recorded))
}

fn encode_header(key: &FiscalJournalKey) -> (Vec<u8>, [u8; 32]) {
    let probe = key.sign(&[HEADER_PROBE_DOMAIN, key.key_id().as_bytes()]);
    let header = format!("{HEADER_PREFIX}:{}:{}\n", key.key_id(), hex::encode(probe));
    (header.into_bytes(), probe)
}

fn decode_header(header: &[u8], key: &FiscalJournalKey) -> Result<[u8; 32], FiscalJournalError> {
    let header = std::str::from_utf8(header).map_err(|_| corrupt(0, "header is not UTF-8"))?;
    let mut fields = header.split(':');
    if fields.next() != Some(HEADER_PREFIX)
        || fields.next() != Some(key.key_id())
        || fields.clone().count() != 1
    {
        return Err(FiscalJournalError::KeyMismatch);
    }
    let recorded = fields
        .next()
        .ok_or_else(|| corrupt(0, "missing header authentication tag"))?;
    let recorded = decode_tag(recorded.as_bytes(), 0)?;
    let expected = key.sign(&[HEADER_PROBE_DOMAIN, key.key_id().as_bytes()]);
    if expected.ct_eq(&recorded).unwrap_u8() != 1 {
        return Err(FiscalJournalError::KeyMismatch);
    }
    Ok(recorded)
}

fn decode_tag(bytes: &[u8], record: usize) -> Result<[u8; 32], FiscalJournalError> {
    if bytes.len() != TAG_HEX_BYTES {
        return Err(corrupt(record, "invalid authentication tag length"));
    }
    let mut tag = [0_u8; 32];
    hex::decode_to_slice(bytes, &mut tag)
        .map_err(|_| corrupt(record, "invalid authentication tag"))?;
    Ok(tag)
}

fn open_file(path: &Path) -> Result<File, FiscalJournalError> {
    let mut options = OpenOptions::new();
    // Full write access, not an append-only handle: Windows withholds
    // FILE_WRITE_DATA from append handles, and partial-write recovery needs it
    // to truncate the file. Unix keeps append semantics through O_APPEND.
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_APPEND);
    }
    options
        .open(path)
        .map_err(|error| io_failure("open", &error))
}

// A new file's directory entry is durable only after its directory is synced;
// otherwise a crash could drop a journal whose prepared records were acknowledged.
#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), FiscalJournalError> {
    File::open(parent_directory(path))
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_failure("directory sync", &error))
}

// Windows cannot open a directory as a `File` for syncing; NTFS journals the
// directory entry of a created file.
#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), FiscalJournalError> {
    Ok(())
}

#[cfg(any(unix, test))]
fn parent_directory(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn validate_target(path: &Path) -> Result<(), FiscalJournalError> {
    if path.as_os_str().is_empty() {
        return Err(FiscalJournalError::InvalidPath);
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(FiscalJournalError::UnsafeFileType)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure("metadata", &error)),
    }
}

fn recover_partial_write(
    state: &mut JournalFileState,
    previous_bytes: u64,
    write_error: &io::Error,
) -> Result<(), FiscalJournalError> {
    if state.file.set_len(previous_bytes).is_err() || state.file.sync_data().is_err() {
        state.healthy = false;
        return Err(FiscalJournalError::RecoveryFailed);
    }
    Err(io_failure("append", write_error))
}

const fn corrupt(record: usize, reason: &'static str) -> FiscalJournalError {
    FiscalJournalError::CorruptRecord { record, reason }
}

fn io_failure(operation: &'static str, error: &io::Error) -> FiscalJournalError {
    FiscalJournalError::Io {
        operation,
        kind: error.kind(),
    }
}

#[cfg(test)]
mod file_tests {
    use super::super::{JournalEnvironment, JournalOutcome};
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn a_bare_file_name_syncs_the_working_directory() {
        assert_eq!(
            parent_directory(Path::new("fiscal.journal")),
            Path::new(".")
        );
        assert_eq!(
            parent_directory(Path::new("/var/lib/app/fiscal.journal")),
            Path::new("/var/lib/app")
        );
    }

    #[test]
    fn partial_write_recovery_truncates_and_later_appends_stay_valid() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rullst-capital-nfse-recovery-{}-{}.journal",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let key = FiscalJournalKey::try_new("recovery", [3_u8; 32]).expect("key");
        let (mut state, _) = open(&path, 4_096, &key).expect("new journal");
        let previous = state.bytes;
        state
            .file
            .write_all(b"torn-frame")
            .expect("simulate a torn write");
        let error = io::Error::other("disk full");
        assert!(matches!(
            recover_partial_write(&mut state, previous, &error),
            Err(FiscalJournalError::Io {
                operation: "append",
                ..
            })
        ));
        assert!(state.healthy, "recovery must truncate on every platform");
        assert_eq!(std::fs::metadata(&path).expect("metadata").len(), previous);

        let event = JournalEvent {
            schema_version: super::super::SCHEMA_VERSION,
            sequence: 1,
            command_id: "invoice:recovered".to_string(),
            environment: JournalEnvironment::Homologation,
            request_digest: "a".repeat(64),
            observed_at_unix_ms: 1,
            outcome: JournalOutcome::Prepared,
        };
        append(&mut state, 4_096, &key, &event).expect("append after recovery");
        assert_eq!(
            verify_and_read(&mut state, 4_096, &key)
                .expect("read")
                .len(),
            1
        );
        drop(state);
        let _cleanup = std::fs::remove_file(&path);
    }
}
