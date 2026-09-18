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

#[cfg(target_os = "macos")]
#[test]
fn darwin_extended_acl_rejects_replacement_without_changing_the_acl_or_content() {
    let fixture = Fixture::new();
    let path = fixture.root.join("Cargo.toml");
    let record = snapshot::record("Cargo.toml", Some(b"before Cargo.toml"));
    assert!(Permissions::capture(&fixture.root, &record).is_ok());
    let owner = std::process::Command::new("id")
        .arg("-un")
        .output()
        .unwrap();
    assert!(owner.status.success());
    let owner = String::from_utf8(owner.stdout).unwrap();
    let grant = format!("user:{} allow read", owner.trim());
    let result = std::process::Command::new("chmod")
        .args(["+a", &grant])
        .arg(&path)
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result);
    let error = Permissions::capture(&fixture.root, &record).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("extended ACL requires manual update"),
        "{error}"
    );
    let retained = std::process::Command::new("ls")
        .arg("-le")
        .arg(&path)
        .output()
        .unwrap();
    assert!(retained.status.success());
    assert!(String::from_utf8_lossy(&retained.stdout).contains("allow read"));
    assert_eq!(fs::read(&path).unwrap(), b"before Cargo.toml");
    assert_eq!(fs::read_dir(&fixture.root).unwrap().count(), 2);
    assert!(
        std::process::Command::new("chmod")
            .arg("-N")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(Permissions::capture(&fixture.root, &record).is_ok());
}

#[test]
fn interruption_child_fixture() {
    let Some(base) = std::env::var_os("RULLST_TEST_TRANSACTION_ROOT") else {
        return;
    };
    let base = PathBuf::from(base);
    let root = base.join("source");
    let content = base.join("candidate");
    let mut replacements = Vec::new();
    for name in ["Cargo.lock", "Cargo.toml"] {
        let current = snapshot::record(name, snapshot::read(&root, name).unwrap().as_deref());
        replacements.push(Replacement {
            permissions: Permissions::capture(&root, &current).unwrap(),
            current,
            desired: snapshot::record(name, snapshot::read(&content, name).unwrap().as_deref()),
        });
    }
    let intent = Intent {
        schema_version: "rullst.project-application-intent.v1".into(),
        phase: "applying".into(),
        source: root.clone(),
        prepared_directory: base.join("before"),
        verified_directory: base.clone(),
        receipt_sha256: "a".repeat(64),
        review_sha256: "b".repeat(64),
        changes: replacements
            .iter()
            .map(|replacement| super::super::review::Change {
                before: replacement.current.clone(),
                after: replacement.desired.clone(),
            })
            .collect(),
        permissions: replacements
            .iter()
            .map(|replacement| replacement.permissions.clone())
            .collect(),
    };
    let staged = Staged::create(&root, &content, replacements).unwrap();
    intent.store(&base).unwrap();
    let _ = staged.commit_with(&root, |index, temporary, target, absent| {
        replace(temporary, target, absent)?;
        if index == 0 {
            fs::write(base.join("first-written"), "ready")?;
            // A test-only barrier: the parent kills this real child after one
            // production replacement. No runtime/CLI fault switch is exposed.
            loop {
                std::thread::park_timeout(std::time::Duration::from_secs(1));
            }
        }
        Ok(())
    });
}

#[test]
fn killed_commit_retains_a_readable_intent_and_recoverable_before_after_states() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let fixture = Fixture::new();
    let base = fixture.root.parent().unwrap();
    let before = base.join("before");
    fs::create_dir(&before).unwrap();
    for name in ["Cargo.lock", "Cargo.toml"] {
        fs::copy(fixture.root.join(name), before.join(name)).unwrap();
    }
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "interruption_child_fixture",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("RULLST_TEST_TRANSACTION_ROOT", base)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let start = Instant::now();
    while !base.join("first-written").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before replacement barrier"
        );
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "child did not reach replacement barrier"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
    let intent: Intent =
        serde_json::from_slice(&fs::read(base.join("application.json")).unwrap()).unwrap();
    assert_eq!(intent.phase, "applying");
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.lock")).unwrap(),
        "after Cargo.lock"
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap(),
        "before Cargo.toml"
    );
    let replacements = intent
        .changes
        .into_iter()
        .zip(intent.permissions)
        .filter_map(|(change, permissions)| {
            let current = snapshot::record(
                &change.before.path,
                snapshot::read(&fixture.root, &change.before.path)
                    .unwrap()
                    .as_deref(),
            );
            if current == change.before {
                return None;
            }
            assert_eq!(current, change.after);
            Some(Replacement {
                current,
                desired: change.before,
                permissions,
            })
        })
        .collect();
    assert_eq!(
        Staged::create(&fixture.root, &before, replacements)
            .unwrap()
            .commit(&fixture.root)
            .unwrap(),
        1
    );
    for name in ["Cargo.lock", "Cargo.toml"] {
        assert_eq!(
            fs::read(fixture.root.join(name)).unwrap(),
            fs::read(before.join(name)).unwrap()
        );
    }
}
