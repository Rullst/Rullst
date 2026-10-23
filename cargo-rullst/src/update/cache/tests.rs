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
    lock.unlock().unwrap();
    drop(lock);
    store_at(&base, b"new", 101).unwrap();
    assert_eq!(load_at(&base, 101).unwrap().body, b"new");
}

#[test]
fn explicit_unlock_releases_a_lock_even_while_a_duplicate_handle_survives() {
    let temp = fixture();
    let path = temp.path().join("lock");
    let owner = options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    owner.try_lock().unwrap();
    let duplicate = owner.try_clone().unwrap();
    let contender = options().read(true).write(true).open(&path).unwrap();
    assert!(matches!(
        contender.try_lock(),
        Err(fs::TryLockError::WouldBlock)
    ));
    {
        let _release = super::super::UnlockOnDrop(&owner);
    }
    contender.try_lock().unwrap();
    contender.unlock().unwrap();
    drop(duplicate);
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

#[test]
fn a_cache_file_one_byte_over_its_bound_is_refused_before_decoding() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, &vec![b'x'; CATALOG_LIMIT as usize], 100).unwrap();
    let file = options()
        .write(true)
        .open(base.join("rullst-update-v1").join(CATALOG))
        .unwrap();
    file.set_len(256 * 1024 + 64 + 1).unwrap();
    assert!(matches!(
        load_at(&base, 100),
        Err(CacheError::Invalid("catalog cache exceeds its size limit"))
    ));
}

#[test]
fn cache_timestamps_have_at_most_20_digits() {
    let entry = |timestamp: &str| [HEADER, timestamp.as_bytes(), b"\n", BODY].concat();
    assert_eq!(
        decode(&entry("00000000000000000100"), 100)
            .unwrap()
            .age_seconds,
        0
    );
    assert!(matches!(
        decode(&entry("000000000000000000100"), 100),
        Err(CacheError::Invalid("invalid catalog cache timestamp"))
    ));
}

/// Runs `base_directory` in a child test process with its own environment.
#[test]
fn base_directory_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_CACHE_BASE_REPORT") else {
        return;
    };
    let create = std::env::var_os("RULLST_CACHE_BASE_CREATE").is_some();
    let text = match std::env::var("RULLST_CACHE_OPERATION").as_deref() {
        Ok("store") => store(BODY, 100).map(|()| String::new()),
        Ok("workspace") => project_workspace().map(|path| path.display().to_string()),
        _ => base_directory(create).map(|path| path.display().to_string()),
    };
    let text = match text {
        Ok(text) => format!("ok {text}"),
        Err(error) => format!("err {error}"),
    };
    fs::write(report, text).unwrap();
}

fn base_directory_in_child(environment: &[(&str, &std::ffi::OsStr)], create: bool) -> String {
    let temp = fixture();
    let report = temp.path().join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "update::cache::platform::tests::base_directory_child_fixture",
            "--nocapture",
        ])
        .env("RULLST_CACHE_BASE_REPORT", &report)
        .env_remove("XDG_CACHE_HOME")
        .env_remove("RULLST_CACHE_BASE_CREATE")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());
    for (name, value) in environment {
        command.env(name, value);
    }
    if create {
        command.env("RULLST_CACHE_BASE_CREATE", "1");
    }
    assert!(command.status().unwrap().success());
    fs::read_to_string(report).unwrap()
}

#[test]
fn the_cache_base_comes_from_xdg_or_home_and_is_created_only_on_request() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    let xdg = base.join("xdg");
    let selected = [("XDG_CACHE_HOME", xdg.as_os_str())];
    assert!(base_directory_in_child(&selected, false).starts_with("err "));
    assert!(!xdg.exists());
    assert_eq!(
        base_directory_in_child(&selected, true),
        format!("ok {}", xdg.display())
    );
    assert_eq!(
        fs::metadata(&xdg).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let home = base.join("home");
    fs::create_dir(&home).unwrap();
    fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        base_directory_in_child(
            &[
                ("XDG_CACHE_HOME", std::ffi::OsStr::new("")),
                ("HOME", home.as_os_str()),
            ],
            true
        ),
        format!("ok {}", home.join(".cache").display())
    );
}

#[test]
fn cache_paths_must_be_absolute_without_traversal() {
    validate_absolute(Path::new("/var/cache")).unwrap();
    for path in ["relative/cache", "/var/../cache"] {
        assert!(validate_absolute(Path::new(path)).is_err(), "{path}");
    }
}

#[test]
fn cache_directories_are_owned_directories() {
    let temp = fixture();
    let file = temp.path().join("file");
    fs::write(&file, b"x").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(validate_directory(&file, false).is_err());
    validate_directory(temp.path(), true).unwrap();
}

#[test]
fn creating_a_private_directory_reports_errors_other_than_existence() {
    let temp = fixture();
    let path = temp.path().join("private");
    create_private_directory(&path).unwrap();
    create_private_directory(&path).unwrap();
    assert!(create_private_directory(&temp.path().join("missing").join("child")).is_err());
}

#[test]
fn ancestors_must_be_directories_without_other_writers() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    validate_ancestors(&base).unwrap();
    let file = base.join("file");
    fs::write(&file, b"x").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(validate_ancestors(&file).is_err());
    // Only root's sticky directories may be group or world writable.
    let sticky = base.join("sticky");
    fs::create_dir(&sticky).unwrap();
    fs::set_permissions(&sticky, fs::Permissions::from_mode(0o1770)).unwrap();
    assert!(validate_ancestors(&sticky).is_err());
    let shared = Path::new("/var/mail");
    if let Ok(metadata) = fs::symlink_metadata(shared)
        && metadata.is_dir()
        && metadata.uid() == 0
        && metadata.mode() & 0o022 != 0
        && metadata.mode() & 0o1000 == 0
    {
        assert!(validate_ancestors(shared).is_err());
    }
}

#[test]
fn installation_roots_report_lookup_errors_other_than_absence() {
    if rustix::process::geteuid().is_root() {
        return;
    }
    let temp = fixture();
    let parent = temp.path().canonicalize().unwrap().join("unsearchable");
    fs::create_dir(&parent).unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o600)).unwrap();
    let result = installation_root(&parent.join("installed"));
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
}

#[test]
fn stores_and_project_workspaces_live_in_the_private_cache() {
    let temp = fixture();
    let xdg = temp.path().canonicalize().unwrap().join("xdg");
    let operation = |name: &str| {
        base_directory_in_child(
            &[
                ("XDG_CACHE_HOME", xdg.as_os_str()),
                ("RULLST_CACHE_OPERATION", std::ffi::OsStr::new(name)),
            ],
            false,
        )
    };
    assert_eq!(operation("store"), "ok ");
    assert_eq!(load_at(&xdg, 101).unwrap().body, BODY);
    let workspace = PathBuf::from(operation("workspace").strip_prefix("ok ").unwrap());
    assert!(workspace.starts_with(xdg.join("rullst-update-v1")));
    assert_eq!(
        fs::metadata(&workspace).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[test]
fn a_cache_file_of_exactly_its_bound_is_read_and_then_decoded() {
    let temp = fixture();
    let base = temp.path().canonicalize().unwrap();
    store_at(&base, BODY, 100).unwrap();
    let file = options()
        .write(true)
        .open(base.join("rullst-update-v1").join(CATALOG))
        .unwrap();
    file.set_len(256 * 1024 + 64).unwrap();
    assert!(matches!(
        load_at(&base, 100),
        Err(CacheError::Invalid("invalid cached catalog size"))
    ));
}
