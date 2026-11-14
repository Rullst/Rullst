//! Must-change findings stop an upgrade before it changes anything, and the
//! plan links the guide section for the project's version.

use super::*;

fn rule(code: &str) -> &'static rules::Rule {
    rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == code)
        .unwrap()
}

fn finding(code: &str, path: &str, line: usize) -> rules::SourceFinding {
    rules::SourceFinding {
        rule: rule(code),
        path: Path::new("/project").join(path),
        line,
    }
}

fn scan_of(findings: Vec<rules::SourceFinding>) -> SourceScan {
    SourceScan {
        findings,
        ..SourceScan::default()
    }
}

#[test]
fn the_refusal_names_each_rule_its_first_locations_and_its_guidance() {
    let root = Path::new("/project");
    let review = scan_of(vec![finding("V13-RENDER-PAGE-LANGUAGE", "src/main.rs", 2)]);
    assert!(must_change_refusal(root, &review).is_none());
    assert!(must_change_refusal(root, &SourceScan::default()).is_none());

    let mut findings = (1..=5)
        .map(|line| finding("V13-ORM-FROM-ROW", "src/models/user.rs", line))
        .collect::<Vec<_>>();
    findings.push(finding("V13-RENDER-PAGE-LANGUAGE", "src/main.rs", 2));
    findings.push(finding("V13-HTML-DYNAMIC-EVENT-HANDLER", "src/view.rs", 7));
    let refusal = must_change_refusal(root, &scan_of(findings)).unwrap();
    let from_row = rule("V13-ORM-FROM-ROW");
    let handler = rule("V13-HTML-DYNAMIC-EVENT-HANDLER");
    assert_eq!(
        refusal.to_string(),
        format!(
            "the plan lists 6 must-change finding(s), so nothing was changed: V13-ORM-FROM-ROW at src/models/user.rs:1, src/models/user.rs:2, src/models/user.rs:3 and 2 more: {}; V13-HTML-DYNAMIC-EVENT-HANDLER at src/view.rs:7: {}. Fix them and run `cargo rullst upgrade` again, or run `cargo rullst upgrade --keep-on-failure` to change the versions anyway and fix them in the kept files",
            from_row.guidance, handler.guidance
        )
    );
    let three = (1..=3)
        .map(|line| finding("V13-ORM-FROM-ROW", "src/models/user.rs", line))
        .collect::<Vec<_>>();
    let refusal = must_change_refusal(root, &scan_of(three))
        .unwrap()
        .to_string();
    assert!(
        refusal.contains("src/models/user.rs:3: Remove `FromRow`"),
        "{refusal}"
    );
    assert!(!refusal.contains("more"), "{refusal}");
}

#[test]
fn the_plan_says_that_must_change_findings_stop_the_upgrade() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0").unwrap();
    let note = "1 must-change finding(s) stop `cargo rullst upgrade` before it changes anything: fix them first, or pass --keep-on-failure to change the versions anyway and fix them in the kept files.";
    let blocked = scan_of(vec![finding("V13-ORM-FROM-ROW", "src/main.rs", 1)]);
    let text = report::plan_text(root, &target, &[], &blocked);
    assert!(
        text.contains(&format!("\n{note}\nMigration guide: ")),
        "{text}"
    );
    let review = scan_of(vec![finding("V13-RENDER-PAGE-LANGUAGE", "src/main.rs", 2)]);
    let text = report::plan_text(root, &target, &[], &review);
    assert!(!text.contains("stop `cargo rullst upgrade`"), "{text}");
}

fn selection(current: Option<&str>, all_rules: bool) -> rules::RuleSelection {
    rules::RuleSelection {
        current: current.map(|version| Version::parse(version).unwrap()),
        all_rules,
        ..rules::RuleSelection::default()
    }
}

#[test]
fn the_guide_link_follows_the_project_version() {
    let table = rules::MIGRATION_GUIDE_URL;
    assert!(table.starts_with(&format!("{}#", report::GUIDE)), "{table}");
    let alpha2 = format!("{}#{}", report::GUIDE, report::ALPHA2_CHECKLIST);
    let alpha3 = format!("{}#{}", report::GUIDE, report::ALPHA3_CHECKLIST);
    let alpha4 = format!("{}#{}", report::GUIDE, report::ALPHA4_CHECKLIST);
    for (current, expected) in [
        (None, table.to_string()),
        (Some("12.3.0"), table.to_string()),
        (Some("13.0.0-alpha.0"), table.to_string()),
        (Some("13.0.0-alpha.1"), table.to_string()),
        (Some("13.0.0-alpha"), table.to_string()),
        (Some("13.0.0-alpha.x"), table.to_string()),
        (Some("13.0.0-alpha.2"), alpha2.clone()),
        (Some("13.0.0-alpha.3"), alpha3.clone()),
        (Some("13.0.0-alpha.4"), alpha4.clone()),
        (Some("13.0.0-alpha.5"), alpha4.clone()),
        (Some("13.0.0-beta.1"), alpha4.clone()),
        (Some("13.0.0-rc.1"), alpha4.clone()),
        (Some("13.0.0"), table.to_string()),
        (Some("13.1.0-rc.1"), table.to_string()),
        (Some("13.0.1-rc.1"), table.to_string()),
        (Some("14.0.0-alpha.3"), table.to_string()),
    ] {
        assert_eq!(
            report::guide_url(&selection(current, false)),
            expected,
            "{current:?}"
        );
    }
    // --all-rules reports the 12.x changes too, which the table lists.
    assert_eq!(
        report::guide_url(&selection(Some("13.0.0-alpha.3"), true)),
        table
    );
}

