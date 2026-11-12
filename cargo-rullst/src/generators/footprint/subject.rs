//! The app being measured: an app already running on `--url`, found by its
//! port, or the project's release binary started on a free loopback port.
use super::failure::Failure;
use super::report::Target;
use super::{process, target};
use crate::ui::error_report::ProjectRequired;
use std::error::Error;
use std::path::{Path, PathBuf};

/// The app being measured: its URL, PID and executable, plus the process
/// started by footprint (stopped on drop), or why it could not be measured.
pub(super) struct Subject {
    pub base: Option<reqwest::Url>,
    pub target: Target,
    pub executable: Option<PathBuf>,
    pub executable_method: &'static str,
    pub owned: Option<target::OwnedApp>,
    pub failure: Option<Failure>,
}

const RELEASE_BINARY: &str = "file size of the release binary built by cargo";

impl Subject {
    /// A started app (`release_build`) that footprint could not measure.
    fn unstarted(
        url: Option<&reqwest::Url>,
        binary: Option<PathBuf>,
        environment: &'static str,
        failure: Failure,
    ) -> Self {
        Self {
            base: None,
            target: Target {
                mode: "release_build",
                url: url.map(ToString::to_string),
                pid: None,
                process_lookup: failure.unmeasured().to_string(),
                executable: binary.as_deref().and_then(file_name),
                environment: Some(started_environment(environment)),
            },
            executable: binary,
            executable_method: RELEASE_BINARY,
            owned: None,
            failure: Some(failure),
        }
    }
}

pub(super) fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// The variables footprint set for the app it started.
pub(super) fn started_environment(environment: &str) -> &'static str {
    if environment == "staging" {
        "RULLST_ENV=staging HOST=127.0.0.1"
    } else {
        "RULLST_ENV=production HOST=127.0.0.1"
    }
}

/// The app already running on `base`, found by the port it listens on.
pub(super) async fn existing(
    client: &reqwest::Client,
    base: &reqwest::Url,
    path: &str,
) -> Result<Subject, Box<dyn Error>> {
    let url = target::request_url(base, path);
    let probe = client
        .get(url.clone())
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    if probe.is_err() {
        let failure = Failure::reach(url.as_str());
        return Ok(Subject {
            base: None,
            target: Target {
                mode: "existing_url",
                url: Some(url.to_string()),
                pid: None,
                process_lookup: failure.unmeasured().to_string(),
                executable: None,
                environment: None,
            },
            executable: None,
            executable_method: process::METHODS.executable,
            owned: None,
            failure: Some(failure),
        });
    }
    let port = base.port_or_known_default().unwrap_or(80);
    let (pid, lookup) = match process::listener(port) {
        target::Listener::Found(pid) => (
            Some(pid),
            format!("listening on port {port} ({})", process::METHODS.lookup),
        ),
        target::Listener::Unknown(reason) => (None, reason.to_string()),
    };
    let executable = pid.and_then(process::executable);
    Ok(Subject {
        base: Some(base.clone()),
        target: Target {
            mode: "existing_url",
            url: Some(url.to_string()),
            pid,
            process_lookup: lookup,
            executable: executable.as_ref().and_then(|found| found.name.clone()),
            environment: None,
        },
        executable: executable.map(|found| found.path),
        executable_method: process::METHODS.executable,
        owned: None,
        failure: None,
    })
}

/// Builds the project's release binary and starts it on a free port.
pub(super) async fn spawned(
    client: &reqwest::Client,
    path: &str,
    environment: &'static str,
) -> Result<Subject, Box<dyn Error>> {
    if crate::generators::platform_name::package_name(Path::new("Cargo.toml")).is_none() {
        return Err(ProjectRequired.into());
    }
    eprintln!(
        "Building the release binary (cargo build --release; an existing build is reused)..."
    );
    let binary = match crate::generators::dev::compile_release_in(Path::new(".")).await {
        Ok(binary) => binary,
        Err(error) => {
            let failure = Failure::build(&error.to_string());
            return Ok(Subject::unstarted(None, None, environment, failure));
        }
    };
    let port = target::free_port()?;
    let base = target::validate_url(&format!("http://127.0.0.1:{port}"))?;
    let url = target::request_url(&base, path);
    eprintln!("Starting the app with RULLST_ENV={environment} on {base} ...");
    let mut app = match target::OwnedApp::spawn(&binary, port, environment) {
        Ok(app) => app,
        Err(error) => {
            let failure = Failure {
                stage: "start",
                title: "App could not be started",
                message: format!("Starting {} failed: {error}.", binary.display()),
                fix: "Check that the release binary runs: `cargo run --release`.".into(),
            };
            return Ok(Subject::unstarted(
                Some(&url),
                Some(binary),
                environment,
                failure,
            ));
        }
    };
    if let Err(error) = app.wait_ready(client, &url).await {
        let failure = Failure::start(format!("{error}."), &app.stderr_tail(), environment);
        app.stop();
        return Ok(Subject::unstarted(
            Some(&url),
            Some(binary),
            environment,
            failure,
        ));
    }
    let name = file_name(&binary);
    Ok(Subject {
        base: Some(base),
        target: Target {
            mode: "release_build",
            url: Some(url.to_string()),
            pid: Some(app.pid()),
            process_lookup: "started by footprint".to_string(),
            executable: name,
            environment: Some(started_environment(environment)),
        },
        executable: Some(binary),
        executable_method: RELEASE_BINARY,
        owned: Some(app),
        failure: None,
    })
}
