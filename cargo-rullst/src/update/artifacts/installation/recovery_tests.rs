//! Recovery refuses an installation it cannot account for, before any move.
use super::*;

fn installed_name(index: usize) -> String {
    state::executable_names(native_target().unwrap())[index].clone()
}

#[test]
fn a_deleted_binary_after_a_complete_installation_blocks_recovery() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let report = install(&second);
    let operation = PathBuf::from(report["operation"].as_str().unwrap());
    // The retired predecessor is still in the operation, but a completed
    // installation has no reason to be missing its new binary.
    fs::remove_file(second.root.join(installed_name(1))).unwrap();
    let intent = transaction::Intent::load(&operation, &second.root).unwrap();
    let error = recovery::validate(&operation, &intent)
        .unwrap_err()
        .to_string();
    assert!(error.contains("recovery found unrelated edits"), "{error}");
    assert!(!second.root.join(installed_name(1)).exists());
}

#[test]
fn a_binary_deleted_before_it_was_retired_blocks_recovery() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let (storage, operation, intent) = begin(&second);
    // Prepared, not committed: the predecessor was never retired or removed.
    fs::remove_file(second.root.join(installed_name(0))).unwrap();
    let error = recovery::validate(&operation, &intent)
        .unwrap_err()
        .to_string();
    assert!(error.contains("recovery found unrelated edits"), "{error}");
    drop(storage);
}

#[test]
fn a_changed_staged_predecessor_stops_recovery_before_moving_any_entry() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let report = install(&second);
    let operation = PathBuf::from(report["operation"].as_str().unwrap());
    let staging = cache::create_installation_directory(&operation.join("restore")).unwrap();
    let mut staged = cache::new_installation_file(&staging.join(installed_name(0))).unwrap();
    std::io::Write::write_all(&mut staged, b"not the predecessor").unwrap();
    drop(staged);
    let installed = fs::read(second.root.join(installed_name(0))).unwrap();
    let mut intent = transaction::Intent::load(&operation, &second.root).unwrap();
    let error = recovery::restore(&operation, &mut intent)
        .unwrap_err()
        .to_string();
    assert!(error.contains("recovery staging was changed"), "{error}");
    assert_eq!(
        fs::read(second.root.join(installed_name(0))).unwrap(),
        installed
    );
    assert!(
        transaction::Intent::load(&operation, &second.root)
            .unwrap()
            .phase
            == transaction::Phase::Complete
    );
}

#[cfg(unix)]
#[test]
fn restored_binaries_are_executable_and_the_receipt_is_not() {
    use std::os::unix::fs::PermissionsExt;
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    install(&second);
    recover(&second);
    for name in [installed_name(0), installed_name(1)] {
        let mode = fs::metadata(second.root.join(&name))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "{name}");
    }
    let mode = fs::metadata(second.root.join(state::RECEIPT))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn recovery_without_a_selected_operation_is_refused() {
    let base = base();
    let root = cache::installation_root(&base.path().join("installed")).unwrap();
    assert!(application::recover(&root, &"f".repeat(64)).is_err());
}

#[test]
fn recover_requires_a_lowercase_review_digest_and_a_selected_operation() {
    let base = base();
    let root = base.path().join("installed");
    let recover = |digest: &str| {
        let matches = command()
            .try_get_matches_from([
                "install",
                "recover",
                "--root",
                root.to_str().unwrap(),
                "--approved-review",
                digest,
            ])
            .unwrap();
        match execute(&matches) {
            Ok(_) => panic!("recovery was accepted"),
            Err(error) => error.to_string(),
        }
    };
    assert_eq!(
        recover(&"f".repeat(64)),
        "artifact verification rejected: no selected installation operation to recover"
    );
    assert_eq!(
        recover(&"F".repeat(64)),
        "artifact verification rejected: approval must be a lowercase SHA-256 review digest"
    );
}

#[test]
fn a_destination_changed_after_review_is_refused() {
    let base = base();
    let first = candidate(base.path(), false);
    unchanged(&first).unwrap();
    install(&first);
    assert_eq!(
        unchanged(&first).unwrap_err().to_string(),
        "artifact verification rejected: installation destination changed after review"
    );
}
