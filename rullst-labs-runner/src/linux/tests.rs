use super::*;
use rullst_labs::{CaseOutput, ExecutionLimits, WorkerOutcome};
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

// Fixed, reviewed addition fixture only. Hostile/looping/compiler submissions
// belong to the mandatory isolated acceptance harness, never this local test.
const ADD: &[u8] = &[
    0, 97, 115, 109, 1, 0, 0, 0, 1, 7, 1, 96, 2, 126, 126, 1, 126, 3, 2, 1, 0, 7, 9, 1, 5, 115,
    111, 108, 118, 101, 0, 0, 10, 9, 1, 7, 0, 32, 0, 32, 1, 124, 11,
];
#[test]
fn validated_fixed_wasm_has_the_exact_rust_profile_abi_and_values() {
    let result = engine::evaluate(
        ADD,
        &[[3, 5], [9, -2]],
        &ExecutionLimits::new(10, 10000, 64).unwrap(),
    );
    assert!(
        matches!(result,WorkerOutcome::Executed {cases,..} if cases==[CaseOutput::Value(8),CaseOutput::Value(7)])
    );
    let rust = include_bytes!("../../tests/fixtures/checked_sum.wasm");
    structure::validate(rust, &ExecutionLimits::new(10, 10000, 64).unwrap())
        .expect("trusted Rust structural bounds");
    let wasm_engine = wasmi::Engine::new(&engine::configuration());
    wasmi::Module::new(&wasm_engine, rust.as_slice()).expect("trusted Rust module validation");
    assert!(
        matches!(engine::evaluate(rust,&[[3,5],[9,-2]],&ExecutionLimits::new(10,10000,64).unwrap()),WorkerOutcome::Executed {cases,..} if cases==[CaseOutput::Value(8),CaseOutput::Value(7)])
    );
    for invalid in [
        &ADD[..7],
        b"(module)".as_slice(),
        b"\0asm\x01\0\0\0".as_slice(),
    ] {
        assert!(matches!(
            engine::evaluate(
                invalid,
                &[[1, 2]],
                &ExecutionLimits::new(10, 10000, 64).unwrap()
            ),
            WorkerOutcome::Rejected(_)
        ));
    }
}
#[test]
fn bundle_identity_binds_paths_bytes_and_refuses_symlinks_or_writable_shared_tools() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let file = dir.path().join("compiler");
    std::fs::write(&file, b"trusted-fixture-one").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let first = config::hash_tree(dir.path()).unwrap();
    std::fs::rename(&file, dir.path().join("different")).unwrap();
    assert_ne!(config::hash_tree(dir.path()).unwrap(), first);
    std::fs::rename(dir.path().join("different"), &file).unwrap();
    std::fs::write(&file, b"trusted-fixture-two").unwrap();
    assert_ne!(config::hash_tree(dir.path()).unwrap(), first);
    symlink(&file, dir.path().join("alias")).unwrap();
    assert!(config::hash_tree(dir.path()).is_err());
    std::fs::remove_file(dir.path().join("alias")).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();
    assert!(config::hash_tree(dir.path()).is_err());
}
#[test]
fn pinned_artifacts_require_trusted_ancestors_as_well_as_read_only_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let parent = dir.path().join("tools");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    let file = parent.join("artifact");
    std::fs::write(&file, b"abc").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).unwrap();
    assert_eq!(
        config::hash_file(&file).unwrap().as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(config::hash_file(&file).is_err());
    assert!(config::hash_tree(&parent).is_err());
    // A sticky ancestor protects an owned file from another UID's replacement,
    // but a shared directory is still not admitted as the actual tool tree.
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777)).unwrap();
    assert!(config::hash_file(&file).is_ok());
    assert!(config::hash_tree(&parent).is_err());
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(config::hash_tree(&parent).is_ok());
}

#[test]
fn launcher_has_fixed_read_only_boundaries_and_no_host_environment_or_command_string() {
    let command = bootstrap::command(
        Path::new("/trusted/bwrap"),
        Path::new("/trusted/bundle"),
        Path::new("/sys/fs/cgroup/delegated/job"),
    );
    let args: Vec<_> = command.get_args().map(|v| v.to_str().unwrap()).collect();
    for required in [
        "--unshare-all",
        "--unshare-user",
        "--unshare-cgroup",
        "--disable-userns",
        "--assert-userns-disabled",
        "--new-session",
        "--die-with-parent",
        "--clearenv",
    ] {
        assert!(args.contains(&required));
    }
    assert!(!args.contains(&"--bind"));
    assert!(!args.contains(&"--dev-bind"));
    assert!(!args.contains(&"--share-net"));
    assert!(!args.contains(&"/home"));
    assert!(
        args.windows(3)
            .any(|v| v == ["--ro-bind", "/sys/fs/cgroup/delegated/job", "/limits"])
    );
    assert!(args.ends_with(&["--", "/runner", "__worker"]));
    assert!(command.get_envs().all(|(_, value)| value.is_none()));
}
#[test]
fn ordinary_directory_cannot_impersonate_a_delegated_cgroup() {
    let dir = tempfile::tempdir().unwrap();
    for (name, value) in [
        ("memory.max", "1073741824"),
        ("pids.max", "32"),
        ("cgroup.procs", ""),
        ("cgroup.type", "domain"),
    ] {
        std::fs::write(dir.path().join(name), value).unwrap();
    }
    assert!(cgroup::validate_root(dir.path()).is_err());
}
#[test]
fn key_loading_requires_an_owned_private_regular_file_with_exact_length() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("seed");
    std::fs::write(&file, [9u8; 32]).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(*service::read_key(&file).unwrap(), [9u8; 32]);
    symlink(&file, dir.path().join("alias")).unwrap();
    assert!(service::read_key(&dir.path().join("alias")).is_err());
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(service::read_key(&file).is_err());
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&file, [9u8; 33]).unwrap();
    assert!(service::read_key(&file).is_err());
}
