use super::*;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    content: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("source");
        let content = base.join("candidate");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&content).unwrap();
        for name in ["Cargo.lock", "Cargo.toml"] {
            fs::write(root.join(name), format!("before {name}")).unwrap();
            fs::write(content.join(name), format!("after {name}")).unwrap();
        }
        Self {
            _temp: temp,
            root,
            content,
        }
    }

    fn replacements(&self) -> Vec<Replacement> {
        ["Cargo.lock", "Cargo.toml"]
            .iter()
            .map(|name| {
                let before = snapshot::read(&self.root, name).unwrap();
                let after = snapshot::read(&self.content, name).unwrap();
                let current = snapshot::record(name, before.as_deref());
                let permissions = Permissions::capture(&self.root, &current).unwrap();
                Replacement {
                    current,
                    desired: snapshot::record(name, after.as_deref()),
                    permissions,
                }
            })
            .collect()
    }
}

#[test]
fn staging_failure_removes_temporary_files_without_touching_any_original() {
    let fixture = Fixture::new();
    let replacements = fixture.replacements();
    fs::remove_file(fixture.content.join("Cargo.toml")).unwrap();
    assert!(Staged::create(&fixture.root, &fixture.content, replacements).is_err());
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "before Cargo.lock"
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap(),
        "before Cargo.toml"
    );
}

#[test]
fn late_io_failure_reports_partial_progress_and_never_truncates_a_hardlink() {
    let fixture = Fixture::new();
    let alias = fixture.content.join("external-alias");
    fs::hard_link(fixture.root.join("Cargo.lock"), &alias).unwrap();
    let staged = Staged::create(&fixture.root, &fixture.content, fixture.replacements()).unwrap();
    let error = staged
        .commit_with(&fixture.root, |index, temporary, path, absent| {
            if index == 1 {
                return Err(std::io::Error::other("injected replacement I/O failure").into());
            }
            replace(temporary, path, absent)
        })
        .unwrap_err();
    assert_eq!(error.0, 1);
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "after Cargo.lock"
    );
    assert_eq!(fs::read_to_string(alias).unwrap(), "before Cargo.lock");
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap(),
        "before Cargo.toml"
    );
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
}

#[test]
fn a_post_staging_user_edit_fails_before_the_first_replacement() {
    let fixture = Fixture::new();
    let staged = Staged::create(&fixture.root, &fixture.content, fixture.replacements()).unwrap();
    fs::write(fixture.root.join("Cargo.toml"), "later user edit").unwrap();
    let error = staged.commit(&fixture.root).unwrap_err();
    assert_eq!(error.0, 0);
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "before Cargo.lock"
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap(),
        "later user edit"
    );
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn readonly_targets_fail_without_leaving_readonly_staging_files() {
    let fixture = Fixture::new();
    let path = fixture.root.join("Cargo.toml");
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = Staged::create(&fixture.root, &fixture.content, fixture.replacements());
    fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "before Cargo.lock"
    );
}

#[cfg(unix)]
#[test]
fn permission_changes_after_staging_stop_before_any_replacement() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let staged = Staged::create(&fixture.root, &fixture.content, fixture.replacements()).unwrap();
    fs::set_permissions(
        fixture.root.join("Cargo.toml"),
        fs::Permissions::from_mode(0o400),
    )
    .unwrap();
    assert_eq!(staged.commit(&fixture.root).unwrap_err().0, 0);
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "before Cargo.lock"
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn extended_attributes_are_never_silently_discarded() {
    let fixture = Fixture::new();
    let path = fixture.root.join("Cargo.toml");
    let record = snapshot::record("Cargo.toml", Some(b"before Cargo.toml"));
    rustix::fs::setxattr(
        &path,
        "user.rullst-test",
        b"retained",
        rustix::fs::XattrFlags::empty(),
    )
    .unwrap();
    assert!(Permissions::capture(&fixture.root, &record).is_err());
    let mut value = [0u8; 8];
    assert_eq!(
        rustix::fs::getxattr(&path, "user.rullst-test", &mut value[..]).unwrap(),
        8
    );
    assert_eq!(&value, b"retained");
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
}

#[test]
fn a_new_lockfile_never_clobbers_a_concurrently_created_file() {
    let fixture = Fixture::new();
    let target = fixture.root.join("new-lock");
    let mut temporary = tempfile::NamedTempFile::new_in(&fixture.root).unwrap();
    temporary.write_all(b"update").unwrap();
    fs::write(&target, "other writer").unwrap();
    assert!(replace(Some(temporary), &target, true).is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "other writer");
}
