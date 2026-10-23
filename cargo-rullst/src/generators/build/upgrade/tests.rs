//! Unit tests of the upgrade planner.

use super::*;

#[test]
fn target_must_match_the_installed_cli_major() {
    let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let incompatible = format!("{}.0.0", cli.major + 1);
    assert!(matches!(
        target_version(Some(&incompatible)),
        Err(UpgradeError::IncompatibleCli { .. })
    ));
}

#[test]
fn requirements_above_the_target_are_downgrades() {
    let target = Version::parse("13.0.0").unwrap();
    for newer in [
        "=13.2.0", "13.1", "^13.0.1", "~13.0.5", ">=14", ">13.0.0", "13.1.*",
    ] {
        assert!(requirement_exceeds(newer, &target), "{newer}");
    }
    for not_newer in [
        "=13.0.0",
        "13",
        "12",
        ">=12, <14",
        "<13.5",
        "*",
        "13.0.0-rc.1",
    ] {
        assert!(!requirement_exceeds(not_newer, &target), "{not_newer}");
    }
    // A stable requirement is above a prerelease CLI of the same version.
    let prerelease = Version::parse("13.0.0-alpha.1").unwrap();
    assert!(requirement_exceeds("13", &prerelease));
    assert!(!requirement_exceeds("13.0.0-alpha.1", &prerelease));
}

/// A removed provider is a review finding, so a failed gate used to give no
/// hint; it now names the finding and repeats its guidance.
#[test]
fn a_failed_gate_repeats_the_guidance_of_removed_api_findings() {
    let root = Path::new("/project");
    let finding = |rule: &'static rules::Rule, path: &str, line| rules::SourceFinding {
        rule,
        path: root.join(path),
        line,
    };
    let capital = rules::REMOVED_API_RULES
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-CAPITAL-REMOVED")
        .unwrap();
    let other = rules::REMOVED_API_RULES
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-MAIL-REMOVED")
        .unwrap();
    let scan = SourceScan {
        findings: vec![
            finding(capital, ".env.example", 21),
            finding(capital, "src/controllers/billing_controller.rs", 8),
            finding(other, "src/mail.rs", 3),
        ],
        ..SourceScan::default()
    };
    let hint = findings_hint(root, &scan);
    assert!(!hint.contains("must-change"), "{hint}");
    assert!(
        hint.contains("V13-CAPITAL-REMOVED at src/controllers/billing_controller.rs:8"),
        "{hint}"
    );
    assert!(!hint.contains(".env.example"), "{hint}");
    assert!(hint.contains("`StripeProvider`"), "{hint}");
    assert!(
        hint.contains("`InfinitePayProvider` is experimental"),
        "{hint}"
    );
    assert!(hint.contains("V13-MAIL-REMOVED at src/mail.rs:3"), "{hint}");
    assert!(findings_hint(root, &SourceScan::default()).is_empty());
}

#[test]
fn target_accepts_an_exact_prerelease_in_the_cli_train() {
    let cli = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let prerelease = format!("{}.1.0-rc.2", cli.major);
    assert_eq!(
        target_version(Some(&prerelease)).unwrap().to_string(),
        prerelease
    );
}

fn selected_scan(current: Option<&str>, all_rules: bool) -> SourceScan {
    let finding = |rule: &'static rules::Rule, line| rules::SourceFinding {
        rule,
        path: Path::new("/project/src/main.rs").to_path_buf(),
        line,
    };
    let page = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-RENDER-PAGE-LANGUAGE")
        .unwrap();
    let transaction = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-ORM-TRANSACTION-ERRORS")
        .unwrap();
    let mut scan = SourceScan {
        findings: vec![finding(page, 2), finding(transaction, 5)],
        ..SourceScan::default()
    };
    scan.select(
        current.map(|version| Version::parse(version).unwrap()),
        all_rules,
    );
    scan
}

