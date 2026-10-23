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
