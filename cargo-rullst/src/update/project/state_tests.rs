//! A preparation is loaded only while its record, baseline, candidate and
//! source still match.
use super::*;
use crate::update::project::tests::prepared_project;
use std::fs;

const INVALID: &str =
    "project preparation rejected: invalid or foreign-platform preparation record";

fn error(stage: &Path) -> String {
    match State::load(stage) {
        Ok(_) => panic!("the preparation was accepted"),
        Err(error) => error.to_string(),
    }
}

fn record(stage: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(stage.join("preparation.json")).unwrap()).unwrap()
}

fn save(stage: &Path, value: &serde_json::Value) {
    fs::write(
        stage.join("preparation.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

#[test]
fn an_unchanged_preparation_loads_its_inputs() {
    let project = prepared_project();
    let state = State::load(&project.stage).unwrap();
    assert_eq!(state.prepared.target, "13.0.0");
    assert_eq!(state.candidate, project.stage.join("candidate"));
    assert_eq!(
        state.paths,
        BTreeSet::from([
            "Cargo.lock".to_string(),
            "Cargo.toml".to_string(),
            "src/main.rs".to_string(),
        ])
    );
    assert_eq!(state.candidate_records.len(), 3);
    state.unchanged().unwrap();
}

#[test]
fn every_identity_and_authority_field_of_the_record_is_checked() {
    let project = prepared_project();
    let original = record(&project.stage);
    let source = original["source"].as_str().unwrap().to_string();
    for (field, value) in [
        (
            "schema_version",
            serde_json::json!("rullst.project-preparation.v2"),
        ),
        ("phase", serde_json::json!("verified")),
        ("platform", serde_json::json!("plan9")),
        ("execution_authorized", serde_json::json!(true)),
        ("application_authorized", serde_json::json!(true)),
        ("files", serde_json::json!([])),
        ("source", serde_json::json!(format!("{source}/../source"))),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        save(&project.stage, &changed);
        assert_eq!(error(&project.stage), INVALID, "{field}");
    }
    let mut duplicate = original.clone();
    let first = duplicate["files"][0].clone();
    duplicate["files"].as_array_mut().unwrap().push(first);
    save(&project.stage, &duplicate);
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: duplicate preparation input"
    );
    let mut without_lock = original.clone();
    without_lock["files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["path"] != "Cargo.lock");
    save(&project.stage, &without_lock);
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: prepared file inventory changed"
    );
    let mut replanned = original.clone();
    replanned["plan"]["automatic_scope"] = serde_json::json!([]);
    save(&project.stage, &replanned);
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: preparation plan does not match the current migration catalog"
    );
}

#[test]
fn baseline_candidate_and_source_changes_are_refused() {
    let project = prepared_project();
    fs::write(project.stage.join("before/src/main.rs"), "changed").unwrap();
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: baseline bytes differ from preparation record"
    );
    let project = prepared_project();
    fs::write(project.stage.join("before/extra.rs"), "").unwrap();
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: prepared file inventory changed"
    );
    let project = prepared_project();
    fs::write(project.stage.join("candidate/extra.rs"), "").unwrap();
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: prepared file inventory changed"
    );
    let project = prepared_project();
    fs::write(project.stage.join("candidate/src/main.rs"), "changed").unwrap();
    assert_eq!(
        error(&project.stage),
        "project preparation rejected: candidate source changed; review the original and prepare again"
    );
    let project = prepared_project();
    fs::write(project.source.join("src/main.rs"), "changed").unwrap();
    assert!(State::load(&project.stage).is_err());
    State::load_for_recovery(&project.stage).unwrap();
}

#[test]
fn a_loaded_preparation_notices_later_changes() {
    let changed = |edit: &dyn Fn(&Path), message: &str| {
        let project = prepared_project();
        let state = State::load(&project.stage).unwrap();
        edit(&project.stage);
        assert_eq!(
            state.unchanged().unwrap_err().to_string(),
            format!("project preparation rejected: {message}")
        );
    };
    changed(
        &|stage| {
            let mut value = record(stage);
            value["target"] = serde_json::json!("13.0.1");
            save(stage, &value);
        },
        "preparation record changed during verification",
    );
    changed(
        &|stage| fs::write(stage.join("before/extra.rs"), "").unwrap(),
        "baseline inventory changed during verification",
    );
    changed(
        &|stage| fs::write(stage.join("before/src/main.rs"), "changed").unwrap(),
        "baseline changed during verification",
    );
    changed(
        &|stage| fs::write(stage.join("candidate/extra.rs"), "").unwrap(),
        "candidate inventory changed during verification",
    );
    changed(
        &|stage| fs::write(stage.join("candidate/src/main.rs"), "changed").unwrap(),
        "candidate changed during verification",
    );
}
