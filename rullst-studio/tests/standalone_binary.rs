//! The `rullst-studio` binary's command line, run as a real process.
#![cfg(feature = "remote")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

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

/// An empty home for one run, so the user's own project list is untouched.
fn private_home(name: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("rullst-studio-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create private home");
    home
}

/// `rullst-studio --port PORT` with its configuration and data under `home`.
fn studio(port: u16, home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rullst-studio"));
    command
        .args(["--port", &port.to_string()])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("APPDATA", home.join("config"))
        .env("LOCALAPPDATA", home.join("data"));
    command
}

#[test]
fn a_port_in_use_fails_naming_it_before_printing_an_address() {
    let taken = TcpListener::bind("127.0.0.1:0").expect("take a loopback port");
    let port = taken.local_addr().expect("taken address").port();
    let home = private_home("taken");
    let output = studio(port, &home).output().expect("run rullst-studio");
    drop(taken);
    let history_created = home.join("data/rullst/studio/history.sqlite").exists();
    let _ = std::fs::remove_dir_all(&home);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with(&format!(
            "rullst-studio: Studio could not listen: 127.0.0.1:{port}: "
        )),
        "{stderr}"
    );
    assert!(!history_created, "nothing starts before the port is taken");
}

#[test]
fn the_address_is_printed_once_the_port_listens() {
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|probe| probe.local_addr())
        .expect("find a free loopback port")
        .port();
    let home = private_home("listening");
    let mut child = studio(port, &home)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start rullst-studio");
    let stdout = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        let first = lines.next().and_then(Result::ok);
        let second = lines.next().and_then(Result::ok);
        let _ = sender.send((first, second));
    });
    let printed = receiver.recv_timeout(Duration::from_secs(60));
    // The address is printed only once the port accepts connections.
    let accepted = TcpStream::connect(("127.0.0.1", port)).is_ok();
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&home);

    let (first, second) = printed.expect("rullst-studio printed its address");
    assert_eq!(
        first.as_deref(),
        Some(format!("Rullst Studio (remote projects) on http://127.0.0.1:{port}/studio").as_str())
    );
    let projects = second.expect("project list line");
    assert!(projects.starts_with("Project list: "), "{projects}");
    assert!(projects.ends_with("projects.json"), "{projects}");
    assert!(accepted);
}
