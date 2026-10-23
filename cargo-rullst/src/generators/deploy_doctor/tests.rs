//! The diagnostic fails on a local error or an incomplete inspection only.
use super::*;
use std::fs;

const KEY: &str = "fixture-0123456789-ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[test]
fn only_failed_checks_record_a_local_error() {
    let mut report = Report::new("explicit_env_file");
    report.add("first", "PASS", "fine");
    report.add("second", "REVIEW", "look");
    assert!(!report.local_errors_found);
    report.add("third", "FAIL", "broken");
    report.add("fourth", "PASS", "fine");
    assert!(report.local_errors_found);
    assert_eq!(
        report
            .checks
            .iter()
            .map(|check| check.code)
            .collect::<Vec<_>>(),
        ["first", "second", "third", "fourth"]
    );
}

#[test]
fn a_complete_snapshot_without_local_errors_is_the_only_success() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let config = root.join("Rullst.toml");
    let env = root.join("selected.env");
    fs::write(&config, "[app]\nenv = 'development'\n").unwrap();
    fs::write(&env, format!("RULLST_ENV=production\nAPP_KEY={KEY}\n")).unwrap();
    let matches = command()
        .try_get_matches_from([
            "deploy:doctor",
            "--json",
            "--config",
            config.to_str().unwrap(),
            "--env-file",
            env.to_str().unwrap(),
        ])
        .unwrap();
    run(&matches).unwrap();
    // A complete inspection with a local error.
    fs::write(&env, "RULLST_ENV=production\nAPP_KEY=short\n").unwrap();
    assert!(matches!(run(&matches), Err(DoctorError::InspectionFailed)));
    // An inspection that could not complete.
    fs::remove_file(&env).unwrap();
    assert!(matches!(run(&matches), Err(DoctorError::InspectionFailed)));
}

fn snapshot(config: &str, environment: &[(&str, &str)]) -> input::Snapshot {
    input::Snapshot {
        config: rullst_core::config::RullstConfig::from_toml(config).unwrap(),
        config_present: true,
        unknown_config: false,
        environment: environment
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        environment_selected: true,
        staging: false,
    }
}

fn statuses(snapshot: &input::Snapshot) -> Vec<(&'static str, &'static str)> {
    let mut report = Report::new("explicit_env_file");
    evaluate::inspect(snapshot, &mut report);
    report
        .checks
        .iter()
        .map(|check| (check.code, check.status))
        .collect()
}

const PRODUCTION: [(&str, &str); 3] = [
    ("RULLST_ENV", "production"),
    ("APP_ENV", "production"),
    ("APP_KEY", KEY),
];

#[test]
fn a_clean_production_snapshot_passes_every_inspected_check() {
    assert_eq!(
        statuses(&snapshot("", &PRODUCTION)),
        [
            ("configuration_source", "PASS"),
            ("security_configuration", "PASS"),
            ("environment_target", "PASS"),
            ("application_key_obvious_errors", "PASS"),
        ]
    );
    let mut unselected = snapshot("", &PRODUCTION);
    unselected.environment_selected = false;
    assert_eq!(
        statuses(&unselected).last(),
        Some(&("environment_source", "NOT_INSPECTED"))
    );
    let mut staging = snapshot("", &PRODUCTION);
    staging.staging = true;
    assert!(statuses(&staging).contains(&("environment_target", "FAIL")));
}

#[test]
fn each_browser_policy_exception_is_reviewed_on_its_own() {
    for security in [
        "coep = 'unsafe-none'",
        "csrf_same_site = 'None'",
        "cors_allow_credentials = true",
    ] {
        let checks = statuses(&snapshot(&format!("[security]\n{security}\n"), &PRODUCTION));
        assert!(
            checks.contains(&("browser_policy_exceptions", "REVIEW")),
            "{security}: {checks:?}"
        );
    }
}

#[test]
fn obvious_application_key_mistakes_are_rejected() {
    let key_status = |key: &str| {
        let checks = statuses(&snapshot(
            "",
            &[("RULLST_ENV", "production"), ("APP_KEY", key)],
        ));
        checks
            .into_iter()
            .find(|(code, _)| *code == "application_key_obvious_errors")
            .map(|(_, status)| status)
            .unwrap()
    };
    let good = "0123456789-abcdefghijklmnopqrstu";
    assert_eq!(good.len(), 32);
    assert_eq!(key_status(good), "PASS");
    for bad in [
        &good[..31],
        "0123456789-abcdefghijklmnopqrst\u{7}v",
        "mock_0123456789abcdefghijklmnopqrstuv",
        "  CHANGE_ME_0123456789abcdefghijklmnop",
        &"z".repeat(40),
    ] {
        assert_eq!(key_status(bad), "FAIL", "{bad:?}");
    }
}
