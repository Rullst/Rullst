//! The release that introduced each rule, and the plan's rule selection.

use super::*;

/// Every release the catalog dates rules with, oldest first.
const RELEASES: &[&str] = &[
    "13.0.0-alpha.1",
    "13.0.0-alpha.2",
    "13.0.0-alpha.3",
    "13.0.0-alpha.4",
    "13.0.0-alpha.5",
    "13.0.0-alpha.6",
    "13.0.0-rc.1",
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
        [
            "V13-DEPLOY-TARGETS-REMOVED",
            "V13-DEPLOY-GENERATORS-REMOVED",
            "V13-CLI-SELF-UPDATE-REMOVED",
            "V13-CLI-AI-REMOVED",
            "V13-ORM-FROM-ROW",
            "V13-SQLX-MACROS",
            "V13-NEXUS-AI-FEATURE",
            "V13-SQLX-TLS-RING"
        ]
    );
    // Rules added after alpha.4 for its changes take the next release,
    // 13.0.0-alpha.5, so an application already on alpha.4 still gets them.
    assert_eq!(
        codes_introduced_in("13.0.0-alpha.5"),
        [
            "V13-SQLX-TLS-NATIVE-ROOTS",
            "V13-DEPLOY-SQLITE-URL",
            "V13-DEPLOY-CADDY-HEALTH",
            "V13-CLI-MIGRATION-AUTO-STATUS",
            "V13-CLI-BLUEPRINTS-REMOVED",
            "V13-API-ALIAS-PATHS",
            "V13-HIDDEN-INTERNALS-PRIVATE",
            "V13-SECURITY-STORE-COLLECTIONS",
            "V13-ERROR-PAYLOADS",
            "V13-SECURITY-GLOBAL-COLLECTIONS",
            "V13-CONNECT-JWKS-KEY-SET",
            "V13-AI-HTTP-STATUS",
            "V13-ISLANDS-REMOVED",
            "V13-ISLAND-COMMANDS-REMOVED",
            "V13-LIVE-REMOVED",
            "V13-LIVE-COMMAND-REMOVED",
            "V13-SERVER-FUNCTIONS-REMOVED",
            "V13-FRONTEND-ADAPTERS-REMOVED",
            "V13-HTML-DYNAMIC-ALPINE-DIRECTIVE",
            "V13-IOT-OTA-STATE",
            "V13-IOT-CODEC-ERROR-MATCH",
            "V13-IOT-SENSOR-CARD-HTML",
            "V13-CAPITAL-INVOICE-MINOR-UNITS",
            "V13-ORM-ADAPTERS-REMOVED",
            "V13-MAIL-MESSAGE-REPLY-TO",
            "V13-PRIVACY-CONSENT-CHOICE-MATCH",
            "V13-LABS-NON-EXHAUSTIVE-MATCH",
            "V13-CLI-GREEN-RENAMED",
            "V13-ACADEMY-REMOVED",
            "V13-ORM-QDRANT-REMOVED",
            "V13-AI-PROVIDER-PRESETS",
            "V13-HTMX-2",
        ]
    );
    // The changes of the 13.0 freeze: the health probes' framework version,
    // CSRF static files by path, the removal of the APIs still deprecated,
    // the explicit ORM save mode, passkeys behind a feature, the
    // non-exhaustive public types, the dependencies kept out of the API,
    // chrono replaced by jiff and the scheduler's cron syntax for messaging.
    assert_eq!(
        codes_introduced_in("13.0.0-alpha.6"),
        [
            "V13-HEALTH-FRAMEWORK-VERSION",
            "V13-HEALTH-PROBE-VERSION-FIELD",
            "V13-CSRF-STATIC-PREFIXES",
            "V13-CSRF-COOKIE-IN-PAGE",
            "V13-ROUTER-BASELINE-LAYERS",
            "V13-DEPLOY-CADDY-ENCODE",
            "V13-MAIL-DEPRECATED-REMOVED",
            "V13-IOT-OTA-LEGACY-REMOVED",
            "V13-NEXUS-INFALLIBLE-BUILD-REMOVED",
            "V13-AI-DEPRECATED-REMOVED",
            "V13-SECURITY-DEPRECATED-REMOVED",
            "V13-QUEUE-PURGE-COMPLETED-REMOVED",
            "V13-CONNECT-INFALLIBLE-NEW-REMOVED",
            "V13-ROUTE-MARKER-REMOVED",
            "V13-ORM-SCHEMA-DIFF-REMOVED",
            "V13-ORM-SAVE-MODE",
            "V13-AUTH-PASSKEYS-FEATURE",
            "V13-MEDIA-PROCESSING-MATCH",
            "V13-NON-EXHAUSTIVE-MATCH",
            "V13-NON-EXHAUSTIVE-LITERAL",
            "V13-NON-EXHAUSTIVE-LITERAL-GLOB",
            "V13-SCHEDULER-CRON-FIELDS",
            "V13-CSRF-TOKEN-COMPARED",
            "V13-JSONWEBTOKEN-12",
            "V13-ED25519-DALEK-3",
            "V13-SQLITE-WAL-AVAILABLE",
            "V13-AUTH-SESSION-REGISTRY",
            "V13-TELEMETRY-LAYER-TYPE",
            "V13-VALIDATION-FIELD-ERRORS",
            "V13-STUDIO-OPENAPI-INFERRED",
            "V13-ORM-CHRONO-TYPES",
            "V13-LOCALE-JIFF-DATES",
            "V13-MAIL-JIFF-TIMESTAMPS",
            "V13-CAPITAL-INVOICE-JIFF-DATE",
            "V13-MESSAGING-CRON-POSIX",
        ]
    );
    // The maintainer-approved API changes of rc.1: no public name says
    // Experimental.
    assert_eq!(
        codes_introduced_in("13.0.0-rc.1"),
        ["V13-LABS-VARIANTS-RENAMED", "V13-IOT-SIMULATORS-REMOVED"]
    );
    let alpha4 = version("13.0.0-alpha.4");
    let alpha5 = version("13.0.0-alpha.5");
    for rule in CATALOG
        .iter()
        .filter(|rule| rule.introduced == "13.0.0-alpha.5")
    {
        assert!(rule.introduced_after(&alpha4), "{}", rule.code);
        assert!(!rule.introduced_after(&alpha5), "{}", rule.code);
    }
    // 95 until 13.0.0-alpha.6 removed `V13-STUDIO-UTOIPA-6`: Studio takes any
    // utoipa version's document.
    assert_eq!(codes_introduced_in("13.0.0-alpha.1").len(), 94);
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
        note: None,
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
            skipped_rules: 99,
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
        (94, 1)
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
