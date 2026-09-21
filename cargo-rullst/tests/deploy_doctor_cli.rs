//! Real executable contracts: source isolation, bounded input and redacted output.
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const KEY: &str = "fixture-0123456789-ABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rullst"));
    command
        .current_dir(root)
        .env_remove("RULLST_ENV")
        .env_remove("APP_ENV")
        .env_remove("APP_KEY");
    command
}

fn run(root: &Path, args: &[&str]) -> (Output, Value) {
    let output = command(root)
        .args(["deploy:doctor", "--json"])
        .args(args)
        .output()
        .unwrap();
    let value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "invalid report: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output, value)
}

fn status<'a>(value: &'a Value, code: &str) -> Option<&'a str> {
    value["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["code"] == code)
        .and_then(|check| check["status"].as_str())
}

fn env(root: &Path, prefix: &str) {
    fs::write(
        root.join("selected.env"),
        format!("{prefix}\nAPP_KEY={KEY}\n"),
    )
    .unwrap();
}

#[test]
fn explicit_sources_are_isolated_and_reports_are_deterministic_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let config = "[app]\nenv = 'development'\n[database]\nurl = 'postgres://private-user:private-password@private.example/db'\n";
    fs::write(root.join("Rullst.toml"), config).unwrap();
    fs::write(root.join(".env"), "INVALID=\"unterminated").unwrap();
    env(
        root,
        "export RULLST_ENV='prod' # reviewed\nAPP_ENV=development\nIGNORED_KEY='private-other-value'",
    );
    let selected_before = fs::read(root.join("selected.env")).unwrap();
    let output = command(root)
        .env("RULLST_ENV", "development")
        .env("APP_KEY", "mock_should_not_override_snapshot")
        .args(["deploy:doctor", "--json", "--env-file", "selected.env"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], "rullst.deployment-diagnostic.v1");
    assert_eq!(value["deployment_verified"], false);
    assert_eq!(value["environment_source"], "explicit_env_file");
    assert_eq!(value["inspection_complete"], true);
    assert_eq!(status(&value, "environment_target"), Some("PASS"));
    assert_eq!(
        status(&value, "legacy_environment_conflict"),
        Some("REVIEW")
    );
    assert!(value["not_inspected"].as_array().unwrap().len() >= 9);
    let (again, _) = run(root, &["--env-file", "selected.env"]);
    assert_eq!(output.stdout, again.stdout);
    for secret in [
        KEY,
        "private-password",
        "private.example",
        "private-other-value",
        "mock_should_not_override_snapshot",
    ] {
        assert!(!String::from_utf8_lossy(&output.stdout).contains(secret));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
    }
    assert_eq!(
        fs::read_to_string(root.join("Rullst.toml")).unwrap(),
        config
    );
    assert_eq!(
        fs::read(root.join("selected.env")).unwrap(),
        selected_before
    );
    assert_eq!(fs::read_dir(root).unwrap().count(), 3);
    let (output, value) = run(root, &[]);
    assert!(!output.status.success());
    assert_eq!(value["inspection_complete"], false);
    assert_eq!(status(&value, "environment_source"), Some("NOT_INSPECTED"));
}

