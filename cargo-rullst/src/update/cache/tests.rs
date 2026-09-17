use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

const BODY: &[u8] = br#"{"versions":[]}"#;

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    temp
}

#[test]
fn roundtrip_is_private_bounded_and_atomic() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let first = load_at(&base, 200).unwrap();
    assert_eq!(first.body, BODY);
    assert_eq!(first.age_seconds, 100);
    let directory = base.join("rullst-update-v1");
    assert_eq!(fs::metadata(&directory).unwrap().mode() & 0o077, 0);
    assert_eq!(
        fs::metadata(directory.join(CATALOG)).unwrap().mode() & 0o077,
        0
    );
    store_at(&base, b"new catalog", 201).unwrap();
    assert_eq!(load_at(&base, 201).unwrap().body, b"new catalog");
    assert_eq!(fs::read_dir(directory).unwrap().count(), 2);
}

#[test]
fn missing_cache_reads_create_nothing() {
    let temp = fixture();
    assert!(load_at(temp.path(), 100).is_err());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn expiration_future_and_bad_headers_fail_closed() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    assert!(load_at(&base, 99).is_err());
    assert!(load_at(&base, 100 + MAX_AGE_SECONDS).is_err());
    assert!(load_at(&base, 100 + MAX_AGE_SECONDS - 1).is_ok());
    for bad in [
        b"legacy-format".as_slice(),
        b"rullst-update-cache-v1\n-1\n{}",
        b"rullst-update-cache-v1\n+1\n{}",
        b"rullst-update-cache-v1\n18446744073709551616\n{}",
        b"rullst-update-cache-v1\n1\n",
        b"rullst-update-cache-v1\n\n{}",
    ] {
        assert!(decode(bad, 100).is_err());
    }
}

#[test]
fn non_private_directories_are_never_chmodded_or_used() {
    for mode in [0o755, 0o750, 0o777] {
        let temp = fixture();
        let base = temp.path().canonicalize().unwrap();
        let directory = base.join("rullst-update-v1");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(mode)).unwrap();
        assert!(store_at(&base, BODY, 100).is_err());
        assert_eq!(fs::metadata(&directory).unwrap().mode() & 0o777, mode);
        assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
    }
}

#[test]
fn writable_parent_and_ancestor_are_rejected() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    let child = base.join("child");
    create_private_directory(&child).unwrap();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(store_at(&base, BODY, 100).is_err());
    assert!(store_at(&child, BODY, 100).is_err());
    // Restore only this test-owned directory before TempDir cleanup.
    fs::set_permissions(base, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn directory_and_file_symlinks_are_not_read_or_followed() {
    let temp = fixture();
    let outside = fixture();
    let base = temp.path().canonicalize().unwrap();
    let directory = base.join("rullst-update-v1");
    symlink(outside.path(), &directory).unwrap();
    assert!(store_at(&base, BODY, 100).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::remove_file(&directory).unwrap();
    store_at(&base, BODY, 100).unwrap();
    let saved = outside.path().join("original");
    fs::rename(directory.join(CATALOG), &saved).unwrap();
    symlink(&saved, directory.join(CATALOG)).unwrap();
    assert!(load_at(&base, 100).is_err());
    // Replacement never follows an existing link or truncates its destination.
    store_at(&base, b"replacement", 101).unwrap();
    assert_eq!(decode(&fs::read(saved).unwrap(), 100).unwrap().body, BODY);
    assert_eq!(load_at(&base, 101).unwrap().body, b"replacement");
}

#[test]
fn hardlinked_or_world_readable_catalog_is_not_trusted() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let path = base.join("rullst-update-v1").join(CATALOG);
    fs::hard_link(&path, base.join("linked")).unwrap();
    assert!(load_at(&base, 100).is_err());
    store_at(&base, BODY, 101).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load_at(&base, 101).is_err());
}

#[test]
fn readers_keep_old_snapshot_and_busy_writer_fails_without_waiting() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let directory = base.join("rullst-update-v1");
    let lock = options()
        .read(true)
        .write(true)
        .open(directory.join("catalog.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(store_at(&base, b"new", 101).is_err());
    assert_eq!(load_at(&base, 101).unwrap().body, BODY);
    drop(lock);
    store_at(&base, b"new", 101).unwrap();
    assert_eq!(load_at(&base, 101).unwrap().body, b"new");
}

#[test]
fn linked_lock_and_failed_publication_leave_previous_catalog_intact() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let directory = base.join("rullst-update-v1");
    fs::remove_file(directory.join("catalog.lock")).unwrap();
    symlink(directory.join(CATALOG), directory.join("catalog.lock")).unwrap();
    assert!(store_at(&base, b"new", 101).is_err());
    assert_eq!(load_at(&base, 101).unwrap().body, BODY);
    fs::remove_file(directory.join("catalog.lock")).unwrap();
    fs::rename(directory.join(CATALOG), base.join("old")).unwrap();
    fs::create_dir(directory.join(CATALOG)).unwrap();
    assert!(store_at(&base, BODY, 101).is_err());
    assert_eq!(
        decode(&fs::read(base.join("old")).unwrap(), 100)
            .unwrap()
            .body,
        BODY
    );
    assert!(!fs::read_dir(directory).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("catalog-stage-")
    }));
}

#[test]
fn file_and_body_size_limits_precede_allocation_and_writes() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    assert!(store_at(&base, &[], 100).is_err());
    assert!(store_at(&base, &vec![b'x'; CATALOG_LIMIT as usize + 1], 100).is_err());
    assert!(!base.join("rullst-update-v1").exists());
    store_at(&base, &vec![b'x'; CATALOG_LIMIT as usize], 100).unwrap();
    assert_eq!(
        load_at(&base, 100).unwrap().body.len(),
        CATALOG_LIMIT as usize
    );
    let file = options()
        .write(true)
        .open(base.join("rullst-update-v1").join(CATALOG))
        .unwrap();
    file.set_len(FILE_LIMIT + 1).unwrap();
    assert!(load_at(&base, 100).is_err());
}

#[test]
fn fifo_catalog_and_lock_are_rejected_without_waiting_for_a_peer() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let directory = base.join("rullst-update-v1");
    let catalog = directory.join(CATALOG);
    fs::remove_file(&catalog).unwrap();
    create_test_fifo(&catalog);
    assert!(load_at(&base, 100).is_err());
    let lock = directory.join("catalog.lock");
    fs::remove_file(&lock).unwrap();
    create_test_fifo(&lock);
    assert!(store_at(&base, BODY, 100).is_err());
}

fn create_test_fifo(path: &Path) {
    // rustix::fs::mkfifoat is unavailable on macOS. The POSIX utility creates
    // the same adversarial fixture on both supported Unix CI platforms.
    assert!(
        std::process::Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(path)
            .status()
            .expect("POSIX mkfifo is required by the Unix test environment")
            .success()
    );
}
