//! `--from` and the warning for a project already on the target version
//! (#823).

use super::*;

/// A scan of one alpha.1 and one alpha.3 finding selected for an
/// application on `current`, with `--from` when `from` is given.
fn scan_from(current: Option<&str>, from: Option<&str>) -> SourceScan {
    let mut scan = selected_scan(None, true);
    let parse = |version: &str| Version::parse(version).unwrap();
    scan.select(current.map(parse), from.map(parse), false);
    scan
}

/// A project already on the target version gets a warning that the
/// target's rules did not run, with the release before it as the example
/// of `--from`; every other plan gets none.
#[test]
fn a_project_already_on_the_target_is_warned_that_its_rules_did_not_run() {
    let root = Path::new("/project");
    let alpha3 = Version::parse("13.0.0-alpha.3").unwrap();
    let scan = scan_from(Some("13.0.0-alpha.3"), None);
    let warning = "this project already uses 13.0.0-alpha.3, the target version, so the rules introduced in 13.0.0-alpha.3 and earlier did not run, and a plan without findings says nothing about them; to apply the rules introduced since the version you upgraded from, run `cargo rullst upgrade --dry-run --from <VERSION>` with that version (for example `--from 13.0.0-alpha.2`), or `--all-rules` for every rule; a rule keeps the release that introduced it when a later release widens it, so the five alpha.6 rules that 13.0.0-rc.1 widened need `--from 13.0.0-alpha.5` or earlier";
    let text = report::plan_text(root, &alpha3, &[], &scan);
    assert!(
        text.contains(&format!(
            "Source findings (rullst-upgrade-rules-v4): 0 must-change, 0 review\n  Rules: introduced after 13.0.0-alpha.3, the Rullst version this project uses; earlier rules skipped: 106 (findings hidden: 2); --all-rules applies every rule\n  WARNING: {warning}"
        )),
        "{text}"
    );
    let markdown = report::render_report(root, &alpha3, &[], &scan);
    assert!(
        markdown.contains(&format!("**Warning:** {warning}.\n\n")),
        "{markdown}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &alpha3, &[], &scan).unwrap())
            .unwrap();
    assert_eq!(json["rule_selection"]["target_rules_skipped"], true);
    assert_eq!(
        json["rule_selection"]["from_version"],
        serde_json::Value::Null
    );

    // alpha.1 has no earlier rules, so there is no example version.
    let alpha1 = Version::parse("13.0.0-alpha.1").unwrap();
    assert_eq!(
        report::at_target_warning(&alpha1, &scan_from(Some("13.0.0-alpha.1"), None).selection)
            .as_deref(),
        Some(
            "this project already uses 13.0.0-alpha.1, the target version, so the rules introduced in 13.0.0-alpha.1 and earlier did not run, and a plan without findings says nothing about them; to apply the rules introduced since the version you upgraded from, run `cargo rullst upgrade --dry-run --from <VERSION>` with that version, or `--all-rules` for every rule; a rule keeps the release that introduced it when a later release widens it, so the five alpha.6 rules that 13.0.0-rc.1 widened need `--from 13.0.0-alpha.5` or earlier"
        )
    );

    // An rc.1 project upgraded from alpha.6: the example reruns the rc.1
    // rules only, so the warning also names the `--from` that reruns the
    // alpha.6 rules rc.1 widened (#896).
    let rc1 = Version::parse("13.0.0-rc.1").unwrap();
    let warning =
        report::at_target_warning(&rc1, &scan_from(Some("13.0.0-rc.1"), None).selection).unwrap();
    assert!(
        warning.contains("with that version (for example `--from 13.0.0-alpha.6`), or `--all-rules` for every rule; "),
        "{warning}"
    );
    assert!(
        warning.ends_with("the five alpha.6 rules that 13.0.0-rc.1 widened need `--from 13.0.0-alpha.5` or earlier"),
        "{warning}"
    );

    // An older project, `--from`, `--all-rules`, an unknown version, and a
    // selection that skipped nothing are not warned.
    let all = selected_scan(Some("13.0.0-alpha.3"), true);
    for scan in [
        scan_from(Some("13.0.0-alpha.2"), None),
        scan_from(Some("13.0.0-alpha.3"), Some("13.0.0-alpha.2")),
        scan_from(None, None),
        all,
    ] {
        assert_eq!(report::at_target_warning(&alpha3, &scan.selection), None);
        let text = report::plan_text(root, &alpha3, &[], &scan);
        assert!(!text.contains("WARNING"), "{text}");
        let json: serde_json::Value =
            serde_json::from_str(&report::render_json_report(root, &alpha3, &[], &scan).unwrap())
                .unwrap();
        assert_eq!(json["rule_selection"]["target_rules_skipped"], false);
    }
    let nothing_skipped = rules::RuleSelection {
        current: Some(Version::parse("12.3.0").unwrap()),
        ..rules::RuleSelection::default()
    };
    assert_eq!(
        report::at_target_warning(&Version::parse("12.3.0").unwrap(), &nothing_skipped),
        None
    );
}

/// With `--from`, the plan names that version and the project's, applies the
/// rules introduced after it and links the checklist for it.
#[test]
fn a_from_version_is_named_in_the_plan_and_selects_the_checklist() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0-alpha.3").unwrap();
    let scan = scan_from(Some("13.0.0-alpha.3"), Some("13.0.0-alpha.2"));
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(
        text.contains("  Rules: introduced after 13.0.0-alpha.2, passed with --from (this project uses 13.0.0-alpha.3); earlier rules skipped: 99 (findings hidden: 1); --all-rules applies every rule\n"),
        "{text}"
    );
    assert!(text.contains("[V13-ORM-TRANSACTION-ERRORS]"), "{text}");
    assert!(
        text.ends_with(&format!(
            "Migration guide: {}#{}",
            report::GUIDE,
            report::ALPHA2_CHECKLIST
        )),
        "{text}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &target, &[], &scan).unwrap())
            .unwrap();
    assert_eq!(json["rule_selection"]["from_version"], "13.0.0-alpha.2");
    assert_eq!(json["rule_selection"]["current_version"], "13.0.0-alpha.3");
    // Without a readable project version, only `--from` is named.
    let unknown = scan_from(None, Some("13.0.0-alpha.2"));
    let text = report::plan_text(root, &target, &[], &unknown);
    assert!(
        text.contains("  Rules: introduced after 13.0.0-alpha.2, passed with --from; earlier rules skipped: 99 (findings hidden: 1); --all-rules applies every rule\n"),
        "{text}"
    );
}

/// `--from` takes an exact version that is not newer than the project's.
#[test]
fn from_versions_are_exact_and_never_newer_than_the_project() {
    let version = |text: &str| Version::parse(text).unwrap();
    let alpha6 = version("13.0.0-alpha.6");
    assert_eq!(from_version(None, Some(&alpha6)).unwrap(), None);
    for (requested, current) in [
        ("13.0.0-alpha.5", Some(&alpha6)),
        ("13.0.0-alpha.6", Some(&alpha6)),
        ("13.0.0-alpha.6+build", Some(&alpha6)),
    ] {
        assert_eq!(
            from_version(Some(requested), current).unwrap(),
            Some(version(requested)),
            "{requested}"
        );
    }
    // Without a known project version every rule applies, so `--from` could
    // only skip some.
    assert_eq!(
        from_version(Some("13.0.0-alpha.1"), None)
            .unwrap_err()
            .to_string(),
        "`--from 13.0.0-alpha.1` would skip rules, but the Rullst version this project uses could not be read from its manifests or Cargo.lock, so every rule already applies; omit `--from`"
    );
    assert_eq!(
        from_version(Some("13.0"), Some(&alpha6))
            .unwrap_err()
            .to_string(),
        "invalid `--from` version `13.0`; use an exact SemVer version, such as 13.0.0-alpha.5"
    );
    assert_eq!(
        from_version(Some("13.0.0-rc.1"), Some(&alpha6))
            .unwrap_err()
            .to_string(),
        "`--from 13.0.0-rc.1` is newer than 13.0.0-alpha.6, the Rullst version this project uses, so it would skip rules the project has not been checked against; pass an earlier version, or omit `--from`"
    );
    // The planner checks `--from` against the project's version.
    let project = demo_project("=13.0.0-alpha.3");
    let root = project.path().canonicalize().unwrap();
    assert_eq!(
        plan_project(&root, Some("13.0.0"), false, Some("13.0.0-alpha.4"))
            .err()
            .map(|error| error.to_string())
            .as_deref(),
        Some(
            "`--from 13.0.0-alpha.4` is newer than 13.0.0-alpha.3, the Rullst version this project uses, so it would skip rules the project has not been checked against; pass an earlier version, or omit `--from`"
        )
    );
    let planned = plan_project(&root, Some("13.0.0"), false, Some("13.0.0-alpha.1")).unwrap();
    assert_eq!(planned.scan.selection.from, Some(version("13.0.0-alpha.1")));
    assert_eq!(
        planned.scan.selection.current,
        Some(version("13.0.0-alpha.3"))
    );
}
