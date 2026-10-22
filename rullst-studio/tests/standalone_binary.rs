//! The `rullst-studio` binary's command line, run as a real process.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst-studio"))
        .args(arguments)
        .output()
        .expect("run rullst-studio")
}

#[test]
fn version_and_help_print_and_exit_successfully() {
    let version = run(&["--version"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version.stdout),
        format!("rullst-studio {}\n", env!("CARGO_PKG_VERSION"))
    );

    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&help.stdout).starts_with("Usage: rullst-studio [--port PORT]")
    );
    assert!(help.stderr.is_empty());
}

#[test]
fn invalid_arguments_exit_with_usage_errors() {
    let invalid = run(&["--port", "x"]);
    assert_eq!(invalid.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(
        stderr.starts_with("rullst-studio: invalid port `x`"),
        "{stderr}"
    );
    assert!(invalid.stdout.is_empty());
}
