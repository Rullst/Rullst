use super::*;
use serde_json::{Value, json};

fn current() -> Version {
    Version::parse("12.0.0").unwrap()
}
fn release(version: &str) -> Value {
    json!({"crate":"cargo-rullst", "num":version, "yanked":false, "rust_version":"1.96.0", "checksum":"a".repeat(64)})
}
fn catalog(versions: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"versions":versions})).unwrap()
}
fn stable() -> Selection {
    Selection::new(&current(), None, false, false).unwrap()
}

#[test]
fn default_selects_highest_stable_same_major_and_explains_authority() {
    let report = resolve(
        &catalog(vec![
            release("13.0.0"),
            release("12.1.0"),
            release("12.2.0-rc.1"),
            release("12.0.0"),
        ]),
        &current(),
        &stable(),
    )
    .unwrap();
    assert_eq!(report.status, "update-available");
    let output = serde_json::to_value(&report).unwrap();
    assert_eq!(output["schema_version"], "rullst.update-discovery.v1");
    assert_eq!(output["target"]["version"], "12.1.0");
    assert_eq!(output["target"]["rust_version"], "1.96.0");
    assert_eq!(output["target"]["requires_target_major_cli"], false);
    assert!(
        output["authority"]
            .as_object()
            .unwrap()
            .values()
            .all(|value| value == &Value::Bool(false))
    );
}

#[test]
fn exact_major_and_prerelease_require_separate_explicit_choices() {
    for (target, major, pre) in [
        ("13.0.0", false, false),
        ("12.1.0-rc.1", false, false),
        ("13.0.0-rc.1", true, false),
        ("13.0.0-rc.1", false, true),
        ("11.9.9", true, false),
    ] {
        assert!(
            Selection::new(&current(), Some(target), major, pre).is_err(),
            "accepted {target}"
        );
    }
    assert!(Selection::new(&current(), None, true, false).is_err());
    assert!(Selection::new(&current(), None, false, true).is_err());
    let selection = Selection::new(&current(), Some("13.0.0-rc.1"), true, true).unwrap();
    let report = resolve(
        &catalog(vec![release("13.0.0-rc.1"), release("13.0.0")]),
        &current(),
        &selection,
    )
    .unwrap();
    let target = report.target.unwrap();
    assert_eq!(target.version, "13.0.0-rc.1");
    assert!(target.requires_target_major_cli);
}

#[test]
fn yanked_missing_or_foreign_exact_targets_cannot_be_substituted() {
    let mut yanked = release("12.1.0");
    yanked["yanked"] = json!(true);
    let mut foreign = release("12.1.0");
    foreign["crate"] = json!("other-cli");
    let selection = Selection::new(&current(), Some("12.1.0"), false, false).unwrap();
    for entries in [vec![yanked], vec![foreign], vec![release("12.2.0")]] {
        assert!(resolve(&catalog(entries), &current(), &selection).is_err());
    }
}

#[test]
fn duplicate_entries_and_missing_yank_status_fail_closed() {
    let mut missing_yank = release("12.1.0");
    missing_yank.as_object_mut().unwrap().remove("yanked");
    for entries in [
        vec![release("12.1.0"), release("12.1.0")],
        vec![missing_yank],
    ] {
        assert!(resolve(&catalog(entries), &current(), &stable()).is_err());
    }
}

#[test]
fn malformed_selected_metadata_and_terminal_controls_are_rejected() {
    for (key, value) in [
        ("rust_version", "1.96.0\u{1b}[2J"),
        ("rust_version", "1.96.0-beta.1"),
        ("rust_version", "0.1.0"),
        ("checksum", "fake"),
        ("checksum", "\u{1b}[2J"),
    ] {
        let mut candidate = release("12.1.0");
        candidate[key] = json!(value);
        assert!(resolve(&catalog(vec![candidate]), &current(), &stable()).is_err());
    }
    for target in ["12.1.0+metadata", "12.1.0\u{1b}[2J", "latest", "12.1"] {
        assert!(Selection::new(&current(), Some(target), false, false).is_err());
    }
}

#[test]
fn normalizes_two_component_msrv_but_does_not_invent_missing_metadata() {
    let mut candidate = release("12.1.0");
    candidate["rust_version"] = json!("1.96");
    let report = resolve(&catalog(vec![candidate.clone()]), &current(), &stable()).unwrap();
    assert_eq!(
        report.target.unwrap().rust_version.as_deref(),
        Some("1.96.0")
    );
    candidate.as_object_mut().unwrap().remove("rust_version");
    candidate.as_object_mut().unwrap().remove("checksum");
    let report = resolve(&catalog(vec![candidate]), &current(), &stable()).unwrap();
    let target = report.target.unwrap();
    assert!(target.rust_version.is_none());
    assert!(target.registry_checksum.is_none());
}

#[test]
fn already_current_and_no_eligible_release_are_not_updates() {
    assert_eq!(
        resolve(&catalog(vec![release("12.0.0")]), &current(), &stable())
            .unwrap()
            .status,
        "already-current"
    );
    assert_eq!(
        resolve(
            &catalog(vec![release("11.0.0"), release("13.0.0")]),
            &current(),
            &stable()
        )
        .unwrap()
        .status,
        "no-eligible-release"
    );
}

#[test]
fn a_prerelease_installation_can_discover_its_stable_release() {
    let installed = Version::parse("12.0.0-rc.1").unwrap();
    let selection = Selection::new(&installed, None, false, false).unwrap();
    let report = resolve(&catalog(vec![release("12.0.0")]), &installed, &selection).unwrap();
    assert_eq!(report.status, "update-available");
}

#[test]
fn the_catalog_limit_is_checked_before_parsing() {
    let body = vec![b' '; crate::ui::update_check::CATALOG_LIMIT as usize + 1];
    assert!(matches!(
        resolve(&body, &current(), &stable()),
        Err(SelectionError::Invalid("catalog exceeds 256 KiB"))
    ));
}
