use super::*;
use tempfile::TempDir;

struct Fixture {
    directory: TempDir,
    backup: PathBuf,
}

#[cfg(unix)]
#[test]
fn root_alias_is_accepted_without_allowing_descendant_symlinks() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    let alias = outside.path().join("project-alias");
    symlink(fixture.root(), &alias).unwrap();
    let requested = alias.join("target/rullst-upgrades/test");
    restore_from(&fixture.root(), &requested).unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("Cargo.toml")).unwrap(),
        "previous manifest"
    );
    fs::write(fixture.root().join("Cargo.toml"), "current manifest").unwrap();
    fs::rename(
        fixture.root().join("target"),
        fixture.root().join("real-target"),
    )
    .unwrap();
    symlink("real-target", fixture.root().join("target")).unwrap();
    assert!(restore_from(&fixture.root(), &requested).is_err());
    fixture.unchanged_manifest();
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let backup = root.join("target/rullst-upgrades/test");
        fs::create_dir_all(backup.join("files/src")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("Cargo.toml"), "current manifest").unwrap();
        fs::write(root.join("src/main.rs"), "current source").unwrap();
        fs::write(backup.join("files/Cargo.toml"), "previous manifest").unwrap();
        fs::write(backup.join("files/src/main.rs"), "previous source").unwrap();
        let fixture = Self { directory, backup };
        fixture.index("present\tCargo.toml\npresent\tsrc/main.rs\nabsent\tCargo.lock\n");
        fixture
    }

    fn root(&self) -> PathBuf {
        self.directory.path().canonicalize().unwrap()
    }
    fn index(&self, content: &str) {
        fs::write(self.backup.join("index.tsv"), content).unwrap();
    }
    fn run(&self) -> Result<PathBuf, RestoreError> {
        restore_from(&self.root(), &self.backup)
    }
    fn unchanged_manifest(&self) {
        assert_eq!(
            fs::read_to_string(self.root().join("Cargo.toml")).unwrap(),
            "current manifest"
        );
    }
}

#[test]
fn valid_legacy_index_restores_files_and_only_the_absent_root_lock() {
    let fixture = Fixture::new();
    fs::write(fixture.root().join("Cargo.lock"), "new lock").unwrap();
    fixture.run().unwrap();
    assert_eq!(
        fs::read_to_string(fixture.root().join("Cargo.toml")).unwrap(),
        "previous manifest"
    );
    assert_eq!(
        fs::read_to_string(fixture.root().join("src/main.rs")).unwrap(),
        "previous source"
    );
    assert!(!fixture.root().join("Cargo.lock").exists());
    assert!(fixture.backup.join("files/Cargo.toml").is_file());
}

#[test]
fn malformed_late_entries_never_apply_an_earlier_valid_entry() {
    for invalid in [
        "broken",
        "other\tsrc/main.rs",
        "present\t../Cargo.toml",
        "absent\tsrc/main.rs",
        "present\tCargo.toml",
        "present\t.env",
        "present\ttarget/evil.rs",
        "present\t.git/evil.rs",
        "present\tsrc/missing.rs",
        "present\tsrc/main.rs\textra",
        "present\tC:evil.rs",
    ] {
        let fixture = Fixture::new();
        fixture.index(&format!("present\tCargo.toml\n{invalid}\n"));
        assert!(fixture.run().is_err(), "accepted {invalid:?}");
        fixture.unchanged_manifest();
    }
}

#[test]
fn empty_and_oversized_indexes_leave_originals_untouched() {
    let fixture = Fixture::new();
    fixture.index("");
    assert!(fixture.run().is_err());
    File::create(fixture.backup.join("index.tsv"))
        .unwrap()
        .set_len(MAX_INDEX_BYTES + 1)
        .unwrap();
    assert!(fixture.run().unwrap_err().to_string().contains("8 MiB"));
    fixture.unchanged_manifest();
}

