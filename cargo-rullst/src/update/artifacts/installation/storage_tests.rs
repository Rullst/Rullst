//! Destination storage records, names, copies and retained operations.
use super::*;

fn error_text<T>(result: Result<T, ArtifactError>) -> String {
    match result {
        Ok(_) => panic!("the operation was accepted"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn operation_names_are_canonical_version_4_uuids() {
    let id = uuid::Uuid::new_v4();
    storage::validate_operation(&format!("op-{id}")).unwrap();
    for name in [
        format!("op-{}", id.to_string().to_uppercase()),
        format!("op-{}", id.simple()),
        "op-00000000-0000-1000-8000-000000000000".into(),
        id.to_string(),
        "operation.lock".into(),
    ] {
        assert!(storage::validate_operation(&name).is_err(), "{name}");
    }
}

#[test]
fn records_are_nonempty_and_at_most_128_kib() {
    let base = base();
    assert!(storage::atomic_write(base.path(), "empty", b"").is_err());
    storage::atomic_write(base.path(), "largest", &vec![b'x'; 128 * 1024]).unwrap();
    assert_eq!(
        fs::read(base.path().join("largest")).unwrap().len(),
        131_072
    );
    assert!(storage::atomic_write(base.path(), "larger", &vec![b'x'; 128 * 1024 + 1]).is_err());
}

#[test]
fn copies_have_exactly_the_authenticated_length_and_digest() {
    let base = base();
    let source = base.path().join("source");
    fs::write(&source, b"0123456789").unwrap();
    let prefix = hex::encode(Sha256::digest(b"01234"));
    let whole = hex::encode(Sha256::digest(b"0123456789"));
    let longer = base.path().join("longer");
    assert!(storage::copy(&source, &longer, 5, &prefix, false).is_err());
    assert!(!longer.exists());
    assert!(storage::copy(&source, &base.path().join("forged"), 10, &prefix, false).is_err());
    storage::copy(&source, &base.path().join("exact"), 10, &whole, false).unwrap();
    assert_eq!(fs::read(base.path().join("exact")).unwrap(), b"0123456789");
}

#[cfg(unix)]
#[test]
fn syncing_a_missing_directory_fails() {
    let base = base();
    assert!(storage::sync(&base.path().join("missing")).is_err());
}

#[test]
fn new_operations_refuse_unknown_entries_and_stop_at_eight() {
    let base = base();
    let root = cache::installation_root(&base.path().join("installed")).unwrap();
    let storage = storage::Storage::open(&root).unwrap();
    let unknown = storage.path.join("notes");
    fs::write(&unknown, b"not updater evidence").unwrap();
    assert_eq!(
        error_text(storage.operation()),
        "artifact verification rejected: invalid installation operation name"
    );
    fs::remove_file(unknown).unwrap();
    for _ in 0..8 {
        storage.operation().unwrap();
    }
    assert_eq!(
        error_text(storage.operation()),
        "artifact verification rejected: eight retained installation operations; archive older completed evidence before continuing"
    );
}

#[cfg(unix)]
#[test]
fn storage_of_a_deeply_nested_root_can_be_reopened() {
    use std::os::unix::fs::DirBuilderExt;
    let base = base();
    let mut parent = base.path().to_path_buf();
    for _ in 0..5 {
        parent.push("d".repeat(220));
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&parent)
        .unwrap();
    // Its owner record is longer than 1 KiB.
    let root = parent.join("installed");
    drop(storage::Storage::open(&root).unwrap());
    storage::Storage::open(&root).unwrap();
}

#[test]
fn retention_allows_at_most_eight_operations() {
    let base = base();
    let root = cache::installation_root(&base.path().join("installed")).unwrap();
    let storage = storage::Storage::open(&root).unwrap();
    for _ in 0..8 {
        cache::create_installation_directory(
            &storage.path.join(format!("op-{}", uuid::Uuid::new_v4())),
        )
        .unwrap();
    }
    retention::prune(&storage.path, &root, None).unwrap();
    cache::create_installation_directory(
        &storage.path.join(format!("op-{}", uuid::Uuid::new_v4())),
    )
    .unwrap();
    assert_eq!(
        error_text(retention::prune(&storage.path, &root, None)),
        "artifact verification rejected: installation evidence exceeds retention bound"
    );
}

#[test]
fn retention_removes_a_completed_update_with_its_retired_files() {
    let base = base();
    install(&candidate(base.path(), false));
    let second = candidate(base.path(), true);
    let second_operation = PathBuf::from(install(&second)["operation"].as_str().unwrap());
    assert!(second_operation.join("retired").read_dir().unwrap().count() > 0);
    install(&candidate(base.path(), false));
    assert!(second_operation.exists());
    install(&candidate(base.path(), true));
    assert!(!second_operation.exists());
}