#[test]
fn process_selection_never_loads_dotenv_and_core_precedence_and_targets_are_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(
        root.join(".env"),
        format!("RULLST_ENV=production\nAPP_KEY={KEY}\n"),
    )
    .unwrap();
    let (missing, value) = run(root, &["--process-env"]);
    assert!(!missing.status.success());
    assert_eq!(
        status(&value, "application_key_obvious_errors"),
        Some("FAIL")
    );
    for (first, second, target, expected) in [
        (Some("staging"), None, "staging", true),
        (Some("production"), Some("development"), "production", true),
        (Some(""), Some("production"), "production", false),
        (
            Some("invalid-private-value"),
            Some("production"),
            "production",
            false,
        ),
        (None, Some("prod"), "production", true),
        (Some("staging"), None, "production", false),
    ] {
        let mut child = command(root);
        child.env("APP_KEY", KEY).args([
            "deploy:doctor",
            "--json",
            "--process-env",
            "--target",
            target,
        ]);
        if let Some(first) = first {
            child.env("RULLST_ENV", first);
        }
        if let Some(second) = second {
            child.env("APP_ENV", second);
        }
        let output = child.output().unwrap();
        assert_eq!(output.status.success(), expected);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("invalid-private-value"));
    }
    fs::write(root.join("Rullst.toml"), "[app]\nenv='production'\n").unwrap();
    let output = command(root)
        .env("APP_KEY", KEY)
        .args(["deploy:doctor", "--process-env"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Deployment has not been verified"));
    let conflict = command(root)
        .args([
            "deploy:doctor",
            "--process-env",
            "--env-file",
            "selected.env",
        ])
        .output()
        .unwrap();
    assert!(!conflict.status.success());
}

#[test]
fn malformed_oversized_and_ambiguous_inputs_fail_without_echoing_values_or_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for bad in [
        "APP_KEY=private-value\nAPP_KEY=duplicate\n",
        "APP_KEY=${PRIVATE_SECRET}\n",
        "APP_KEY=$(touch sentinel)\n",
        "APP_KEY=\"multiline\nvalue\"\n",
        "APP_KEY=unquoted value\n",
        "APP_KEY='unterminated\n",
        "BAD-NAME=value\n",
        "APP_KEY=escaped\\nvalue\n",
        "APP_KEY=value\0more\n",
        "APP_KEY='ok'trailing\n",
        "RULLST_ENV='production'#not-a-comment\n",
    ] {
        fs::write(root.join("selected.env"), bad).unwrap();
        let (output, value) = run(root, &["--env-file", "selected.env"]);
        assert!(!output.status.success());
        assert_eq!(status(&value, "input_env_file"), Some("FAIL"));
        for bytes in [&output.stdout, &output.stderr] {
            assert!(!String::from_utf8_lossy(bytes).contains("private-value"));
        }
    }
    for oversized in [
        format!("IGNORED={}\n", "x".repeat(8193)),
        (0..513).map(|i| format!("KEY_{i}=value\n")).collect(),
    ] {
        fs::write(root.join("selected.env"), oversized).unwrap();
        assert!(
            !run(root, &["--env-file", "selected.env"])
                .0
                .status
                .success()
        );
    }
    env(root, "RULLST_ENV=production");
    for bad in [
        "[app]\nenv = 'private-value'\nenv = 'duplicate'",
        "[app]\nenv = {private = 'value'}",
        "private-value = \"unterminated",
    ] {
        fs::write(root.join("Rullst.toml"), bad).unwrap();
        let (output, value) = run(root, &["--env-file", "selected.env"]);
        assert!(!output.status.success());
        assert_eq!(status(&value, "input_toml"), Some("FAIL"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private-value"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private-value"));
    }
    for body in [vec![b'#'; 65537], vec![255, 254]] {
        fs::write(root.join("Rullst.toml"), body).unwrap();
        assert_eq!(status(&run(root, &[]).1, "input_file"), Some("FAIL"));
    }
    let (output, value) = run(root, &["--config", "private-path-secret"]);
    assert_eq!(status(&value, "input_file"), Some("FAIL"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-path-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("private-path-secret"));
    assert!(!root.join("sentinel").exists());
}

#[test]
fn core_security_errors_and_application_policy_reviews_remain_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    env(root, "RULLST_ENV=production");
    for security in [
        "cors_allow_origins=['*']",
        "csrf_same_site='typo-private'",
        "csrf_signed_webhook_paths=['/billing/*']",
        "coep='invalid-private'",
        "csp=''",
        "cors_allow_origins=['https://private.invalid/path']",
    ] {
        fs::write(
            root.join("Rullst.toml"),
            format!("[security]\n{security}\n"),
        )
        .unwrap();
        let (output, value) = run(root, &["--env-file", "selected.env"]);
        assert!(!output.status.success());
        assert_eq!(status(&value, "security_configuration"), Some("FAIL"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("invalid-private"));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private.invalid"));
    }
    fs::write(root.join("Rullst.toml"), "[app]\nport=0\n[security]\ncoep='unsafe-none'\ncsrf_same_site='None'\ncors_allow_credentials=true\ncsrf_signed_webhook_paths=['/billing/webhook']\ncsp=\"default-src 'self'; script-src 'unsafe-inline'\"\nmispeled='private-value'\n").unwrap();
    let (output, value) = run(root, &["--env-file", "selected.env"]);
    assert!(!output.status.success());
    assert_eq!(status(&value, "configured_port"), Some("FAIL"));
    for code in [
        "custom_csp",
        "browser_policy_exceptions",
        "signed_webhook_exemptions",
        "unrecognized_configuration",
    ] {
        assert_eq!(status(&value, code), Some("REVIEW"));
    }
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private-value"));
}

#[test]
fn obvious_key_mistakes_and_selection_limits_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    for key in [
        "",
        "short",
        "REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY",
        "CHANGE_ME_TO_A_SECURE_RANDOM_KEY",
        "mock_012345678901234567890123456789",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "example_01234567890123456789012345",
        "your_01234567890123456789012345678",
        "fixture-with-control-01234567890\nAB",
    ] {
        let output = command(dir.path())
            .env("APP_KEY", key)
            .env("RULLST_ENV", "prod")
            .args(["deploy:doctor", "--json", "--process-env"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            status(&value, "application_key_obvious_errors"),
            Some("FAIL")
        );
    }
    let oversized = command(dir.path())
        .env("APP_KEY", "x".repeat(8193))
        .args(["deploy:doctor", "--json", "--process-env"])
        .output()
        .unwrap();
    assert!(!oversized.status.success());
    let value: Value = serde_json::from_slice(&oversized.stdout).unwrap();
    assert_eq!(status(&value, "input_process_env"), Some("FAIL"));
}

#[test]
fn generated_saas_configuration_is_inspected_without_building_or_modifying_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let output = command(dir.path())
        .args([
            "new",
            "doctor-app",
            "--default",
            "--blueprint",
            "saas",
            "--database",
            "sqlite",
            "--skip-initial-migration",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root = dir.path().join("doctor-app");
    let (development, value) = run(&root, &["--env-file", ".env"]);
    assert!(!development.status.success());
    assert_eq!(status(&value, "environment_target"), Some("FAIL"));
    assert_eq!(
        status(&value, "application_key_obvious_errors"),
        Some("PASS")
    );
    assert_eq!(status(&value, "signed_webhook_exemptions"), Some("REVIEW"));
    let content = fs::read_to_string(root.join(".env"))
        .unwrap()
        .replace("RULLST_ENV=development", "RULLST_ENV=production");
    fs::write(root.join("production.env"), &content).unwrap();
    let (production, value) = run(&root, &["--env-file", "production.env"]);
    assert!(
        production.status.success(),
        "{}",
        String::from_utf8_lossy(&production.stdout)
    );
    assert_eq!(value["deployment_verified"], false);
    assert_eq!(
        fs::read_to_string(root.join("production.env")).unwrap(),
        content
    );
    assert!(!root.join(".rullst_dev_key").exists());
    assert!(!root.join("target").exists());
}

#[cfg(unix)]
#[test]
fn links_special_files_and_non_unicode_environment_are_rejected() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("real.toml"), "").unwrap();
    symlink("real.toml", root.join("Rullst.toml")).unwrap();
    assert_eq!(status(&run(root, &[]).1, "input_file"), Some("FAIL"));
    fs::remove_file(root.join("Rullst.toml")).unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/config.toml"), "").unwrap();
    symlink("nested", root.join("link")).unwrap();
    assert!(
        !run(root, &["--config", "link/config.toml"])
            .0
            .status
            .success()
    );
    assert!(!run(root, &["--config", "nested"]).0.status.success());
    // rustix's mkfifoat is not exported on Apple platforms. The POSIX fixture
    // utility also exercises a real FIFO there, without unsafe test FFI.
    assert!(
        std::process::Command::new("mkfifo")
            .arg(root.join("Rullst.toml"))
            .status()
            .unwrap()
            .success()
    );
    assert!(!run(root, &[]).0.status.success());
    fs::remove_file(root.join("Rullst.toml")).unwrap();
    let output = command(root)
        .env("RULLST_ENV", std::ffi::OsString::from_vec(vec![255]))
        .args(["deploy:doctor", "--json", "--process-env"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status(&value, "input_process_env"), Some("FAIL"));
}
