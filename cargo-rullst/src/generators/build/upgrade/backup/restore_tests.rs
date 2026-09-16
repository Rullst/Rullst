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