/// An alpha.2 application's plan names its version, the skipped rules and
/// the flag that shows them; every other plan says nothing about it.
#[test]
fn the_plan_says_which_earlier_rules_it_skipped() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0-alpha.3").unwrap();
    let scan = selected_scan(Some("13.0.0-alpha.2"), false);
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(
        text.contains("  Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 1); --all-rules applies every rule\n"),
        "{text}"
    );
    assert!(text.contains("[V13-ORM-TRANSACTION-ERRORS]"), "{text}");
    assert!(!text.contains("[V13-RENDER-PAGE-LANGUAGE]"), "{text}");
    let markdown = report::render_report(root, &target, &[], &scan);
    assert!(
        markdown.contains("Rules: introduced after 13.0.0-alpha.2, the Rullst version this project uses; earlier rules skipped: 100 (findings hidden: 1); --all-rules applies every rule.\n\n"),
        "{markdown}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&report::render_json_report(root, &target, &[], &scan).unwrap())
            .unwrap();
    assert_eq!(
        json["rule_selection"],
        serde_json::json!({
            "current_version": "13.0.0-alpha.2",
            "all_rules": false,
            "skipped_rules": 100,
            "hidden_findings": 1,
        })
    );
    assert_eq!(
        json["source_findings"][0]["introduced_in"],
        "13.0.0-alpha.3"
    );

    for (current, all_rules) in [
        (Some("12.3.0"), false),
        (None, false),
        (Some("13.0.0-alpha.2"), true),
    ] {
        let scan = selected_scan(current, all_rules);
        let text = report::plan_text(root, &target, &[], &scan);
        assert!(!text.contains("Rules:"), "{text}");
        assert!(text.contains("[V13-RENDER-PAGE-LANGUAGE]"), "{text}");
        let markdown = report::render_report(root, &target, &[], &scan);
        assert!(!markdown.contains("Rules:"), "{markdown}");
    }
}

/// Must-change and review findings as the assistant receives them, and the
/// guide links that only a plan with findings carries.
#[test]
fn findings_reach_the_plan_text_and_the_assistant() {
    let root = Path::new("/project");
    let target = Version::parse("13.0.0").unwrap();
    let clean = report::plan_text(root, &target, &[], &SourceScan::default());
    assert!(!clean.contains("Migration guide:"), "{clean}");
    let scan = selected_scan(None, false);
    let text = report::plan_text(root, &target, &[], &scan);
    assert!(text.contains("Migration guide: "), "{text}");
    assert!(
        text.contains("Reviewed fixes for these findings: cargo rullst ai upgrade"),
        "{text}"
    );

    let handler = rules::CATALOG
        .iter()
        .copied()
        .find(|rule| rule.code == "V13-HTML-DYNAMIC-EVENT-HANDLER")
        .unwrap();
    let mut mixed = selected_scan(None, false);
    mixed.findings.push(rules::SourceFinding {
        rule: handler,
        path: root.join("src/view.rs"),
        line: 7,
    });
    let findings = report::assist_findings(root, &mixed);
    let summary: Vec<_> = findings
        .iter()
        .map(|finding| {
            (
                finding.code,
                finding.must_change,
                finding.path.as_str(),
                finding.line,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("V13-RENDER-PAGE-LANGUAGE", false, "src/main.rs", 2),
            ("V13-ORM-TRANSACTION-ERRORS", false, "src/main.rs", 5),
            ("V13-HTML-DYNAMIC-EVENT-HANDLER", true, "src/view.rs", 7),
        ]
    );
}

/// Runs a dry-run upgrade in the current directory when a parent test names
/// a report file; otherwise does nothing.
#[test]
fn upgrade_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_UPGRADE_REPORT") else {
        return;
    };
    let outcome = match run_upgrade(UpgradeOptions {
        target: Some("13.0.0".into()),
        dry_run: true,
        json: false,
        keep_on_failure: false,
        restore: None,
        all_rules: false,
    }) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    std::fs::write(report, outcome).unwrap();
}

#[test]
fn a_dry_run_prints_the_plan() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(project.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"12.3\", default-features = false }\n",
    )
    .unwrap();
    let report = project.path().join("report");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "generators::build::upgrade::tests::upgrade_child_fixture",
            "--nocapture",
        ])
        .current_dir(project.path())
        .env("NO_COLOR", "1")
        .env("RULLST_UPGRADE_REPORT", &report)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(std::fs::read_to_string(&report).unwrap(), "ok", "{stdout}");
    assert!(
        stdout.contains("Rullst assisted upgrade — DRY RUN — target 13.0.0"),
        "{stdout}"
    );
    assert!(stdout.contains("rullst (rullst) 12.3 -> "), "{stdout}");
}
