//! Process-level behavior of the `dev` supervisor: shutdown on termination
//! signals and `--ts-sync` regeneration after rebuilds.
#![cfg(unix)]
#![allow(clippy::expect_used, clippy::panic)]

use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::Path,
    process::{Child, Command, Stdio},
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

/// A project whose fake Cargo "builds" `app.sh`, which records its PID and
/// keeps running like a server.
struct DevSession {
    project: tempfile::TempDir,
    tools: tempfile::TempDir,
    dev: Child,
}

impl DevSession {
    fn start(arguments: &[&str], routes: &str) -> Self {
        let project = tempfile::tempdir().expect("project directory");
        let tools = tempfile::tempdir().expect("tool directory");
        let root = project.path();
        fs::create_dir_all(root.join("src")).expect("source directory");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"dev-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13\"\n",
        )
        .expect("manifest");
        fs::write(root.join("src/main.rs"), routes).expect("entry point");
        executable(
            &root.join("app.sh"),
            "#!/bin/sh\necho $$ > app.pid\nexec sleep 60\n",
        );
        executable(
            &tools.path().join("cargo"),
            "#!/bin/sh\nif [ \"$1\" = build ]; then\n  printf '{\"reason\":\"compiler-artifact\",\"manifest_path\":\"%s/Cargo.toml\",\"target\":{\"kind\":[\"bin\"],\"name\":\"dev-fixture\"},\"profile\":{\"test\":false},\"executable\":\"%s/app.sh\"}\\n' \"$PWD\" \"$PWD\"\nfi\nexit 0\n",
        );
        let log = fs::File::create(tools.path().join("dev.log")).expect("log file");
        let dev = Command::new(env!("CARGO_BIN_EXE_rullst"))
            .args(arguments)
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
        Self {
            project,
            tools,
            dev,
        }
    }

    fn root(&self) -> &Path {
        self.project.path()
    }

    fn log(&self) -> String {
        fs::read_to_string(self.tools.path().join("dev.log")).unwrap_or_default()
    }

    /// PID of the running application.
    fn application(&mut self) -> String {
        let pid_file = self.root().join("app.pid");
        let started = wait_until(Duration::from_secs(30), || {
            fs::read_to_string(&pid_file).is_ok_and(|pid| !pid.trim().is_empty())
        });
        assert!(started, "the application never started:\n{}", self.log());
        fs::read_to_string(&pid_file)
            .expect("application PID")
            .trim()
            .to_string()
    }

    fn signal(&mut self, signal: &str) -> bool {
        Command::new("kill")
            .args([&format!("-{signal}"), &self.dev.id().to_string()])
            .status()
            .expect("signal dev");
        let dev = &mut self.dev;
        wait_until(Duration::from_secs(15), || {
            matches!(dev.try_wait(), Ok(Some(_)))
        })
    }
}

/// Kills an application that a still-running `dev` owns after a failed
/// assertion; once `dev` has exited, its PID may already belong to another
/// process.
fn kill_application(pid: &str) {
    let _ = Command::new("kill")
        .args(["-KILL", pid])
        .stderr(Stdio::null())
        .status();
}

impl Drop for DevSession {
    fn drop(&mut self) {
        if matches!(self.dev.try_wait(), Ok(None)) {
            if let Ok(pid) = fs::read_to_string(self.root().join("app.pid")) {
                kill_application(pid.trim());
            }
            let _ = self.dev.kill();
            let _ = self.dev.wait();
        }
    }
}

#[test]
fn termination_and_hangup_stop_the_application_instead_of_orphaning_it() {
    for signal in ["TERM", "HUP"] {
        let mut session = DevSession::start(&["dev"], "fn main() {}\n");
        let app = session.application();
        assert!(alive(&app), "{signal}: application not running");
        // `dev` stays plain apart from one hint about the live dashboard.
        let log = session.log();
        assert_eq!(
            log.lines()
                .filter(|line| line.contains("cargo rullst dash"))
                .count(),
            1,
            "{log}"
        );

        let exited = session.signal(signal);
        // The old CLI died on the default signal action and left the
        // application running in its own process group.
        let stopped = wait_until(Duration::from_secs(5), || !alive(&app));
        if !stopped {
            kill_application(&app);
        }
        assert!(exited, "{signal}: dev did not exit:\n{}", session.log());
        assert!(
            stopped,
            "{signal}: application survived:\n{}",
            session.log()
        );
        let snapshots = fs::read_dir(session.root())
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

#[test]
fn ts_sync_regenerates_the_sdk_after_each_successful_rebuild() {
    let mut session = DevSession::start(
        &["dev", "--ts-sync"],
        "fn routes() { routes! { get(\"/users\" => users::index) } }\n",
    );
    session.application();
    let sdk = session.root().join("rullst-client.ts");
    let read = || fs::read_to_string(&sdk).unwrap_or_default();
    assert!(read().contains("/users"), "{}", session.log());

    // The old flag generated the SDK once before startup and never again.
    fs::write(
        session.root().join("src/main.rs"),
        "fn routes() { routes! { get(\"/users\" => users::index), get(\"/teams\" => teams::index) } }\n",
    )
    .expect("edited routes");
    let synced = wait_until(Duration::from_secs(30), || read().contains("/teams"));
    assert!(
        synced,
        "SDK not regenerated:\n{}\n{}",
        read(),
        session.log()
    );
    assert!(session.log().contains("TypeScript SDK synchronized"));
    assert!(
        session.signal("TERM"),
        "dev did not exit:\n{}",
        session.log()
    );
}
