//! Frame, file and capacity bounds of the shared durable audit format.
use super::*;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Note {
    text: String,
}

impl DurableAuditRecord for Note {
    const MAGIC: &'static [u8] = b"TEST-AUDIT-V1\n";

    fn validate(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

fn note(length: usize) -> Note {
    Note {
        text: "n".repeat(length),
    }
}

/// A fresh path under the temporary directory; the caller removes it.
fn scratch(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-ai-durable-{label}-{}-{nonce}",
        std::process::id()
    ))
}

/// One frame around `payload`, built without the encoder under test.
fn raw_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = format!("{:08x}:{}:", payload.len(), sha256_hex(payload)).into_bytes();
    frame.extend_from_slice(payload);
    frame
}

#[test]
fn capacity_ceiling_is_sixteen_mebibytes() {
    assert_eq!(MAX_AI_AUDIT_BYTES, 16_777_216);
    let path = scratch("ceiling");
    let log = DurableAuditLog::<Note>::try_open(&path).expect("default capacity");
    let snapshot = log.snapshot().expect("snapshot");
    assert_eq!(snapshot.max_bytes(), 16_777_216);
    assert_eq!(snapshot.bytes(), 14);
    drop(log);
    std::fs::remove_file(&path).expect("remove log");
    assert!(DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 16_777_216).is_ok());
    std::fs::remove_file(&path).expect("remove log");
    assert!(matches!(
        DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 16_777_217),
        Err(DurableAuditError::InvalidCapacity)
    ));
}

#[test]
fn records_up_to_sixteen_kibibytes_are_framed() {
    let frame = encode_frame(&[b'x'; 16_384]).expect("a 16 KiB record fits");
    assert_eq!(frame.len(), 16_384 + 75);
    assert_eq!(
        encode_frame(&[b'x'; 16_385]),
        Err(DurableAuditError::RecordTooLarge)
    );

    // The decoder accepts the same ceiling and refuses one byte more.
    let fits = serde_json::to_vec(&note(16_384 - 11)).expect("payload");
    assert_eq!(fits.len(), 16_384);
    assert_eq!(
        decode_frame::<Note>(&raw_frame(&fits), 1).expect("16 KiB record"),
        note(16_373)
    );
    let over = serde_json::to_vec(&note(16_385 - 11)).expect("payload");
    assert_eq!(
        decode_frame::<Note>(&raw_frame(&over), 1),
        Err(DurableAuditError::CorruptRecord {
            record: 1,
            reason: "record length exceeds limit",
        })
    );
}

#[test]
fn appends_and_existing_files_may_fill_the_capacity_exactly() {
    let path = scratch("exact");
    // The 14-byte header plus one 75-byte frame overhead leave 423 bytes of
    // payload in 512, of which 11 are the JSON around the text.
    let log = DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 512).expect("open");
    log.append(note(412))
        .expect("an append that fills the file exactly");
    let snapshot = log.snapshot().expect("snapshot");
    assert_eq!((snapshot.bytes(), snapshot.max_bytes()), (512, 512));
    assert_eq!(
        log.append(note(1)),
        Err(DurableAuditError::CapacityExceeded)
    );
    drop(log);

    let reopened =
        DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 512).expect("a full file opens");
    assert_eq!(reopened.entries().expect("entries"), vec![note(412)]);
    drop(reopened);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(b"x"))
        .expect("grow file");
    assert!(matches!(
        DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 512),
        Err(DurableAuditError::CapacityExceeded)
    ));
    std::fs::remove_file(&path).expect("remove log");
}

#[test]
fn a_truncated_frame_names_its_one_based_record() {
    let path = scratch("truncated");
    let log = DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 4_096).expect("open");
    log.append(note(3)).expect("first record");
    drop(log);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(b"0000"))
        .expect("append a partial frame");
    assert!(matches!(
        DurableAuditLog::<Note>::try_open_with_max_bytes(&path, 4_096),
        Err(DurableAuditError::CorruptRecord {
            record: 2,
            reason: "truncated frame",
        })
    ));
    std::fs::remove_file(&path).expect("remove log");
}

