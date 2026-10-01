//! `dev` stops its supervised application when the CLI is terminated.
#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn executable(path: &Path, body: &str) {
    fs::write(path, body).expect("executable fixture");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("fixture permissions");
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn wait_until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    done()
}

fn unused_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("ephemeral port")
        .port()
}

#[test]
fn termination_and_hangup_stop_the_application_instead_of_orphaning_it() {
    for signal in ["TERM", "HUP"] {
        let project = tempfile::tempdir().expect("project directory");
        let tools = tempfile::tempdir().expect("tool directory");
        let root = project.path();
        fs::create_dir_all(root.join("src")).expect("source directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"signal-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
        )
        .expect("manifest");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("entry point");
        // The "application" records its PID and keeps running, like a server.
        executable(
            &root.join("app.sh"),
            "#!/bin/sh\necho $$ > app.pid\nexec sleep 60\n",
        );
        // Cargo reports app.sh as the package binary without compiling.
        executable(
            &tools.path().join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = build ]; then\n  printf '{\"reason\":\"compiler-artifact\",\"manifest_path\":\"%s/Cargo.toml\",\"target\":{\"kind\":[\"bin\"],\"name\":\"signal-fixture\"},\"profile\":{\"test\":false},\"executable\":\"%s/app.sh\"}\\n' \"$PWD\" \"$PWD\"\nfi\nexit 0\n",
        );
        let log = fs::File::create(tools.path().join("dev.log")).expect("log file");
        let mut dev = Command::new(env!("CARGO_BIN_EXE_rullst"))
            .arg("dev")
            .current_dir(root)
            .env("PATH", format!("{}:/usr/bin:/bin", tools.path().display()))
            .env("PORT", unused_port().to_string())
            .env("RULLST_DISABLE_UPDATE_CHECK", "1")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .expect("start dev");

        let pid_file = root.join("app.pid");
        let started = wait_until(Duration::from_secs(30), || {
            fs::read_to_string(&pid_file).is_ok_and(|pid| !pid.trim().is_empty())
        });
        let log = || fs::read_to_string(tools.path().join("dev.log")).unwrap_or_default();
        if !started {
            let _ = dev.kill();
            panic!("{signal}: the application never started:\n{}", log());
        }
        let app = fs::read_to_string(&pid_file).expect("application PID");
        let app = app.trim();
        assert!(alive(app), "{signal}: application not running");

        Command::new("kill")
            .args([&format!("-{signal}"), &dev.id().to_string()])
            .status()
            .expect("signal dev");
        let exited = wait_until(Duration::from_secs(15), || {
            matches!(dev.try_wait(), Ok(Some(_)))
        });
        // The old CLI died on the default signal action and left the
        // application running in its own process group.
        let stopped = wait_until(Duration::from_secs(5), || !alive(app));
        if !stopped {
            let _ = Command::new("kill").args(["-KILL", app]).status();
        }
        if !exited {
            let _ = dev.kill();
        }
        assert!(exited, "{signal}: dev did not exit:\n{}", log());
        assert!(stopped, "{signal}: application survived:\n{}", log());
        let snapshots = fs::read_dir(root)
            .expect("project listing")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("rullst-dev-")
            })
            .count();
        assert_eq!(snapshots, 0, "{signal}: executable snapshot leaked");
    }
}
