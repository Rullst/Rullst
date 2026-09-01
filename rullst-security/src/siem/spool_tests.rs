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
