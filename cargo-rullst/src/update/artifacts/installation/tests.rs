//! Native transaction/probe tests use locally compiled trusted fixture binaries.
//! They do not substitute for published artifact provenance or registry acceptance.
use super::*;
#[path = "fault_tests.rs"]
mod fault_tests;
use crate::update::cache;
use sha2::{Digest, Sha256};
use std::{
    fs,
    process::{Child, Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};

fn base() -> tempfile::TempDir {
    #[cfg(windows)]
    let base = tempfile::tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
    #[cfg(not(windows))]
    let base = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(base.path(), fs::Permissions::from_mode(0o700)).unwrap();
    }
    base
}

fn binaries() -> &'static [Vec<u8>; 2] {
    static BINARIES: OnceLock<[Vec<u8>; 2]> = OnceLock::new();
    BINARIES.get_or_init(|| {
        let temp = base();
        ["12.1.0", "12.2.0"].map(|version| {
            let source = temp.path().join("fixture.rs");
            fs::write(
                &source,
                format!(
                    r#"
fn main() {{
    if std::env::args().any(|arg| arg == "--hold") {{
        std::fs::write(std::env::var_os("RULLST_INSTALL_HELD").unwrap(), b"ready").unwrap();
        loop {{ std::thread::sleep(std::time::Duration::from_secs(1)); }}
    }}
    println!("rullst {version}");
}}
"#
                ),
            )
            .unwrap();
            let output = temp
                .path()
                .join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
            let result = Command::new("rustc")
                .arg(&source)
                .arg("-o")
                .arg(&output)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            fs::read(output).unwrap()
        })
    })
}

