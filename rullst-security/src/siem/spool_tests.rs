use super::*;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_spool(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-security-siem-{label}-{}-{nonce}.spool",
        std::process::id()
    ))
}

#[test]
fn restart_replays_exact_normalized_unsigned_events() {
    let path = temporary_spool("restart");
    let first_offset;
    {
        let spool = DurableSiemSpool::try_open(&path).expect("create spool");
        let mut event = LiveSecurityEvent::local("RBAC_DENIAL", "denied", "192.0.2.1");
        event.verified_hmac = true;
        let receipt = spool.append_local(event).expect("append first event");
        assert_eq!(receipt.sequence(), 1);
        first_offset = receipt.end_offset();
        let second = spool
            .append_local(LiveSecurityEvent::local(
                "RASP_PAYLOAD_INTERCEPTED",
                "body=true",
                "invalid-ip",
            ))
            .expect("append second event");
        assert_eq!(second.sequence(), 2);
        assert!(second.end_offset() > first_offset);
    }

    let reopened = DurableSiemSpool::try_open(&path).expect("reopen valid spool");
    let events = reopened.read_local().expect("read valid events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "RBAC_DENIAL");
    assert!(!events[0].verified_hmac);
    assert_eq!(events[1].client_ip, "unknown");
    assert_eq!(reopened.snapshot().expect("snapshot").records(), 2);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn quota_rejection_preserves_the_previous_file() {
    let path = temporary_spool("quota");
    let spool =
        DurableSiemSpool::try_open_with_max_bytes(&path, 1_024).expect("create bounded spool");
    spool
        .append_local(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "x".repeat(600),
            "192.0.2.2",
        ))
        .expect("first event fits");
    let before = spool.snapshot().expect("snapshot before rejection");
    assert_eq!(
        spool.append_local(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "y".repeat(600),
            "192.0.2.3",
        )),
        Err(SiemSpoolError::CapacityExceeded)
    );
    assert_eq!(spool.snapshot().expect("snapshot after rejection"), before);
    assert_eq!(spool.read_local().expect("read original").len(), 1);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn corruption_and_external_changes_fail_closed() {
    let path = temporary_spool("corrupt");
    let spool = DurableSiemSpool::try_open(&path).expect("create spool");
    spool
        .append_local(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "original",
            "192.0.2.4",
        ))
        .expect("append event");
    let mut external = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open spool externally");
    external.write_all(b"broken\n").expect("append corruption");
    external.sync_data().expect("sync corruption");
    assert_eq!(
        spool.read_local(),
        Err(SiemSpoolError::ExternalModification)
    );
    drop(spool);

    assert!(matches!(
        DurableSiemSpool::try_open(&path),
        Err(SiemSpoolError::CorruptRecord { record: 2, .. })
    ));
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn same_length_payload_tampering_is_detected_on_restart() {
    let path = temporary_spool("digest");
    {
        let spool = DurableSiemSpool::try_open(&path).expect("create spool");
        spool
            .append_local(LiveSecurityEvent::local(
                "SECURITY_EVENT",
                "original",
                "192.0.2.5",
            ))
            .expect("append event");
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("open spool for tampering");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read spool");
    let position = bytes
        .windows(b"original".len())
        .position(|window| window == b"original")
        .expect("fixture payload should exist");
    bytes[position] = b'O';
    file.seek(SeekFrom::Start(0)).expect("seek spool");
    file.write_all(&bytes).expect("rewrite same-length spool");
    file.sync_data().expect("sync tampering");
    drop(file);

    assert!(matches!(
        DurableSiemSpool::try_open(&path),
        Err(SiemSpoolError::CorruptRecord {
            record: 1,
            reason: "digest mismatch"
        })
    ));
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn concurrent_in_process_appends_are_serialized() {
    let path = temporary_spool("concurrency");
    let spool = Arc::new(DurableSiemSpool::try_open(&path).expect("create spool"));
    let handles = (0..32)
        .map(|index| {
            let spool = Arc::clone(&spool);
            std::thread::spawn(move || {
                spool
                    .append_local(LiveSecurityEvent::local(
                        "SECURITY_EVENT",
                        format!("event-{index}"),
                        "192.0.2.6",
                    ))
                    .expect("concurrent append")
                    .sequence()
            })
        })
        .collect::<Vec<_>>();
    let mut sequences = handles
        .into_iter()
        .map(|handle| handle.join().expect("append thread"))
        .collect::<Vec<_>>();
    sequences.sort_unstable();
    assert_eq!(sequences, (1..=32).collect::<Vec<_>>());
    assert_eq!(spool.read_local().expect("read events").len(), 32);
    assert_eq!(spool.snapshot().expect("snapshot").records(), 32);
    drop(spool);
    std::fs::remove_file(path).expect("remove test spool");
}

#[cfg(unix)]
#[test]
fn symlink_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let target = temporary_spool("target");
    let link = temporary_spool("link");
    File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        DurableSiemSpool::try_open(&link),
        Err(SiemSpoolError::UnsafeFileType)
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");
}

/// One encoded frame without its trailing newline, as `decode_frame` sees it.
fn frame_for(payload: &[u8]) -> Vec<u8> {
    let mut frame = encode_frame(payload).expect("frame within the record limit");
    frame.pop();
    frame
}

#[test]
fn quota_bounds_span_512_bytes_to_16_mib() {
    let path = temporary_spool("quota-bounds");
    for (max_bytes, accepted) in [
        (511, false),
        (512, true),
        (16 * 1024 * 1024, true),
        (16 * 1024 * 1024 + 1, false),
    ] {
        let opened = DurableSiemSpool::try_open_with_max_bytes(&path, max_bytes);
        assert_eq!(opened.is_ok(), accepted, "{max_bytes} bytes");
    }
    let default = DurableSiemSpool::try_open(&path).expect("default quota");
    let snapshot = default.snapshot().expect("snapshot");
    assert_eq!(snapshot.max_bytes(), 16 * 1024 * 1024);
    assert_eq!(snapshot.bytes(), SPOOL_MAGIC.len() as u64);
    assert_eq!(snapshot.records(), 0);
    drop(default);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn appends_may_fill_the_quota_exactly_and_reopen_at_that_size() {
    let path = temporary_spool("exact-fit");
    let event = LiveSecurityEvent::local("SECURITY_EVENT", "x".repeat(600), "192.0.2.10");
    let frame = encode_frame(&serde_json::to_vec(&event).expect("encode event")).expect("frame");
    let max_bytes = (SPOOL_MAGIC.len() + frame.len()) as u64;

    let spool = DurableSiemSpool::try_open_with_max_bytes(&path, max_bytes).expect("create");
    let receipt = spool
        .append_local(event)
        .expect("frame fills the quota exactly");
    assert_eq!(receipt.end_offset(), max_bytes);
    let snapshot = spool.snapshot().expect("snapshot");
    assert_eq!(snapshot.bytes(), max_bytes);
    assert_eq!(snapshot.max_bytes(), max_bytes);
    drop(spool);

    let reopened =
        DurableSiemSpool::try_open_with_max_bytes(&path, max_bytes).expect("full spool reopens");
    assert_eq!(reopened.read_local().expect("read full spool").len(), 1);
    drop(reopened);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn records_hold_payloads_up_to_sixteen_kib() {
    let limit = 16 * 1024;
    assert!(encode_frame(&vec![b'x'; limit]).is_ok());
    assert_eq!(
        encode_frame(&vec![b'x'; limit + 1]),
        Err(SiemSpoolError::RecordTooLarge)
    );

    // A frame announcing exactly the limit passes the length check.
    let at_limit = frame_for(&vec![b'x'; limit]);
    assert_eq!(
        decode_frame(&at_limit, 1),
        Err(corrupt(1, "invalid event JSON"))
    );
    let oversized = vec![b'x'; limit + 1];
    let mut over_limit =
        format!("{:08x}:{}:", oversized.len(), sha256_hex(&oversized)).into_bytes();
    over_limit.extend_from_slice(&oversized);
    assert_eq!(
        decode_frame(&over_limit, 1),
        Err(corrupt(1, "record length exceeds limit"))
    );
}

#[test]
fn frame_headers_need_both_delimiters_and_a_full_prefix() {
    let event = LiveSecurityEvent::local("SECURITY_EVENT", "framed", "192.0.2.11");
    let frame = frame_for(&serde_json::to_vec(&event).expect("encode event"));
    assert_eq!(decode_frame(&frame, 1), Ok(event));

    let header = corrupt(1, "invalid frame header");
    for delimiter in [LENGTH_HEX_BYTES, LENGTH_HEX_BYTES + 1 + DIGEST_HEX_BYTES] {
        let mut damaged = frame.clone();
        damaged[delimiter] = b'0';
        assert_eq!(decode_frame(&damaged, 1), Err(header.clone()));
    }

    // A bare prefix announcing an empty payload passes the header check.
    let empty = frame_for(b"");
    assert_eq!(empty.len(), FRAME_PREFIX_BYTES);
    assert_eq!(
        decode_frame(&empty, 1),
        Err(corrupt(1, "invalid event JSON"))
    );
    assert_eq!(
        decode_frame(&empty[..FRAME_PREFIX_BYTES - 1], 1),
        Err(header)
    );
}

#[test]
fn stored_events_must_already_be_normalized_and_unsigned() {
    let mut event = LiveSecurityEvent::local("SECURITY_EVENT", "claimed", "192.0.2.12");
    event.verified_hmac = true;
    let frame = frame_for(&serde_json::to_vec(&event).expect("encode event"));
    assert_eq!(
        decode_frame(&frame, 3),
        Err(corrupt(3, "event violates the local v1 contract"))
    );
}

#[test]
fn an_unterminated_frame_names_its_record() {
    let path = temporary_spool("unterminated");
    {
        let spool = DurableSiemSpool::try_open(&path).expect("create spool");
        spool
            .append_local(LiveSecurityEvent::local(
                "SECURITY_EVENT",
                "complete",
                "192.0.2.13",
            ))
            .expect("append event");
    }
    let mut external = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open spool externally");
    external
        .write_all(b"partial")
        .expect("append partial frame");
    drop(external);

    assert_eq!(
        DurableSiemSpool::try_open(&path).err(),
        Some(corrupt(2, "truncated frame"))
    );
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn directories_are_not_spool_files() {
    let directory = temporary_spool("directory");
    std::fs::create_dir(&directory).expect("create directory");
    assert_eq!(
        DurableSiemSpool::try_open(&directory).err(),
        Some(SiemSpoolError::UnsafeFileType)
    );
    std::fs::remove_dir(directory).expect("remove directory");
}

#[cfg(unix)]
#[test]
fn metadata_failures_other_than_absence_are_reported() {
    let parent = temporary_spool("not-a-directory");
    File::create(&parent).expect("create regular file");
    assert_eq!(
        DurableSiemSpool::try_open(parent.join("child.spool")).err(),
        Some(SiemSpoolError::Io {
            operation: "metadata",
            kind: io::ErrorKind::NotADirectory,
        })
    );
    std::fs::remove_file(parent).expect("remove regular file");
}

#[test]
fn a_failed_rollback_marks_the_writer_unhealthy() {
    let path = temporary_spool("rollback");
    std::fs::write(&path, SPOOL_MAGIC).expect("write header");
    // A read-only handle cannot truncate, so the rollback itself fails.
    let mut state = SpoolState {
        file: File::open(&path).expect("open read-only"),
        bytes: SPOOL_MAGIC.len() as u64,
        records: 0,
        healthy: true,
    };
    assert_eq!(
        recover_partial_write(&mut state, 0, &io::Error::other("disk full")),
        Err(SiemSpoolError::RecoveryFailed)
    );
    assert!(!state.healthy);
    drop(state);
    std::fs::remove_file(path).expect("remove test spool");
}
