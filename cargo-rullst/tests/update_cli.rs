//! Process contracts for the update entry points: the version, the removed
//! self-update commands, the guided project flow's terminal requirement and
//! the silent update notice outside a terminal.
use std::process::Command;

fn run(args: &[&str], cache: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(args)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
        .env("XDG_CACHE_HOME", cache)
        .env("LOCALAPPDATA", cache)
        .output()
        .unwrap()
}

#[test]
fn both_entry_points_report_the_actual_cli_source_version() {
    for (executable, arguments) in [
        (env!("CARGO_BIN_EXE_cargo-rullst"), vec!["--version"]),
        (
            env!("CARGO_BIN_EXE_cargo-rullst"),
            vec!["rullst", "--version"],
        ),
        (env!("CARGO_BIN_EXE_rullst"), vec!["--version"]),
    ] {
        let output = Command::new(executable).args(arguments).output().unwrap();
        assert!(output.status.success());
        let stdout = std::str::from_utf8(&output.stdout).unwrap();
        assert_eq!(
            stdout.trim(),
            format!("rullst {}", env!("CARGO_PKG_VERSION"))
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn the_cli_never_replaces_itself_and_keeps_project_updates() {
    let cache = tempfile::tempdir().unwrap();
    let output = run(&["--help"], cache.path());
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("update"));
    assert!(text.contains("upgrade"));
    assert!(run(&["upgrade", "--help"], cache.path()).status.success());
    for kept in [
        &["rullst", "update", "project", "prepare", "--help"][..],
        &["update", "guided", "--help"],
    ] {
        assert!(run(kept, cache.path()).status.success(), "{kept:?}");
    }
    for removed in [
        &["update", "check"][..],
        &["update", "verify", "--help"],
        &["update", "stage", "--help"],
        &["update", "install", "review", "--help"],
    ] {
        let output = run(removed, cache.path());
        assert_eq!(output.status.code(), Some(2), "{removed:?}");
    }
}

#[test]
fn guided_flow_refuses_piped_approval_before_any_filesystem_or_network_work() {
    let base = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["update", "guided", "--to", env!("CARGO_PKG_VERSION")])
        .arg("--project")
        .arg(base.path().join("not-created"))
        .env("XDG_CACHE_HOME", base.path().join("missing-cache"))
        .env("LOCALAPPDATA", base.path().join("missing-cache"))
        .env("RULLST_DISABLE_UPDATE_CHECK", "true")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("interactive terminal"));
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 0);
}

#[test]
fn the_update_notice_stays_silent_and_offline_outside_a_terminal() {
    let cache = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(["info"])
        .env_remove("RULLST_DISABLE_UPDATE_CHECK")
        .env_remove("CARGO_NET_OFFLINE")
        .env_remove("CI")
        .env("XDG_CACHE_HOME", cache.path())
        .env("LOCALAPPDATA", cache.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains("A newer cargo-rullst"), "{text}");
    // Neither a catalog request nor a notice stamp happened.
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
}
