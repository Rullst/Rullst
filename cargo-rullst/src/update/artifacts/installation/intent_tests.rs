//! Exact bounds of the persisted installation journal and its manifests.
use super::*;

const MAX_BINARY: u64 = 128 * 1024 * 1024;

fn record(body: &[u8]) -> transaction::Record {
    transaction::Record {
        bytes: body.len() as u64,
        sha256: hex::encode(Sha256::digest(body)),
    }
}

fn load_error(operation: &Path, root: &Path) -> String {
    match transaction::Intent::load(operation, root) {
        Ok(_) => panic!("the journal was accepted"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn digests_are_exactly_64_lowercase_hex_characters() {
    assert!(transaction::valid_digest(&"0123456789abcdef".repeat(4)));
    for invalid in [
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        String::new(),
    ] {
        assert!(!transaction::valid_digest(&invalid), "{invalid}");
    }
}

#[test]
fn an_installation_manifest_may_use_its_whole_16_kib_bound() {
    let target = "x86_64-unknown-linux-gnu";
    let manifest = super::super::super::tests::fixture_manifest();
    let mut body = serde_json::to_vec(&manifest).unwrap();
    body.resize(16 * 1024, b' ');
    assert_eq!(
        transaction::parse_manifest(&body, target).unwrap().version,
        "12.1.0"
    );
    body.push(b' ');
    assert!(transaction::parse_manifest(&body, target).is_err());
}

#[test]
fn a_journal_may_record_the_largest_supported_binary() {
    let base = base();
    let first = candidate(base.path(), false);
    let (storage, operation, mut intent) = begin(&first);
    for bytes in [MAX_BINARY, MAX_BINARY + 1] {
        let mut manifest: serde_json::Value = serde_json::from_str(&intent.manifest).unwrap();
        for file in manifest["files"].as_array_mut().unwrap() {
            file["bytes"] = serde_json::json!(bytes);
        }
        intent.manifest = serde_json::to_string(&manifest).unwrap();
        for change in &mut intent.changes[..2] {
            change.after.bytes = bytes;
        }
        intent.changes[2].after =
            record(&state::receipt(&first.root, intent.manifest.as_bytes()).unwrap());
        intent.save(&operation).unwrap();
        let loaded = transaction::Intent::load(&operation, &first.root);
        assert_eq!(loaded.is_ok(), bytes == MAX_BINARY, "{bytes}");
    }
    drop(storage);
}

#[test]
fn a_journal_before_state_must_match_the_prior_manifest() {
    let base = base();
    let first = candidate(base.path(), false);
    install(&first);
    let second = candidate(base.path(), true);
    let (storage, operation, mut intent) = begin(&second);
    intent.changes[0].before = Some(transaction::Record {
        bytes: 1,
        sha256: "b".repeat(64),
    });
    intent.save(&operation).unwrap();
    assert_eq!(
        load_error(&operation, &second.root),
        "artifact verification rejected: journal before-state differs from manifest"
    );
    drop(storage);
}

#[test]
fn journal_identity_and_file_records_are_checked_first() {
    let base = base();
    let first = candidate(base.path(), false);
    let (storage, operation, intent) = begin(&first);
    let original = serde_json::to_value(&intent).unwrap();
    for (pointer, replacement, message) in [
        (
            "/schema",
            serde_json::json!("rullst.cli-installation-intent.v2"),
            "invalid installation intent",
        ),
        (
            "/root",
            serde_json::json!(base.path().join("elsewhere")),
            "invalid installation intent",
        ),
        (
            "/approved_review",
            serde_json::json!("F".repeat(64)),
            "invalid installation intent",
        ),
        (
            "/changes/0/after/bytes",
            serde_json::json!(0),
            "invalid installation file record",
        ),
        (
            "/changes/0/after/sha256",
            serde_json::json!("G".repeat(64)),
            "invalid installation file record",
        ),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).unwrap() = replacement;
        storage::atomic_write(
            &operation,
            "intent.json",
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert_eq!(
            load_error(&operation, &first.root),
            format!("artifact verification rejected: {message}"),
            "{pointer}"
        );
    }
    drop(storage);
}

#[test]
fn matching_compares_the_exact_entry() {
    let base = base();
    let path = base.path().join("entry");
    transaction::matching(&path, &None).unwrap();
    let mut file = cache::new_installation_file(&path).unwrap();
    std::io::Write::write_all(&mut file, b"entry").unwrap();
    drop(file);
    assert_eq!(transaction::record(&path).unwrap(), Some(record(b"entry")));
    transaction::matching(&path, &Some(record(b"entry"))).unwrap();
    assert!(transaction::matching(&path, &None).is_err());
    assert!(transaction::matching(&path, &Some(record(b"other"))).is_err());
    assert!(transaction::matching(&base.path().join("missing"), &Some(record(b"entry"))).is_err());
    // Only a missing entry is absent; any other lookup failure is an error.
    #[cfg(unix)]
    assert!(transaction::record(&path.join("child")).is_err());
}

#[test]
fn an_installed_file_may_be_exactly_128_mib() {
    let base = base();
    let path = base.path().join("largest");
    cache::new_installation_file(&path)
        .unwrap()
        .set_len(MAX_BINARY)
        .unwrap();
    assert_eq!(
        transaction::record(&path).unwrap().unwrap().bytes,
        134_217_728
    );
}