fn candidate(base: &Path, newer: bool) -> Candidate {
    let version = if newer { "12.2.0" } else { "12.1.0" };
    let body = &binaries()[usize::from(newer)];
    let root = cache::installation_root(&base.join("installed")).unwrap();
    let directory = base.join(format!("download-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let target = native_target().unwrap();
    let names = state::executable_names(target);
    let records = ["cargo-rullst", "rullst"].iter().zip(names).map(|(executable, name)| {
        let suffix = if name.ends_with(".exe") { ".exe" } else { "" };
        let name = format!("{executable}-{version}-{target}{suffix}");
        fs::write(directory.join(&name), body).unwrap();
        serde_json::json!({"name":name,"executable":executable,"bytes":body.len(),"sha256":hex::encode(Sha256::digest(body))})
    }).collect::<Vec<_>>();
    let raw_manifest = serde_json::to_vec(&serde_json::json!({"schema":"rullst.cli-artifacts.v1", "version":version,
        "target":target,"repository":"Rullst/Rullst","source_commit":"a".repeat(40),"release_tag":format!("v{version}"),
        "build_runner":"fixture-native","files":records})).unwrap();
    let artifact = transaction::parse_manifest(&raw_manifest, target).unwrap();
    let prior = state::inspect(&root, target).unwrap();
    Candidate {
        root,
        directory,
        artifact,
        raw_manifest,
        prior,
    }
}

fn digest(candidate: &Candidate) -> String {
    review(
        &candidate.root,
        &candidate.artifact,
        candidate.prior.as_ref(),
    )
    .unwrap()["review_sha256"]
        .as_str()
        .unwrap()
        .into()
}

fn install(candidate: &Candidate) -> serde_json::Value {
    application::apply(candidate, &digest(candidate)).unwrap()
}

fn recover(candidate: &Candidate) -> serde_json::Value {
    application::recover_with(&candidate.root, &digest(candidate), |_, _| Ok(())).unwrap()
}

fn begin(candidate: &Candidate) -> (storage::Storage, PathBuf, transaction::Intent) {
    let storage = storage::Storage::open(&candidate.root).unwrap();
    let (operation, intent) = transaction::prepare(
        &storage,
        &candidate.root,
        &candidate.directory,
        &candidate.artifact,
        &candidate.raw_manifest,
        candidate.prior.as_ref(),
        &digest(candidate),
    )
    .unwrap();
    cache::create_installation_directory(&candidate.root).unwrap();
    storage.select(&operation).unwrap();
    (storage, operation, intent)
}

#[test]
fn installs_both_native_binaries_then_updates_and_restores_the_exact_predecessor() {
    let base = base();
    let first = candidate(base.path(), false);
    assert_eq!(install(&first)["installed"], true);
    let second = candidate(base.path(), true);
    assert_eq!(install(&second)["known_predecessor_retained"], true);
    assert_eq!(
        state::inspect(&second.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .manifest
            .version,
        "12.2.0"
    );
    assert!(application::recover_with(&second.root, &"f".repeat(64), |_, _| Ok(())).is_err());
    assert!(
        application::recover_with(&second.root, &digest(&second), |_, _| Err(
            ArtifactError::Invalid("rejected provenance")
        ))
        .is_err()
    );
    assert_eq!(
        state::inspect(&second.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .manifest
            .version,
        "12.2.0"
    );
    assert_eq!(recover(&second)["predecessor_restored"], true);
    assert_eq!(recover(&second)["already_recovered"], true);
    assert_eq!(
        state::inspect(&first.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .raw_manifest,
        first.raw_manifest
    );
}

#[test]
fn first_installation_recovery_removes_only_the_recorded_entries() {
    let base = base();
    let candidate = candidate(base.path(), false);
    install(&candidate);
    fs::write(candidate.root.join("personal-note"), b"keep this").unwrap();
    recover(&candidate);
    assert_eq!(
        fs::read(candidate.root.join("personal-note")).unwrap(),
        b"keep this"
    );
    assert!(!candidate.root.join(state::RECEIPT).exists());
    assert_eq!(recover(&candidate)["already_recovered"], true);
}

#[test]
fn all_retirement_and_replacement_boundaries_recover_after_a_late_io_failure() {
    for failure_index in 0..3 {
        for after_replace in [false, true] {
            let base = base();
            let first = candidate(base.path(), false);
            install(&first);
            let second = candidate(base.path(), true);
            let (storage, operation, mut intent) = begin(&second);
            let result = transaction::commit_with(&operation, &mut intent, |index, replaced| {
                if index == failure_index && replaced == after_replace {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::StorageFull,
                        "injected write failure",
                    )
                    .into());
                }
                Ok(())
            });
            assert!(result.is_err());
            drop(storage);
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

#[test]
fn divergent_targets_and_modified_backups_block_recovery_before_any_write() {
    for tamper_backup in [false, true] {
        let base = base();
        let first = candidate(base.path(), false);
        install(&first);
        let second = candidate(base.path(), true);
        let result = install(&second);
        let operation = PathBuf::from(result["operation"].as_str().unwrap());
        let name = &state::executable_names(native_target().unwrap())[1];
        let target = if tamper_backup {
            operation.join("before").join(name)
        } else {
            second.root.join(name)
        };
        fs::write(&target, b"later unrelated bytes").unwrap();
        let untouched = fs::read(
            second
                .root
                .join(&state::executable_names(native_target().unwrap())[0]),
        )
        .unwrap();
        assert!(application::recover_with(&second.root, &digest(&second), |_, _| Ok(())).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"later unrelated bytes");
        assert_eq!(
            fs::read(
                second
                    .root
                    .join(&state::executable_names(native_target().unwrap())[0])
            )
            .unwrap(),
            untouched
        );
    }
}

#[test]
fn digest_copy_smoke_and_destination_lock_failures_preserve_installed_files() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let mut second = candidate(base.path(), true);
    let locked = storage::Storage::open(&second.root).unwrap();
    assert!(storage::Storage::open(&second.root).is_err());
    drop(locked);
    let file = &second.artifact.files[0];
    fs::write(second.directory.join(&file.name), b"tampered").unwrap();
    assert!(application::apply(&second, &digest(&second)).is_err());
    second = candidate(base.path(), true);
    // A manifest can bind bytes that do not implement its expected --version contract.
    for file in &mut second.artifact.files {
        fs::write(second.directory.join(&file.name), &binaries()[0]).unwrap();
        file.bytes = binaries()[0].len() as u64;
        file.sha256 = hex::encode(Sha256::digest(&binaries()[0]));
    }
    second.raw_manifest = serde_json::to_vec(&second.artifact).unwrap();
    assert!(application::apply(&second, &digest(&second)).is_err());
    assert_eq!(
        state::inspect(&first.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .raw_manifest,
        first.raw_manifest
    );
}

#[test]
fn retention_keeps_the_selected_recovery_and_refuses_unknown_historical_files() {
    let base = base();
    let first = candidate(base.path(), false);
    let first_result = install(&first);
    let first_operation = PathBuf::from(first_result["operation"].as_str().unwrap());
    let second = candidate(base.path(), true);
    let second_result = install(&second);
    let second_operation = PathBuf::from(second_result["operation"].as_str().unwrap());
    fs::write(first_operation.join("personal-note"), b"keep this").unwrap();
    let third = candidate(base.path(), true);
    assert!(application::apply(&third, &digest(&third)).is_err());
    assert_eq!(
        fs::read(first_operation.join("personal-note")).unwrap(),
        b"keep this"
    );
    assert!(second_operation.exists());
    fs::remove_file(first_operation.join("personal-note")).unwrap();
    install(&third);
    assert!(!first_operation.exists());
    assert!(second_operation.exists());
    recover(&third);
}

#[cfg(windows)]
#[test]
fn windows_new_destination_case_aliases_share_one_lock() {
    let base = base();
    let first = cache::installation_root(&base.path().join("Installed")).unwrap();
    let alias = cache::installation_root(&base.path().join("INSTALLED")).unwrap();
    let locked = storage::Storage::open(&first).unwrap();
    assert!(storage::Storage::open(&alias).is_err());
    let storage_path = locked.path.clone();
    drop(locked);
    assert_eq!(storage::Storage::open(&alias).unwrap().path, storage_path);
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_marker(child: &mut OwnedChild, marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !marker.exists() && Instant::now() < deadline {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before checkpoint"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(marker.exists(), "child did not reach the checkpoint");
}

#[test]
fn an_executing_old_cli_can_be_retained_and_recovered_without_unlinking_its_image() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let marker = base.path().join("held");
    let mut child = OwnedChild(
        Command::new(
            first
                .root
                .join(&state::executable_names(native_target().unwrap())[0]),
        )
        .arg("--hold")
        .env("RULLST_INSTALL_HELD", &marker)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap(),
    );
    wait_marker(&mut child, &marker);
    let second = candidate(base.path(), true);
    install(&second);
    assert!(child.0.try_wait().unwrap().is_none());
    recover(&second);
    assert!(child.0.try_wait().unwrap().is_none());
}

#[test]
fn interrupted_child_fixture() {
    let Some(root) = std::env::var_os("RULLST_INSTALL_CHILD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let storage = storage::Storage::open(&root).unwrap();
    let operation = storage.current().unwrap().unwrap();
    let mut intent = transaction::Intent::load(&operation, &root).unwrap();
    transaction::commit_with(&operation, &mut intent, |index, replaced| {
        if index == 0 && !replaced {
            fs::write(
                std::env::var_os("RULLST_INSTALL_CHILD_MARKER").unwrap(),
                b"retired",
            )
            .unwrap();
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn killed_process_between_retirement_and_replacement_recovers_persisted_intent() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let (storage, _, _) = begin(&second);
    drop(storage);
    let marker = base.path().join("retired");
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "update::artifacts::installation::native_tests::interrupted_child_fixture",
                "--nocapture",
            ])
            .env("RULLST_INSTALL_CHILD_ROOT", &second.root)
            .env("RULLST_INSTALL_CHILD_MARKER", &marker)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    wait_marker(&mut child, &marker);
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
    recover(&second);
    assert_eq!(
        state::inspect(&first.root, native_target().unwrap())
            .unwrap()
            .unwrap()
            .raw_manifest,
        first.raw_manifest
    );
}