#[test]
fn oversized_late_snapshot_is_rejected_before_first_restore() {
    let fixture = Fixture::new();
    File::create(fixture.backup.join("files/src/main.rs"))
        .unwrap()
        .set_len(MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(fixture.run().unwrap_err().to_string().contains("64 MiB"));
    fixture.unchanged_manifest();
}

#[test]
fn missing_parent_or_directory_destination_is_rejected_before_first_restore() {
    for missing_parent in [true, false] {
        let fixture = Fixture::new();
        fs::remove_file(fixture.root().join("src/main.rs")).unwrap();
        if missing_parent {
            fs::remove_dir(fixture.root().join("src")).unwrap();
        } else {
            fs::create_dir(fixture.root().join("src/main.rs")).unwrap();
        }
        assert!(fixture.run().is_err());
        fixture.unchanged_manifest();
    }
}

#[test]
fn hardlinked_destination_does_not_overwrite_an_external_inode() {
    let fixture = Fixture::new();
    let external = tempfile::tempdir().unwrap();
    let external_file = external.path().join("other.rs");
    fs::write(&external_file, "external source").unwrap();
    fs::remove_file(fixture.root().join("src/main.rs")).unwrap();
    fs::hard_link(&external_file, fixture.root().join("src/main.rs")).unwrap();
    fixture.run().unwrap();
    assert_eq!(
        fs::read_to_string(external_file).unwrap(),
        "external source"
    );
    assert_eq!(
        fs::read_to_string(fixture.root().join("src/main.rs")).unwrap(),
        "previous source"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_sources_destinations_indexes_and_lockfiles_fail_before_writes() {
    use std::os::unix::fs::symlink;
    for target in [
        "src/main.rs",
        "Cargo.lock",
        "target/rullst-upgrades/test/files/src/main.rs",
        "target/rullst-upgrades/test/index.tsv",
    ] {
        let fixture = Fixture::new();
        let path = fixture.root().join(target);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        symlink(fixture.root().join("Cargo.toml"), path).unwrap();
        assert!(fixture.run().is_err(), "accepted symlink: {target}");
        fixture.unchanged_manifest();
    }
}

#[cfg(unix)]
#[test]
fn symlinked_parent_inside_project_is_rejected_before_writes() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fs::rename(fixture.root().join("src"), fixture.root().join("other")).unwrap();
    symlink("other", fixture.root().join("src")).unwrap();
    assert!(fixture.run().is_err());
    fixture.unchanged_manifest();
    assert_eq!(
        fs::read_to_string(fixture.root().join("other/main.rs")).unwrap(),
        "current source"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_backup_ancestor_is_rejected_even_if_it_points_inside_the_project() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fs::rename(fixture.root().join("target"), fixture.root().join("other")).unwrap();
    symlink("other", fixture.root().join("target")).unwrap();
    assert!(fixture.run().is_err());
    fixture.unchanged_manifest();
}

#[cfg(unix)]
#[test]
fn restored_permissions_match_the_saved_file() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::set_permissions(
        fixture.backup.join("files/src/main.rs"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fixture.run().unwrap();
    assert_eq!(
        fs::metadata(fixture.root().join("src/main.rs"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[cfg(windows)]
#[test]
fn a_locked_windows_destination_reports_interruption_and_retains_backup() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let _lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(fixture.root().join("Cargo.toml"))
        .unwrap();
    let error = fixture.run().unwrap_err().to_string();
    assert!(error.contains("backup retained"), "{error}");
    assert!(fixture.backup.join("files/Cargo.toml").exists());
}

/// Each clause of the path contract on its own, with the exact reason: a
/// restore through `restore_from` also fails on later checks, which hides a
/// clause that stopped working.
#[test]
fn index_paths_must_be_portable_snapshot_files() {
    let unsafe_path = "invalid upgrade backup: unsafe relative path";
    let outside = "invalid upgrade backup: file outside the upgrade snapshot contract";
    for (path, expected) in [
        ("", unsafe_path),
        ("a:b.rs", unsafe_path),
        ("a\u{7}b.rs", unsafe_path),
        ("../main.rs", unsafe_path),
        ("src/../main.rs", unsafe_path),
        ("/main.rs", unsafe_path),
        ("target/main.rs", unsafe_path),
        ("src/Target/main.rs", unsafe_path),
        (".git/main.rs", unsafe_path),
        ("src/.GIT/main.rs", unsafe_path),
        ("src/notes.txt", outside),
        ("src/Cargo.lock", outside),
    ] {
        let error = validate_relative_restore_path(Path::new(path)).unwrap_err();
        assert_eq!(error.to_string(), expected, "{path:?}");
    }
    for path in [
        "Cargo.lock",
        "Cargo.toml",
        "crates/api/Cargo.toml",
        "src/main.rs",
        "src/targets/git.rs",
    ] {
        validate_relative_restore_path(Path::new(path)).unwrap();
    }
}

#[test]
fn a_file_is_not_a_directory_to_create() {
    let fixture = Fixture::new();
    for path in ["Cargo.toml", "src/main.rs/nested"] {
        let error = validate_directory_creation(&fixture.root(), Path::new(path)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid upgrade backup: symlink, reparse point or non-directory parent",
            "{path}"
        );
    }
    validate_directory_creation(&fixture.root(), Path::new("src/missing/deeper")).unwrap();
    assert!(
        validate_file(&fixture.root(), Path::new("src/missing.rs"))
            .unwrap()
            .is_none()
    );
}

/// Only a missing entry counts as absent; any other lookup failure stops the
/// restore. Linux and macOS refuse a 300-byte name (`ENAMETOOLONG`); Windows
/// reports such a name with a different error, so the test is Unix-only.
#[cfg(unix)]
#[test]
fn lookup_failures_other_than_a_missing_entry_are_errors() {
    let fixture = Fixture::new();
    let long = "a".repeat(300);
    let error = validate_file(&fixture.root(), Path::new(&format!("{long}.rs"))).unwrap_err();
    assert!(matches!(error, RestoreError::Io(_)), "{error}");
    let error = validate_directory_creation(&fixture.root(), Path::new(&long)).unwrap_err();
    assert!(matches!(error, RestoreError::Io(_)), "{error}");
}

/// An index of exactly 8 MiB is read whole and parsed, not refused as
/// oversized; it ends in a two-byte character, so a shorter read would not
/// be UTF-8.
#[test]
fn an_index_of_exactly_8_mib_is_parsed() {
    use std::io::{Seek, SeekFrom, Write};
    let fixture = Fixture::new();
    let mut index = File::create(fixture.backup.join("index.tsv")).unwrap();
    // A literal size, so a changed bound cannot move the fixture with it.
    index.set_len(8 * 1024 * 1024 - 2).unwrap();
    index.seek(SeekFrom::End(0)).unwrap();
    index.write_all("é".as_bytes()).unwrap();
    drop(index);
    assert_eq!(
        fixture.run().unwrap_err().to_string(),
        "invalid upgrade backup: malformed index entry"
    );
    fixture.unchanged_manifest();
}

#[test]
fn a_snapshot_of_several_mebibytes_is_restored() {
    let fixture = Fixture::new();
    File::options()
        .write(true)
        .open(fixture.backup.join("files/src/main.rs"))
        .unwrap()
        .set_len(2 * 1024 * 1024)
        .unwrap();
    fixture.run().unwrap();
    assert_eq!(
        fs::metadata(fixture.root().join("src/main.rs"))
            .unwrap()
            .len(),
        2 * 1024 * 1024
    );
}

#[test]
fn the_backup_directory_itself_is_not_a_backup() {
    let fixture = Fixture::new();
    let error = restore_from(
        &fixture.root(),
        &fixture.root().join("target/rullst-upgrades"),
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid upgrade backup: backup must be inside this project's target/rullst-upgrades directory"
    );
}

#[test]
fn only_the_root_lockfile_may_be_absent() {
    let fixture = Fixture::new();
    fixture.index("present\tCargo.toml\nabsent\tsrc/main.rs\n");
    assert_eq!(
        fixture.run().unwrap_err().to_string(),
        "invalid upgrade backup: only a newly created root Cargo.lock may be removed"
    );
    fixture.unchanged_manifest();
}

/// Snapshots totalling exactly 512 MiB pass planning, which then stops on a
/// later entry; more than 512 MiB stops on the snapshot that passes it. The
/// snapshots are sparse and planning fails before anything is copied.
#[test]
fn snapshots_may_total_at_most_512_mib() {
    let plan_total = |count: usize, size: u64| {
        let fixture = Fixture::new();
        let mut index = String::new();
        for number in 0..count {
            let name = format!("src/part{number}.rs");
            File::create(fixture.backup.join("files").join(&name))
                .unwrap()
                .set_len(size)
                .unwrap();
            index.push_str(&format!("present\t{name}\n"));
        }
        index.push_str("absent\tsrc/main.rs\n");
        fixture.index(&index);
        let error = fixture.run().unwrap_err().to_string();
        fixture.unchanged_manifest();
        error
    };
    assert_eq!(
        plan_total(8, 64 * 1024 * 1024),
        "invalid upgrade backup: only a newly created root Cargo.lock may be removed"
    );
    assert_eq!(
        plan_total(9, 60 * 1024 * 1024),
        "invalid upgrade backup: snapshot exceeds 64 MiB/file or 512 MiB/restore"
    );
}
