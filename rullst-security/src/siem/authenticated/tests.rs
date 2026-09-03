use super::*;
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

fn temporary_spool(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-security-auth-siem-{label}-{}-{nonce}.spool",
        std::process::id()
    ))
}

fn key(id: &str, offset: u8) -> SiemIntegrityKey {
    let material = (0_u8..32)
        .map(|byte| byte.wrapping_add(offset))
        .collect::<Vec<_>>();
    SiemIntegrityKey::try_new(id, material).expect("valid test key")
}

fn ring(active_id: &str, offset: u8) -> SiemKeyRing {
    SiemKeyRing::try_new(key(active_id, offset), []).expect("valid test ring")
}

#[test]
fn restart_and_explicit_rotation_verify_every_record() {
    let path = temporary_spool("rotation");
    {
        let spool = AuthenticatedSiemSpool::try_open(&path, ring("key-2026-a", 0))
            .expect("create authenticated spool");
        let mut event = LiveSecurityEvent::local("RBAC_DENIAL", "owner mismatch", "192.0.2.1");
        event.verified_hmac = true;
        let receipt = spool.append_local(event).expect("append first event");
        assert_eq!(receipt.sequence(), 1);
        assert!(receipt.end_offset() > codec::SPOOL_MAGIC.len() as u64);
    }

    let rotated_ring =
        SiemKeyRing::try_new(key("key-2026-b", 32), [key("key-2026-a", 0)]).expect("rotation ring");
    {
        let spool = AuthenticatedSiemSpool::try_open(&path, rotated_ring)
            .expect("reopen with historical key");
        assert_eq!(spool.snapshot().expect("snapshot").records(), 1);
        let receipt = spool
            .append_local(LiveSecurityEvent::local(
                "RASP_PAYLOAD_INTERCEPTED",
                "bounded payload rejected",
                "192.0.2.2",
            ))
            .expect("append using rotated key");
        assert_eq!(receipt.sequence(), 2);
        let events = spool.read_verified().expect("verify both rotations");
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|event| event.verified_hmac));
    }

    let final_ring = SiemKeyRing::try_new(key("key-2026-b", 32), [key("key-2026-a", 0)])
        .expect("final rotation ring");
    let reopened = AuthenticatedSiemSpool::try_open(&path, final_ring).expect("final reopen");
    assert_eq!(reopened.read_verified().expect("final verify").len(), 2);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn tampering_wrong_key_and_missing_rotation_key_fail_closed() {
    // TM-SEC-33
    let path = temporary_spool("tamper");
    {
        let spool = AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
            .expect("create authenticated spool");
        spool
            .append_local(LiveSecurityEvent::local(
                "SECURITY_EVENT",
                "original",
                "192.0.2.3",
            ))
            .expect("append event");
    }

    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("replacement", 32)),
        Err(AuthenticatedSiemSpoolError::UnknownKey { record: 1 })
    ));
    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 32)),
        Err(AuthenticatedSiemSpoolError::AuthenticationFailed { record: 1 })
    ));

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("open spool for tamper");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read spool");
    let position = bytes
        .windows(b"original".len())
        .position(|window| window == b"original")
        .expect("fixture payload");
    bytes[position] = b'O';
    file.seek(SeekFrom::Start(0)).expect("seek spool");
    file.write_all(&bytes).expect("rewrite tampered spool");
    file.sync_data().expect("sync tamper");
    drop(file);

    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 0)),
        Err(AuthenticatedSiemSpoolError::AuthenticationFailed { record: 1 })
    ));
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn sequence_and_predecessor_chain_reject_record_reordering_or_removal() {
    let path = temporary_spool("chain");
    {
        let spool = AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
            .expect("create authenticated spool");
        for index in 0..3 {
            spool
                .append_local(LiveSecurityEvent::local(
                    "SECURITY_EVENT",
                    format!("event-{index}"),
                    "192.0.2.4",
                ))
                .expect("append event");
        }
    }

    let original = std::fs::read(&path).expect("read original spool");
    let mut frames = original[codec::SPOOL_MAGIC.len()..]
        .split_inclusive(|byte| *byte == b'\n')
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 3);
    frames.swap(0, 1);
    let mut reordered = codec::SPOOL_MAGIC.to_vec();
    reordered.extend(frames.into_iter().flatten());
    std::fs::write(&path, reordered).expect("write reordered spool");
    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 0)),
        Err(AuthenticatedSiemSpoolError::CorruptRecord { record: 1, .. })
    ));

    let original_frames = original[codec::SPOOL_MAGIC.len()..]
        .split_inclusive(|byte| *byte == b'\n')
        .collect::<Vec<_>>();
    let mut removed_middle = codec::SPOOL_MAGIC.to_vec();
    removed_middle.extend_from_slice(original_frames[0]);
    removed_middle.extend_from_slice(original_frames[2]);
    std::fs::write(&path, removed_middle).expect("write missing-middle spool");
    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 0)),
        Err(AuthenticatedSiemSpoolError::CorruptRecord { record: 2, .. })
    ));
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn partial_tail_fails_closed_while_a_valid_prefix_needs_an_external_checkpoint() {
    let path = temporary_spool("tail-boundary");
    {
        let spool = AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
            .expect("create authenticated spool");
        for details in ["first", "second"] {
            spool
                .append_local(LiveSecurityEvent::local(
                    "SECURITY_EVENT",
                    details,
                    "192.0.2.7",
                ))
                .expect("append event");
        }
    }
    let original = std::fs::read(&path).expect("read complete spool");
    std::fs::write(&path, &original[..original.len() - 1]).expect("truncate final byte");
    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 0)),
        Err(AuthenticatedSiemSpoolError::CorruptRecord { record: 2, .. })
    ));

    let first_frame_end = codec::SPOOL_MAGIC.len()
        + original[codec::SPOOL_MAGIC.len()..]
            .iter()
            .position(|byte| *byte == b'\n')
            .expect("first frame terminator")
        + 1;
    std::fs::write(&path, &original[..first_frame_end]).expect("retain valid prefix");
    let valid_prefix = AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
        .expect("valid prefix cannot be distinguished without checkpoint");
    assert_eq!(
        valid_prefix.read_verified().expect("verify prefix").len(),
        1
    );
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn concurrent_appends_receive_one_complete_sequence() {
    let path = temporary_spool("concurrency");
    let spool = Arc::new(
        AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
            .expect("create authenticated spool"),
    );
    let handles = (0..32)
        .map(|index| {
            let spool = Arc::clone(&spool);
            std::thread::spawn(move || {
                spool
                    .append_local(LiveSecurityEvent::local(
                        "SECURITY_EVENT",
                        format!("event-{index}"),
                        "192.0.2.8",
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
    assert_eq!(spool.read_verified().expect("verify journal").len(), 32);
    drop(spool);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn quotas_external_changes_and_secret_debug_output_are_bounded() {
    let path = temporary_spool("bounds");
    let secret = (0_u8..32).collect::<Vec<_>>();
    let integrity_key = SiemIntegrityKey::try_new("debug-key", secret.clone()).expect("valid key");
    let debug = format!("{integrity_key:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains(&format!("{secret:?}")));
    assert!(SiemIntegrityKey::try_new("bad:key", secret.clone()).is_err());
    assert!(SiemIntegrityKey::try_new("weak", vec![7; 32]).is_err());

    let spool = AuthenticatedSiemSpool::try_open_with_max_bytes(
        &path,
        SiemKeyRing::try_new(integrity_key, []).expect("valid ring"),
        700,
    )
    .expect("create bounded spool");
    spool
        .append_local(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "first",
            "192.0.2.5",
        ))
        .expect("first event fits");
    assert_eq!(
        spool.append_local(LiveSecurityEvent::local(
            "SECURITY_EVENT",
            "x".repeat(500),
            "192.0.2.6",
        )),
        Err(AuthenticatedSiemSpoolError::CapacityExceeded)
    );

    let mut external = OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("external append handle");
    external.write_all(b"forged\n").expect("external append");
    external.sync_data().expect("external sync");
    assert_eq!(
        spool.read_verified(),
        Err(AuthenticatedSiemSpoolError::ExternalModification)
    );
    drop(spool);
    std::fs::remove_file(path).expect("remove test spool");
}

#[test]
fn key_ring_rejects_duplicates_and_excess_historical_material() {
    assert!(matches!(
        SiemKeyRing::try_new(key("same", 0), [key("same", 32)]),
        Err(AuthenticatedSiemSpoolError::DuplicateKeyId)
    ));
    let historical = (0..MAX_SIEM_INTEGRITY_KEYS)
        .map(|index| key(&format!("old-{index}"), (index as u8).wrapping_add(1)))
        .collect::<Vec<_>>();
    assert!(matches!(
        SiemKeyRing::try_new(key("active", 64), historical),
        Err(AuthenticatedSiemSpoolError::TooManyKeys)
    ));
}

#[cfg(unix)]
#[test]
fn symlink_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let target = temporary_spool("target");
    let link = temporary_spool("link");
    std::fs::File::create(&target).expect("create target");
    symlink(&target, &link).expect("create symlink");
    assert!(matches!(
        AuthenticatedSiemSpool::try_open(&link, ring("primary", 0)),
        Err(AuthenticatedSiemSpoolError::UnsafeFileType)
    ));
    std::fs::remove_file(link).expect("remove link");
    std::fs::remove_file(target).expect("remove target");
}

#[cfg(unix)]
#[test]
fn newly_created_journal_is_owner_only() {
    use std::os::unix::fs::PermissionsExt as _;

    let path = temporary_spool("permissions");
    let spool = AuthenticatedSiemSpool::try_open(&path, ring("primary", 0))
        .expect("create authenticated spool");
    let mode = std::fs::metadata(&path)
        .expect("read journal metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    drop(spool);
    std::fs::remove_file(path).expect("remove journal");
}