/// The plan, the Markdown report and the JSON link the checklist of an
/// alpha.3 project; findings keep pointing at the table that holds their rows.
#[test]
fn every_rendering_links_the_checklist_of_the_project_version() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0-alpha.4").unwrap();
    let mut scan = scan_of(vec![finding("V13-ORM-FROM-ROW", "src/main.rs", 1)]);
    scan.selection = selection(Some("13.0.0-alpha.3"), false);
    let checklist = format!("{}#{}", report::GUIDE, report::ALPHA3_CHECKLIST);
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(
        text.ends_with(&format!("Migration guide: {checklist}")),
        "{text}"
    );
    let markdown = report::render_report(root, &target, &[], &scan);
    assert!(
        markdown.contains(&format!(
            "Checklist for this project's Rullst version: {checklist}\n\n"
        )),
        "{markdown}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &target, &[], &scan).unwrap())
            .unwrap();
    assert_eq!(json["migration_guide"], checklist.as_str());
    assert_eq!(
        json["source_findings"][0]["migration_url"],
        rules::MIGRATION_GUIDE_URL
    );

    scan.selection = selection(Some("12.3.0"), false);
    let markdown = report::render_report(root, &target, &[], &scan);
    assert!(!markdown.contains("Checklist for"), "{markdown}");
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(
        text.ends_with(&format!("Migration guide: {}", rules::MIGRATION_GUIDE_URL)),
        "{text}"
    );
}

/// mdBook's heading ids: lower case, spaces as `-`, other punctuation dropped.
fn anchor(heading: &str) -> String {
    heading
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|character| match character {
            ' ' => Some('-'),
            '-' | '_' => Some(character),
            other if other.is_alphanumeric() => Some(other),
            _ => None,
        })
        .collect()
}

#[test]
fn the_linked_sections_exist_in_the_migration_guide() {
    assert_eq!(
        anchor("Upgrading an alpha.3 application to alpha.4"),
        "upgrading-an-alpha3-application-to-alpha4"
    );
    assert_eq!(
        anchor("Upgrading an alpha.4 application to alpha.5"),
        report::ALPHA4_CHECKLIST
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/src/migration-v13.md");
    // Packaged crates do not ship the repository documentation.
    let Ok(guide) = std::fs::read_to_string(path) else {
        return;
    };
    let anchors = guide
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(anchor)
        .collect::<Vec<_>>();
    let table = rules::MIGRATION_GUIDE_URL
        .rsplit_once('#')
        .map(|(_, anchor)| anchor)
        .unwrap();
    for expected in [
        table,
        report::ALPHA2_CHECKLIST,
        report::ALPHA3_CHECKLIST,
        report::ALPHA4_CHECKLIST,
    ] {
        assert!(
            anchors.iter().any(|anchor| anchor == expected),
            "#{expected} is not a section of migration-v13.md: {anchors:?}"
        );
    }
}

const FROM_ROW_MODEL: &str = "#[derive(FromRow, Orm)]\nstruct User { id: i32 }\nfn main() {}\n";

/// A project on 12.3 whose only source has a must-change finding.
fn blocked_project() -> tempfile::TempDir {
    let project = demo_project("12.3");
    std::fs::write(project.path().join("src/main.rs"), FROM_ROW_MODEL).unwrap();
    project
}

#[test]
fn a_dry_run_says_the_must_change_findings_stop_the_upgrade() {
    let project = blocked_project();
    let (outcome, stdout) = run_child(project.path(), "dry-run", &[]);
    assert_eq!(outcome, "ok", "{stdout}");
    assert!(
        stdout.contains("MUST-CHANGE src/main.rs:1 [V13-ORM-FROM-ROW]"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "1 must-change finding(s) stop `cargo rullst upgrade` before it changes anything"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("Dry run complete"), "{stdout}");
}

/// The refusal comes before the backup, the version edits and the Cargo
/// gates; the stand-in `cargo` would let both gates pass.
#[cfg(unix)]
#[test]
fn must_change_findings_stop_the_upgrade_before_any_change() {
    let project = blocked_project();
    let tools = tempfile::tempdir().unwrap();
    let path = stub_cargo(project.path(), tools.path());
    let environment = [
        ("PATH", path.as_os_str()),
        ("RULLST_STUB_FIX", "0".as_ref()),
        ("RULLST_STUB_CHECK", "0".as_ref()),
    ];
    let (outcome, stdout) = run_child(project.path(), "apply", &environment);
    assert!(
        outcome.starts_with(
            "the plan lists 1 must-change finding(s), so nothing was changed: V13-ORM-FROM-ROW at src/main.rs:1: Remove `FromRow`"
        ),
        "{outcome}\n{stdout}"
    );
    assert!(
        !stdout.contains("Applying compiler-provided migrations"),
        "{stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("12.3")
    );
    assert!(!project.path().join("target/rullst-upgrades").exists());

    // --keep-on-failure changes the versions anyway, as the alpha.3 to
    // alpha.4 checklist does for `FromRow`.
    let (outcome, stdout) = run_child(project.path(), "keep", &environment);
    assert_eq!(outcome, "ok", "{stdout}");
    assert_eq!(
        std::fs::read_to_string(project.path().join("Cargo.toml")).unwrap(),
        demo_manifest("=13.0.0")
    );
}
