//! A recovery reads an application intent only within 8 MiB and only when it
//! describes exactly the reviewed operation.
use super::*;

const MISMATCH: &str =
    "project preparation rejected: application intent does not match the reviewed operation";

fn digest(project: &CachedProject) -> String {
    review(project)["review_sha256"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn an_intent_over_8_mib_is_refused_before_parsing() {
    let project = cached_project();
    let digest = digest(&project);
    let intent = project.verified.join("application.json");
    fs::write(&intent, " ".repeat(8 * 1024 * 1024)).unwrap();
    let error = apply(&project, "recover", &digest).unwrap_err();
    assert!(!error.contains("exceeds"), "{error}");
    fs::write(&intent, " ".repeat(8 * 1024 * 1024 + 1)).unwrap();
    assert_eq!(
        apply(&project, "recover", &digest).unwrap_err(),
        "project preparation rejected: application intent exceeds 8 MiB"
    );
}

#[test]
fn every_field_of_the_intent_must_match_the_review() {
    let project = cached_project();
    let digest = digest(&project);
    apply(&project, "apply", &digest).unwrap();
    let path = project.verified.join("application.json");
    let original: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (field, value) in [
        (
            "schema_version",
            json!("rullst.project-application-intent.v2"),
        ),
        ("phase", json!("planned")),
        ("source", json!(project.prepared)),
        ("prepared_directory", json!(project.verified)),
        ("verified_directory", json!(project.prepared)),
        ("receipt_sha256", json!("0".repeat(64))),
        ("review_sha256", json!("0".repeat(64))),
        ("changes", json!([])),
        ("permissions", json!([])),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(
            apply(&project, "recover", &digest).unwrap_err(),
            MISMATCH,
            "{field}"
        );
    }
    fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(
        apply(&project, "recover", &digest).unwrap()["phase"],
        "restored"
    );
}