#[test]
fn frame_headers_need_both_separators_and_the_full_prefix() {
    let invalid_header = Err(DurableAuditError::CorruptRecord {
        record: 1,
        reason: "invalid frame header",
    });
    let valid = raw_frame(br#"{"text":"ok"}"#);
    assert_eq!(
        decode_frame::<Note>(&valid, 1),
        Ok(Note {
            text: "ok".to_string()
        })
    );
    for separator in [8, 73] {
        let mut frame = valid.clone();
        frame[separator] = b'0';
        assert_eq!(decode_frame::<Note>(&frame, 1), invalid_header);
    }
    // A bare 74-byte prefix passes the header check and fails as JSON.
    let empty = raw_frame(b"");
    assert_eq!(empty.len(), 74);
    assert_eq!(
        decode_frame::<Note>(&empty, 1),
        Err(DurableAuditError::CorruptRecord {
            record: 1,
            reason: "invalid event JSON",
        })
    );
    assert_eq!(decode_frame::<Note>(&empty[..73], 1), invalid_header);
}

#[test]
fn targets_must_be_regular_files_and_metadata_errors_surface() {
    let directory = scratch("target-dir");
    std::fs::create_dir_all(&directory).expect("create directory");
    assert_eq!(
        validate_target(&directory),
        Err(DurableAuditError::UnsafeFileType)
    );
    let file = directory.join("regular");
    std::fs::write(&file, b"x").expect("write file");
    assert_eq!(validate_target(&file), Ok(()));
    assert_eq!(validate_target(&directory.join("missing")), Ok(()));
    // Any metadata error other than NotFound surfaces. A NUL byte is refused
    // as InvalidInput on every platform.
    assert_eq!(
        validate_target(&directory.join("bad\0name")),
        Err(DurableAuditError::Io {
            operation: "metadata",
            kind: io::ErrorKind::InvalidInput,
        })
    );
    // Unix reports a path below a regular file as ENOTDIR. Windows reports it
    // as not found, so it passes as a missing target and the open fails.
    #[cfg(unix)]
    assert_eq!(
        validate_target(&file.join("child")),
        Err(DurableAuditError::Io {
            operation: "metadata",
            kind: io::ErrorKind::NotADirectory,
        })
    );
    #[cfg(windows)]
    {
        assert_eq!(validate_target(&file.join("child")), Ok(()));
        assert!(matches!(
            open_audit_file(&file.join("child")),
            Err(DurableAuditError::Io {
                operation: "open",
                ..
            })
        ));
    }
    std::fs::remove_dir_all(&directory).expect("remove directory");
}

#[test]
fn a_failed_write_is_rolled_back_or_marks_the_writer_unhealthy() {
    let path = scratch("recover");
    std::fs::write(&path, b"header-and-partial").expect("write file");
    let failure = io::Error::new(io::ErrorKind::WriteZero, "fixture");

    // The handle the writer itself opens: O_APPEND on Unix, and on Windows
    // one with the write access that truncation needs.
    let mut writable = AuditState {
        file: open_audit_file(&path).expect("open"),
        bytes: 6,
        records: 0,
        healthy: true,
    };
    assert_eq!(
        recover_partial_write(&mut writable, 6, &failure),
        Err(DurableAuditError::Io {
            operation: "append",
            kind: io::ErrorKind::WriteZero,
        })
    );
    assert!(writable.healthy);
    assert_eq!(std::fs::read(&path).expect("read file"), b"header");

    // A read-only handle cannot truncate, so recovery fails closed.
    let mut read_only = AuditState {
        file: File::open(&path).expect("open read-only"),
        bytes: 6,
        records: 0,
        healthy: true,
    };
    assert_eq!(
        recover_partial_write(&mut read_only, 3, &failure),
        Err(DurableAuditError::RecoveryFailed)
    );
    assert!(!read_only.healthy);
    std::fs::remove_file(&path).expect("remove file");
}

#[test]
fn a_partial_write_on_the_opened_file_is_rolled_back_and_the_next_frame_follows() {
    // Through the real open path, so the handle has the access the writer
    // gets on this platform (Windows needs write access to truncate).
    let path = scratch("rollback-open");
    let log = DurableAuditLog::<Note>::try_open(&path).expect("open");
    log.append(note(3)).expect("first append");
    {
        let mut state = log.state.lock().expect("state");
        let previous_bytes = state.bytes;
        // What a write that failed half-way through a frame leaves behind.
        state
            .file
            .write_all(b"0000002a:partial")
            .expect("partial write");
        let failure = io::Error::new(io::ErrorKind::WriteZero, "fixture");
        assert_eq!(
            recover_partial_write(&mut state, previous_bytes, &failure),
            Err(DurableAuditError::Io {
                operation: "append",
                kind: io::ErrorKind::WriteZero,
            })
        );
        assert!(state.healthy);
        let length = std::fs::metadata(&path).expect("metadata").len();
        assert_eq!(length, previous_bytes);
    }
    // The next frame starts where the rolled-back one did, without a gap.
    log.append(note(4)).expect("append after the rollback");
    drop(log);
    let reopened = DurableAuditLog::<Note>::try_open(&path).expect("reopen");
    assert_eq!(reopened.entries().expect("entries"), vec![note(3), note(4)]);
    drop(reopened);
    std::fs::remove_file(&path).expect("remove log");
}
