//! A verification record is reloaded only when every recorded fact matches.
use super::*;
use serde_json::{Value, json};
use std::fs;

const INVALID: &str = "project preparation rejected: invalid project verification record";
const CHANGED: &str = "project preparation rejected: verification command or status changed";

struct Record {
    _temp: tempfile::TempDir,
    directory: PathBuf,
    value: Value,
}

impl Record {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("verified");
        fs::create_dir(&directory).unwrap();
        let commands = verify::commands(&[], true)
            .iter()
            .enumerate()
            .map(|(index, command)| {
                let mut observation = json!({"program": command.program, "args": command.args,
                    "success": true, "stdout": format!("command-{index}.stdout"),
                    "stderr": format!("command-{index}.stderr")});
                for stream in ["stdout", "stderr"] {
                    let body = format!("{stream} of command {index}");
                    fs::write(directory.join(format!("command-{index}.{stream}")), &body).unwrap();
                    observation[format!("{stream}_sha256")] =
                        json!(hex::encode(Sha256::digest(body.as_bytes())));
                }
                observation
            })
            .collect::<Vec<_>>();
        let value = json!({
            "schema_version": "rullst.project-verification.v1", "phase": "verified",
            "prepared_directory": temp.path().join("prepared"), "verified_directory": directory,
            "verified_candidate_directory": directory.join("candidate"),
            "source": temp.path().join("source"), "target": "13.0.0",
            "platform": std::env::consts::OS, "features": [], "offline": true,
            "commands": commands, "files": [],
            "execution_authorized_for_this_invocation": true,
            "application_authorized": false, "production_ready": false,
        });
        Self {
            _temp: temp,
            directory,
            value,
        }
    }

    fn load(&self, value: &Value) -> Result<(Verified, String), ProjectError> {
        fs::write(
            self.directory.join("verification.json"),
            serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
        Verified::load(&self.directory)
    }

    fn error(&self, value: &Value) -> String {
        match self.load(value) {
            Ok(_) => panic!("the verification record was accepted"),
            Err(error) => error.to_string(),
        }
    }
}

#[test]
fn an_unchanged_record_loads_with_the_digest_of_its_bytes() {
    let record = Record::new();
    let (verified, digest) = record.load(&record.value).unwrap();
    assert_eq!(verified.target, "13.0.0");
    let bytes = fs::read(record.directory.join("verification.json")).unwrap();
    assert_eq!(digest, hex::encode(Sha256::digest(bytes)));
}

#[test]
fn every_identity_and_authority_field_is_checked() {
    let record = Record::new();
    let directory = record.directory.clone();
    for (field, value) in [
        ("schema_version", json!("rullst.project-verification.v2")),
        ("phase", json!("prepared")),
        ("platform", json!("plan9")),
        ("execution_authorized_for_this_invocation", json!(false)),
        ("application_authorized", json!(true)),
        ("production_ready", json!(true)),
        ("verified_directory", json!(directory.join("other"))),
        (
            "verified_candidate_directory",
            json!(directory.join("other")),
        ),
        ("prepared_directory", json!(directory)),
    ] {
        let mut changed = record.value.clone();
        changed[field] = value;
        assert_eq!(record.error(&changed), INVALID, "{field}");
    }
}

#[test]
fn a_record_may_list_at_most_100_000_files() {
    let record = Record::new();
    let file = json!({"path": "src/lib.rs", "bytes": 0, "sha256": null});
    let mut changed = record.value.clone();
    changed["files"] = Value::Array(vec![file.clone(); 100_000]);
    assert_eq!(record.load(&changed).unwrap().0.files.len(), 100_000);
    changed["files"].as_array_mut().unwrap().push(file);
    assert_eq!(record.error(&changed), INVALID);
}

#[test]
fn the_feature_policy_must_be_canonical() {
    let record = Record::new();
    let mut changed = record.value.clone();
    changed["features"] = json!(["--features", "serde", "--no-default-features"]);
    assert_eq!(
        record.error(&changed),
        "project preparation rejected: noncanonical verification feature policy"
    );
    changed["features"] = json!(["--target", "x86_64-unknown-linux-gnu"]);
    assert_eq!(
        record.error(&changed),
        "project preparation rejected: unsupported verification feature policy"
    );
}

#[test]
fn every_recorded_command_and_outcome_is_checked() {
    let record = Record::new();
    let mut changed = record.value.clone();
    changed["commands"].as_array_mut().unwrap().pop();
    assert_eq!(
        record.error(&changed),
        "project preparation rejected: incomplete verification command inventory"
    );
    for (field, value) in [
        ("program", json!("sh")),
        ("args", json!(["--version", "--verbose", "--extra"])),
        ("success", json!(false)),
        ("stdout", json!("command-0.stderr")),
        ("stderr", json!("command-0.stdout")),
    ] {
        let mut changed = record.value.clone();
        changed["commands"][0][field] = value;
        assert_eq!(record.error(&changed), CHANGED, "{field}");
    }
}

