//! Tool discovery, toolchain pinning, bounded output and exit handling.
use super::*;
use std::fs;

fn read_all(input: &[u8], limit: u64) -> Result<Vec<u8>, ProjectError> {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(read(input, limit))
}

#[test]
fn tool_output_may_fill_but_not_exceed_its_bound() {
    assert_eq!(read_all(b"abcd", 4).unwrap(), b"abcd");
    assert!(read_all(b"abcde", 4).is_err());
}

#[test]
fn captured_tools_succeed_or_fail_by_exit_status() {
    let directory = tempfile::tempdir().unwrap();
    let version = capture("git", &["--version"], directory.path(), 4096).unwrap();
    assert!(version.starts_with(b"git version "), "{version:?}");
    // Outside a repository, git exits with status 128.
    assert!(capture("git", &["rev-parse", "HEAD"], directory.path(), 4096).is_err());
    // Only a diff may exit with status 1; elsewhere it is a failure.
    assert!(
        capture(
            "git",
            &["config", "--file", "absent.cfg", "--get", "rullst.absent"],
            directory.path(),
            4096
        )
        .is_err()
    );
}

#[test]
fn verification_commands_report_their_status_and_output() {
    let directory = tempfile::tempdir().unwrap();
    let (success, output, _) = execute(
        "cargo",
        &["--version".into()],
        directory.path(),
        &directory.path().join("target"),
        Duration::from_secs(60),
        true,
    )
    .unwrap();
    assert!(success);
    assert!(output.starts_with(b"cargo "), "{output:?}");
    let (success, _, diagnostics) = execute(
        "cargo",
        &["rullst-no-such-subcommand".into()],
        directory.path(),
        &directory.path().join("target"),
        Duration::from_secs(60),
        true,
    )
    .unwrap();
    assert!(!success);
    assert!(!diagnostics.is_empty());
}

#[test]
fn large_review_diffs_up_to_8_mib_are_returned() {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().canonicalize().unwrap();
    let before = base.join("project-one/before");
    let candidate = base.join("project-two/candidate");
    fs::create_dir_all(&before).unwrap();
    fs::create_dir_all(&candidate).unwrap();
    let line = "x".repeat(99);
    fs::write(before.join("data.txt"), format!("{line}\n").repeat(12_000)).unwrap();
    fs::write(
        candidate.join("data.txt"),
        format!("{line}y\n").repeat(12_000),
    )
    .unwrap();
    let patch = diff(&before, &candidate).unwrap();
    assert!(patch.len() > 2 * 1024 * 1024, "{}", patch.len());
}

#[test]
fn git_needs_no_pinned_toolchain_and_cargo_keeps_the_inherited_one() {
    assert_eq!(pinned_toolchain("git").unwrap(), None);
    if let Some(inherited) = std::env::var_os("RUSTUP_TOOLCHAIN").filter(|value| !value.is_empty())
    {
        assert_eq!(pinned_toolchain("cargo").unwrap(), Some(inherited));
    }
}

/// Runs `pinned_toolchain` or `tool_path` in a child test process.
#[cfg(unix)]
#[test]
fn process_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_PROCESS_REPORT") else {
        return;
    };
    let text = if std::env::var_os("RULLST_PROCESS_TOOL").is_some() {
        format!("{:?}", tool_path("git").map_err(|error| error.to_string()))
    } else {
        format!(
            "{:?}",
            pinned_toolchain("cargo").map_err(|error| error.to_string())
        )
    };
    fs::write(report, text).unwrap();
}

#[cfg(unix)]
fn in_child(path: &std::ffi::OsStr, toolchain: Option<&str>, tool: bool) -> String {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("report");
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "update::project::process::process_tests::process_child_fixture",
            "--nocapture",
        ])
        .env("PATH", path)
        .env("RULLST_PROCESS_REPORT", &report)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RULLST_PROCESS_TOOL")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());
    if let Some(toolchain) = toolchain {
        command.env("RUSTUP_TOOLCHAIN", toolchain);
    }
    if tool {
        command.env("RULLST_PROCESS_TOOL", "1");
    }
    assert!(command.status().unwrap().success());
    fs::read_to_string(report).unwrap()
}

#[cfg(unix)]
fn fake_tool(directory: &Path, name: &str, script: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(unix)]
#[test]
fn the_default_toolchain_comes_from_a_neutral_rustup_query() {
    let bin = tempfile::tempdir().unwrap();
    let path = bin.path().as_os_str();
    let rustup = |output: &str| {
        fake_tool(
            bin.path(),
            "rustup",
            &format!("printf '%s' '{output}'"),
            0o700,
        );
        in_child(path, None, false)
    };
    assert_eq!(
        rustup("stable-x86_64-unknown-linux-gnu (default)\n"),
        r#"Ok(Some("stable-x86_64-unknown-linux-gnu"))"#
    );
    let longest = "a".repeat(128);
    assert_eq!(rustup(&longest), format!("Ok(Some(\"{longest}\"))"));
    let refused = r#"Err("project preparation rejected: rustup reports no usable default toolchain; set RUSTUP_TOOLCHAIN to the trusted toolchain for project commands")"#;
    for output in ["", "nightly;touch", &"a".repeat(129)] {
        assert_eq!(rustup(output), refused, "{output}");
    }
    fake_tool(bin.path(), "rustup", "echo stable; exit 1", 0o700);
    assert_eq!(in_child(path, None, false), refused);
    // An explicit toolchain wins; an empty one does not count.
    assert_eq!(
        in_child(path, Some("1.99.1"), false),
        r#"Ok(Some("1.99.1"))"#
    );
    fake_tool(bin.path(), "rustup", "echo pinned", 0o700);
    assert_eq!(in_child(path, Some(""), false), r#"Ok(Some("pinned"))"#);
}

#[cfg(unix)]
#[test]
fn tools_resolve_to_the_first_executable_absolute_path_entry() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    fake_tool(first.path(), "git", "exit 0", 0o600);
    fake_tool(second.path(), "git", "exit 0", 0o700);
    let path = std::env::join_paths([first.path(), second.path()]).unwrap();
    assert_eq!(
        in_child(&path, None, true),
        format!("Ok({:?})", second.path().join("git"))
    );
}
