use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

const TARGET: &str = "x86_64-unknown-linux-gnu";
const BODY: &[u8] = b"candidate must never be executed";

fn fixture() -> Value {
    json!({
        "schema":"rullst.cli-artifacts.v1", "version":"12.1.0", "target":TARGET,
        "source_commit":"a".repeat(40), "repository":"Rullst/Rullst",
        "release_tag":"v12.1.0", "build_runner":"ubuntu-22.04",
        "files":(["cargo-rullst", "rullst"].map(|binary| json!({
            "name":format!("{binary}-12.1.0-{TARGET}"), "executable":binary,
            "bytes":BODY.len(), "sha256":hex::encode(Sha256::digest(BODY))
        })))
    })
}

fn parse(value: &Value) -> Result<manifest::Manifest, ArtifactError> {
    manifest::Manifest::parse(
        &serde_json::to_vec(value).unwrap(),
        &Version::new(12, 1, 0),
        TARGET,
    )
}

pub(super) fn fixture_manifest() -> manifest::Manifest {
    parse(&fixture()).unwrap()
}

fn binaries(manifest: &manifest::Manifest) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    for binary in &manifest.files {
        fs::write(directory.path().join(&binary.name), BODY).unwrap();
    }
    directory
}

#[test]
fn release_and_file_identity_confusion_is_rejected() {
    parse(&fixture()).unwrap();
    for (pointer, replacement) in [
        ("/schema", json!("rullst.cli-artifacts.v2")),
        ("/version", json!("12.2.0")),
        ("/target", json!("x86_64-pc-windows-msvc")),
        ("/source_commit", json!("main")),
        ("/repository", json!("attacker/Rullst")),
        ("/release_tag", Value::Null),
        ("/release_tag", json!("v12.0.0")),
        ("/source_commit", json!("A".repeat(40))),
        ("/build_runner", json!("bad\nrunner")),
        ("/files", json!([])),
        ("/files/0/name", json!("../cargo-rullst")),
        ("/files/0/name", json!("C:\\escape")),
        ("/files/0/executable", json!("rullst")),
        ("/files/0/bytes", json!(0)),
        ("/files/0/bytes", json!(manifest::MAX_BINARY + 1)),
        ("/files/0/bytes", json!(-1)),
        ("/files/0/sha256", json!("bad")),
        ("/files/0/sha256", json!("G".repeat(64))),
    ] {
        let mut value = fixture();
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(parse(&value).is_err(), "accepted {pointer}");
    }
    let mut duplicate = fixture();
    duplicate["files"][1] = duplicate["files"][0].clone();
    assert!(parse(&duplicate).is_err());
    duplicate = fixture();
    duplicate["install_command"] = json!("unsafe");
    assert!(parse(&duplicate).is_err());
    let bytes = serde_json::to_string(&fixture()).unwrap();
    let duplicate_field = bytes.replacen('{', "{\"version\":\"12.1.0\",", 1);
    assert!(
        manifest::Manifest::parse(duplicate_field.as_bytes(), &Version::new(12, 1, 0), TARGET)
            .is_err()
    );
}

#[test]
fn all_native_platform_filenames_have_an_exact_contract() {
    for target in [
        TARGET,
        "x86_64-pc-windows-msvc",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ] {
        let mut value = fixture();
        value["target"] = json!(target);
        let suffix = if target.ends_with("windows-msvc") {
            ".exe"
        } else {
            ""
        };
        for (i, binary) in ["cargo-rullst", "rullst"].iter().enumerate() {
            value["files"][i]["name"] = json!(format!("{binary}-12.1.0-{target}{suffix}"));
        }
        manifest::Manifest::parse(
            &serde_json::to_vec(&value).unwrap(),
            &Version::new(12, 1, 0),
            target,
        )
        .unwrap();
    }
}

#[test]
fn digest_verification_rejects_changed_missing_empty_and_oversized_files() {
    let manifest = fixture_manifest();
    let directory = binaries(&manifest);
    manifest.verify_files(directory.path()).unwrap();
    let binary = directory.path().join(&manifest.files[0].name);
    let mut changed = BODY.to_vec();
    changed[0] ^= 1;
    fs::write(&binary, changed).unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
    fs::write(&binary, []).unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
    fs::File::create(&binary)
        .unwrap()
        .set_len(manifest::MAX_BINARY + 1)
        .unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
    fs::remove_file(&binary).unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
    fs::create_dir(&binary).unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
}

#[test]
fn manifest_bound_and_report_authority_are_explicit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("manifest");
    fs::write(&path, vec![b' '; 16 * 1024 + 1]).unwrap();
    assert!(files::read_bounded(&path, 16 * 1024).is_err());
    assert!(
        manifest::Manifest::parse(&vec![b' '; 16 * 1024 + 1], &Version::new(12, 1, 0), TARGET)
            .is_err()
    );
    let report = serde_json::to_value(Report::new(fixture_manifest())).unwrap();
    assert_eq!(report["schema_version"], "rullst.update-verification.v1");
    for (field, value) in report["authority"].as_object().unwrap() {
        assert_eq!(value, &(field == "artifact_verified"), "{field}");
    }
}

#[cfg(unix)]
#[test]
fn symlinked_binaries_directories_and_fifos_are_rejected_without_blocking() {
    use std::os::unix::fs::symlink;
    let manifest = fixture_manifest();
    let directory = binaries(&manifest);
    let file = directory.path().join(&manifest.files[0].name);
    fs::remove_file(&file).unwrap();
    symlink(directory.path().join(&manifest.files[1].name), &file).unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
    let alias = directory.path().join("alias");
    symlink(directory.path(), &alias).unwrap();
    assert!(files::directory(&alias).is_err());
    fs::remove_file(&file).unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &file,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR,
        0,
    )
    .unwrap();
    assert!(manifest.verify_files(directory.path()).is_err());
}
