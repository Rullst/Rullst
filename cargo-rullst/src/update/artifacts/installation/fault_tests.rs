use super::*;

#[test]
fn oversized_local_files_reject_before_unbounded_hashing() {
    let base = base();
    let path = base.path().join("oversized");
    let file = cache::new_installation_file(&path).unwrap();
    file.set_len(super::super::super::manifest::MAX_BINARY + 1)
        .unwrap();
    assert!(
        transaction::record(&path)
            .unwrap_err()
            .to_string()
            .contains("exceeds 128 MiB")
    );
}

#[test]
fn recovery_resumes_after_each_removal_and_restore_failure() {
    for failure_index in 0..3 {
        for restored in [false, true] {
            let base = base();
            let first = candidate(base.path(), false);
            install(&first);
            let second = candidate(base.path(), true);
            let report = install(&second);
            let operation = PathBuf::from(report["operation"].as_str().unwrap());
            let mut intent = transaction::Intent::load(&operation, &second.root).unwrap();
            assert!(
                recovery::restore_with(&operation, &mut intent, |index, after| {
                    if index == failure_index && after == restored {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::StorageFull,
                            "injected recovery failure",
                        )
                        .into())
                    } else {
                        Ok(())
                    }
                })
                .is_err()
            );
            recover(&second);
            assert_eq!(
                state::inspect(&first.root, native_target().unwrap())
                    .unwrap()
                    .unwrap()
                    .raw_manifest,
                first.raw_manifest
            );
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn limited_file_child_fixture() {
    let Some(path) = std::env::var_os("RULLST_INSTALL_LIMITED_FIXTURE") else {
        return;
    };
    let data: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let root = PathBuf::from(data["root"].as_str().unwrap());
    let directory = PathBuf::from(data["directory"].as_str().unwrap());
    let raw_manifest = data["manifest"].as_str().unwrap().as_bytes().to_vec();
    let artifact = transaction::parse_manifest(&raw_manifest, native_target().unwrap()).unwrap();
    let prior = state::inspect(&root, native_target().unwrap()).unwrap();
    application::apply(
        &Candidate {
            root,
            directory,
            artifact,
            raw_manifest,
            prior,
        },
        data["digest"].as_str().unwrap(),
    )
    .unwrap();
    panic!("file-size limit did not stop installation");
}

#[cfg(target_os = "linux")]
#[test]
fn operating_system_file_size_limit_stops_backup_without_replacing_installed_entries() {
    use std::os::unix::process::ExitStatusExt;
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let fixture = base.path().join("file-limit.json");
    fs::write(&fixture, serde_json::to_vec(&serde_json::json!({"root":second.root, "directory":second.directory,
        "manifest":String::from_utf8(second.raw_manifest.clone()).unwrap(), "digest":digest(&second)})).unwrap()).unwrap();
    let result = Command::new("bash")
        .args(["-c", "ulimit -c 0; ulimit -f 1; exec \"$@\"", "rullst-install-limit"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "update::artifacts::installation::native_tests::fault_tests::limited_file_child_fixture"])
        .env("RULLST_INSTALL_LIMITED_FIXTURE", &fixture)
        .output().unwrap();
    assert_eq!(result.status.signal(), Some(25), "{:?}", result);
    assert_eq!(
        state::inspect(&first.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .raw_manifest,
        first.raw_manifest
    );
    // The selected known-good operation remains recoverable; partial unselected
    // evidence is retained for explicit inspection after an OS termination.
    recover(&first);
}