#[test]
fn logs_must_match_their_digest_and_stay_within_8_mib() {
    let record = Record::new();
    let mut changed = record.value.clone();
    changed["commands"][1]["stdout_sha256"] = json!(hex::encode(Sha256::digest(b"other")));
    let changed_log =
        "project preparation rejected: verification log changed or exceeds its byte bound";
    assert_eq!(record.error(&changed), changed_log);
    let largest = vec![b'l'; 8 * 1024 * 1024];
    let mut changed = record.value.clone();
    fs::write(record.directory.join("command-2.stderr"), &largest).unwrap();
    changed["commands"][2]["stderr_sha256"] = json!(hex::encode(Sha256::digest(&largest)));
    record.load(&changed).unwrap();
    let larger = [largest.as_slice(), b"l"].concat();
    fs::write(record.directory.join("command-2.stderr"), &larger).unwrap();
    changed["commands"][2]["stderr_sha256"] = json!(hex::encode(Sha256::digest(&larger)));
    assert_eq!(record.error(&changed), changed_log);
    fs::remove_file(record.directory.join("command-2.stderr")).unwrap();
    assert_eq!(
        record.error(&changed),
        "project preparation rejected: missing verification log"
    );
}

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"rullst\"\nversion = \"13.0.0\"\n";

/// A verification record of a prepared project's unchanged candidate.
fn verified_project() -> (super::super::tests::PreparedProject, State, Record) {
    let project = super::super::tests::prepared_project();
    let state = State::load(&project.stage).unwrap();
    let mut record = Record::new();
    let candidate = record.directory.join("candidate");
    for input in &state.candidate_records {
        if input.path != "Cargo.lock" && input.sha256.is_some() {
            let target = candidate.join(&input.path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(state.candidate.join(&input.path), target).unwrap();
        }
    }
    fs::write(candidate.join("Cargo.lock"), LOCK).unwrap();
    let files: Vec<_> = state
        .candidate_records
        .iter()
        .map(|input| {
            if input.path == "Cargo.lock" {
                snapshot::record("Cargo.lock", Some(LOCK.as_bytes()))
            } else {
                input.clone()
            }
        })
        .collect();
    record.value["source"] = json!(project.source);
    record.value["target"] = json!("13.0.0");
    record.value["files"] = json!(files);
    (project, state, record)
}

fn validation(record: &Record, value: &Value, state: &State) -> Result<(), ProjectError> {
    record.load(value).unwrap().0.validate(state)
}

#[test]
fn a_record_of_the_unchanged_candidate_validates() {
    let (_project, state, record) = verified_project();
    validation(&record, &record.value, &state).unwrap();
}

#[test]
fn the_verified_scope_must_be_the_preparation() {
    let (_project, state, record) = verified_project();
    let scope = "project preparation rejected: verification scope differs from preparation";
    for (field, value) in [
        ("source", json!(record.directory)),
        ("target", json!("13.0.1")),
    ] {
        let mut changed = record.value.clone();
        changed[field] = value;
        assert_eq!(
            validation(&record, &changed, &state)
                .unwrap_err()
                .to_string(),
            scope,
            "{field}"
        );
    }
    let mut fewer = record.value.clone();
    fewer["files"].as_array_mut().unwrap().pop();
    assert_eq!(
        validation(&record, &fewer, &state).unwrap_err().to_string(),
        scope
    );
    let mut renamed = record.value.clone();
    renamed["files"][2]["path"] = renamed["files"][1]["path"].clone();
    assert_eq!(
        validation(&record, &renamed, &state)
            .unwrap_err()
            .to_string(),
        "project preparation rejected: invalid verification file inventory"
    );
}

#[test]
fn verified_candidate_bytes_and_inventory_must_match() {
    let (_project, state, record) = verified_project();
    let candidate = record.directory.join("candidate");
    fs::write(candidate.join("src/main.rs"), "fn main() { changed() }\n").unwrap();
    let changed = "project preparation rejected: verified candidate bytes changed";
    assert_eq!(
        validation(&record, &record.value, &state)
            .unwrap_err()
            .to_string(),
        changed
    );
    // A record that matches the changed bytes still differs from the input.
    let mut rerecorded = record.value.clone();
    rerecorded["files"][2] = json!(snapshot::record(
        "src/main.rs",
        Some(b"fn main() { changed() }\n")
    ));
    assert_eq!(
        validation(&record, &rerecorded, &state)
            .unwrap_err()
            .to_string(),
        changed
    );
    let (_project, state, record) = verified_project();
    fs::write(record.directory.join("candidate/extra.rs"), "").unwrap();
    assert_eq!(
        validation(&record, &record.value, &state)
            .unwrap_err()
            .to_string(),
        "project preparation rejected: verified candidate inventory changed"
    );
}

#[test]
fn the_verified_lockfile_must_resolve_the_selected_release() {
    let (_project, state, record) = verified_project();
    let other = LOCK.replace("13.0.0", "13.0.1");
    fs::write(record.directory.join("candidate/Cargo.lock"), &other).unwrap();
    let mut changed = record.value.clone();
    changed["files"][0] = json!(snapshot::record("Cargo.lock", Some(other.as_bytes())));
    assert!(matches!(
        validation(&record, &changed, &state),
        Err(ProjectError::Planning(_))
    ));
}

#[test]
fn recorded_files_keep_the_preparation_order_and_byte_bound() {
    let (_project, state, record) = verified_project();
    let inventory = "project preparation rejected: invalid verification file inventory";
    let mut swapped = record.value.clone();
    let files = swapped["files"].as_array_mut().unwrap();
    files.swap(1, 2);
    assert_eq!(
        validation(&record, &swapped, &state)
            .unwrap_err()
            .to_string(),
        inventory
    );
    let mut sized = record.value.clone();
    sized["files"][0]["bytes"] = json!(snapshot::MAX_TOTAL);
    assert_eq!(
        validation(&record, &sized, &state).unwrap_err().to_string(),
        "project preparation rejected: verified candidate bytes changed"
    );
    sized["files"][0]["bytes"] = json!(snapshot::MAX_TOTAL + 1);
    assert_eq!(
        validation(&record, &sized, &state).unwrap_err().to_string(),
        inventory
    );
}
