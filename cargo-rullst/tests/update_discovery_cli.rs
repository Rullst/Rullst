//! No-network process contracts for the new executable-only update entry point.
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    let cache = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(args)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("XDG_CACHE_HOME", cache.path())
        .output()
        .unwrap()
}

#[test]
fn help_lists_update_without_removing_the_legacy_upgrade_command() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("update"));
    assert!(text.contains("upgrade"));
    assert!(run(&["upgrade", "--help"]).status.success());
    assert!(
        run(&["rullst", "update", "check", "--help"])
            .status
            .success()
    );
}

#[test]
fn offline_discovery_fails_actionably_instead_of_using_an_untrusted_cache() {
    let output = run(&["update", "check", "--offline", "--json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("offline mode forbids registry discovery")
    );
}

#[test]
fn major_and_prerelease_opt_ins_cannot_select_an_unpinned_target() {
    for flag in ["--allow-major", "--prerelease"] {
        let output = run(&["update", "check", flag]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--to"));
    }
}

#[test]
fn environment_offline_is_not_overridden_by_refresh_or_no_cache() {
    for flag in ["--refresh", "--no-cache"] {
        let output = run(&["update", "check", flag, "--json"]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("offline mode forbids registry discovery")
        );
    }
}

#[cfg(unix)]
mod private_cache {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture(body: &str, age: u64) -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let directory = temp.path().join("rullst-update-v1");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - age;
        let path = directory.join("catalog-v1");
        fs::write(
            &path,
            format!("rullst-update-cache-v1\n{timestamp}\n{body}"),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        temp
    }

    fn cached_run(temp: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
            .args(["update", "check", "--offline", "--json"])
            .args(args)
            .env("CARGO_NET_OFFLINE", "true")
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("XDG_CACHE_HOME", temp.path())
            .output()
            .unwrap()
    }

    #[test]
    fn offline_report_revalidates_metadata_without_granting_authority_or_writing() {
        let body = r#"{"versions":[{"crate":"cargo-rullst","num":"12.1.0","yanked":false,"rust_version":"1.96"}]}"#;
        let temp = fixture(body, 20);
        let path = temp.path().join("rullst-update-v1/catalog-v1");
        let before = fs::read(&path).unwrap();
        let output = cached_run(&temp, &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["metadata_source"], "private-cache");
        assert!(report["metadata_age_seconds"].as_u64().unwrap() >= 20);
        assert_eq!(report["target"]["version"], "12.1.0");
        assert_eq!(report["target"]["rust_version"], "1.96.0");
        assert!(
            report["authority"]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v == false)
        );
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(
            fs::read_dir(temp.path().join("rullst-update-v1"))
                .unwrap()
                .count(),
            1
        );
        // Metadata is selected again for this request, not a cached decision.
        let rejected = cached_run(&temp, &["--to", "13.0.0", "--allow-major"]);
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("exact release is unavailable"));
    }

    #[test]
    fn expired_malformed_and_yanked_cache_cannot_substitute_for_an_exact_target() {
        for (body, age) in [
            (r#"{"versions":[]}"#, 6 * 3600),
            ("not-json", 0),
            (
                r#"{"versions":[{"crate":"cargo-rullst","num":"12.1.0","yanked":true}]}"#,
                0,
            ),
        ] {
            let temp = fixture(body, age);
            let output = cached_run(&temp, &["--to", "12.1.0"]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("offline mode forbids registry discovery")
            );
        }
    }
}
