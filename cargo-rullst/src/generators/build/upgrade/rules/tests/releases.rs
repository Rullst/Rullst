//! The release that introduced each rule, and the plan's rule selection.

use super::*;

/// Every release the catalog dates rules with, oldest first.
const RELEASES: &[&str] = &[
    "13.0.0-alpha.1",
    "13.0.0-alpha.2",
    "13.0.0-alpha.3",
    "13.0.0-alpha.4",
];

fn version(text: &str) -> Version {
    Version::parse(text).unwrap()
}

fn codes_introduced_in(release: &str) -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|rule| rule.introduced == release)
        .map(|rule| rule.code)
        .collect()
}

#[test]
fn every_catalog_rule_has_a_valid_release() {
    let parsed = RELEASES
        .iter()
        .map(|release| version(release))
        .collect::<Vec<_>>();
    assert!(parsed.windows(2).all(|pair| pair[0] < pair[1]));
    for rule in CATALOG {
        let introduced = Version::parse(rule.introduced)
            .unwrap_or_else(|error| panic!("{}: {error}", rule.code));
        assert_eq!(introduced.to_string(), rule.introduced, "{}", rule.code);
        assert!(
            RELEASES.contains(&rule.introduced),
            "{} names an unknown release {}",
            rule.code,
            rule.introduced
        );
    }
}

/// The rules added, or reworded for a new change, after 13.0.0-alpha.1. The
/// others report the v12 → v13 changes that alpha.1 already contained.
#[test]
fn rules_carry_the_release_of_their_change() {
    assert_eq!(
        codes_introduced_in("13.0.0-alpha.2"),
        [
            "V13-HTML-MARKUP-RETURN",
            "V13-HTML-RAW-MARKUP",
            "V13-ORM-CONSTRAINT-ERRORS",
            "V13-TEXT-TIMESTAMPS",
            "V13-POLYGLOT-FEATURE",
        ]
    );
    assert_eq!(
        codes_introduced_in("13.0.0-alpha.3"),
        [
            "V13-CONNECT-GITHUB-EMAIL",
            "V13-ORM-TRANSACTION-ERRORS",
            "V13-ORM-SKIP-MANAGED-COLUMNS",
            "V13-ORM-DEFAULT-FEATURES",
            "V13-RULLST-DEFAULT-FEATURES",
            "V13-OMNI-REMOVED",
            "V13-OFFLINE-SYNC-REMOVED",
        ]
    );
    assert_eq!(
        codes_introduced_in("13.0.0-alpha.4"),
        ["V13-CLI-SELF-UPDATE-REMOVED", "V13-CLI-EXTRAS"]
    );
    assert_eq!(codes_introduced_in("13.0.0-alpha.1").len(), 95);
    assert!(codes_introduced_in("13.0.0-alpha.1").contains(&"V13-RENDER-PAGE-LANGUAGE"));
}

#[test]
fn a_rule_is_newer_only_than_earlier_versions() {
    let rule = &catalog::ORM_CONSTRAINT_ERRORS;
    assert_eq!(rule.introduced, "13.0.0-alpha.2");
    assert!(rule.introduced_after(&version("12.3.0")));
    assert!(rule.introduced_after(&version("13.0.0-alpha.1")));
    assert!(!rule.introduced_after(&version("13.0.0-alpha.2")));
    assert!(!rule.introduced_after(&version("13.0.0-alpha.2+local")));
    assert!(!rule.introduced_after(&version("13.0.0-alpha.3")));
    assert!(!rule.introduced_after(&version("13.0.0")));
    let undated = Rule {
        introduced: "alpha",
        ..catalog::ORM_CONSTRAINT_ERRORS
    };
    assert!(undated.introduced_after(&version("13.0.0")));
}

/// One finding of an alpha.1, an alpha.2 and an alpha.3 rule.
fn three_release_scan() -> SourceScan {
    let finding = |rule: &'static Rule, line| SourceFinding {
        rule,
        path: PathBuf::from("src/main.rs"),
        line,
    };
    SourceScan {
        findings: vec![
            finding(&catalog::PAGE_LANGUAGE, 1),
            finding(&catalog::ORM_CONSTRAINT_ERRORS, 2),
            finding(&catalog::ORM_TRANSACTION_ERRORS, 3),
        ],
        ..SourceScan::default()
    }
}

fn selected(current: Option<&str>, all_rules: bool) -> (Vec<&'static str>, RuleSelection) {
    let mut scan = three_release_scan();
    scan.select(current.map(version), all_rules);
    let codes = scan
        .findings
        .iter()
        .map(|finding| finding.rule.code)
        .collect();
    (codes, scan.selection)
}

#[test]
fn a_prerelease_application_sees_only_the_rules_introduced_after_it() {
    let (codes, selection) = selected(Some("13.0.0-alpha.2"), false);
    assert_eq!(codes, ["V13-ORM-TRANSACTION-ERRORS"]);
    assert_eq!(
        selection,
        RuleSelection {
            current: Some(version("13.0.0-alpha.2")),
            all_rules: false,
            skipped_rules: 100,
            hidden_findings: 2,
        }
    );

    let (codes, selection) = selected(Some("13.0.0-alpha.1"), false);
    assert_eq!(
        codes,
        ["V13-ORM-CONSTRAINT-ERRORS", "V13-ORM-TRANSACTION-ERRORS"]
    );
    assert_eq!(
        (selection.skipped_rules, selection.hidden_findings),
        (95, 1)
    );
}

#[test]
fn v12_unknown_versions_and_all_rules_keep_every_finding() {
    let every = [
        "V13-RENDER-PAGE-LANGUAGE",
        "V13-ORM-CONSTRAINT-ERRORS",
        "V13-ORM-TRANSACTION-ERRORS",
    ];
    for (current, all_rules) in [
        (Some("12.3.0"), false),
        (None, false),
        (Some("13.0.0-alpha.2"), true),
    ] {
        let (codes, selection) = selected(current, all_rules);
        assert_eq!(codes, every, "{current:?} {all_rules}");
        assert_eq!(selection.skipped_rules, 0, "{current:?} {all_rules}");
        assert_eq!(selection.hidden_findings, 0, "{current:?} {all_rules}");
        assert_eq!(selection.all_rules, all_rules);
        assert_eq!(selection.current, current.map(version));
    }
}
