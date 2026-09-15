//! No-network process contracts for the new executable-only update entry point.
use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-rullst"))
        .args(args)
        .env("CARGO_NET_OFFLINE", "true")
        .env("RULLST_DISABLE_UPDATE_CHECK", "1")
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
