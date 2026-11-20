//! Remote projects: the standalone Studio on a developer's machine.
//!
//! Requires the non-default `remote` feature, which adds the HTTP client,
//! token encryption and the SQLite history (and the `rullst-studio` binary).
//!
//! [`run_remote_studio`] (the `rullst-studio` binary) serves Studio on
//! `127.0.0.1` without an application process. It keeps a list of projects
//! (name, base URL, monitor token) and renders each project's read-only
//! monitoring document, which an application serves with
//! `rullst_core::Server::with_monitor`: `rullst.monitor.v2` (with request
//! traces and the security radar) when the application serves it, else
//! `rullst.monitor.v1`. Studio only connects outwards; the
//! data browser, `.env` viewer, AI and API playgrounds and migration pages are
//! not part of this mode.
//!
//! The project list is an owner-only file in the user's configuration
//! directory. Each token is encrypted with AES-256-GCM under a key kept in a
//! separate owner-only file in the user's data directory, bound to its project
//! name and base URL. Requests go only to `https://` origins, or `http://` to a
//! loopback address (an SSH tunnel), follow no redirects and are bounded in
//! size and time. Threat model: `TM-STUDIO-3`. Added in v13.
//!
//! Other accounts on the same machine can connect to `127.0.0.1` too, so
//! since 13.0.0-rc.1 each start prints an address with a random key and
//! Studio serves only a browser that opened it; anyone who reads that address
//! can use Studio until it restarts.

mod alerts;
mod client;
mod crypto;
mod document;
mod document_v2;
mod history;
mod launch;
mod monitoring;
mod notify;
mod pages;
mod paths;
mod poller;
mod preferences;
mod projects;
mod proxy;
mod router;
mod secure_file;
mod settings;
mod stored;
mod webhook;

#[cfg(test)]
mod alerts_tests;
#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod document_tests;
#[cfg(test)]
mod history_tests;
#[cfg(test)]
mod launch_tests;
#[cfg(test)]
mod notify_tests;
#[cfg(test)]
mod poller_tests;
#[cfg(test)]
mod preferences_tests;
#[cfg(test)]
mod proxy_env_tests;
#[cfg(test)]
mod proxy_tests;
#[cfg(test)]
mod router_tests;
#[cfg(test)]
mod server_test;
#[cfg(test)]
mod settings_tests;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod stored_bounds_tests;
#[cfg(test)]
mod stored_tests;
#[cfg(test)]
mod webhook_tests;

pub use alerts::AlertRules;
pub use client::{FetchError, FetchLimits};
pub use history::Retention;
pub use paths::StudioPaths;
pub use poller::{DEFAULT_POLL_INTERVAL, MAX_POLL_INTERVAL, MIN_POLL_INTERVAL, clamp_interval};
pub use projects::{MAX_PROJECTS, ProjectError};

use std::fmt;
use std::sync::Arc;

/// Port of the standalone Studio unless another is chosen. It differs from
/// the debug Studio of an application (`5555`), so both can run.
pub const DEFAULT_REMOTE_PORT: u16 = 5556;

/// Settings of the standalone Studio.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RemoteStudioOptions {
    /// Loopback port to listen on.
    pub port: u16,
    /// Where the project list, key and history live.
    pub paths: StudioPaths,
    /// Bounds of each read of a project.
    pub limits: FetchLimits,
    /// How often watched projects are read (clamped to 15 s–1 h).
    pub poll_interval: std::time::Duration,
    /// How much history is kept.
    pub retention: Retention,
}

impl RemoteStudioOptions {
    /// Options with the default port and limits.
    pub fn new(paths: StudioPaths) -> Self {
        Self {
            port: DEFAULT_REMOTE_PORT,
            paths,
            limits: FetchLimits::default(),
            poll_interval: DEFAULT_POLL_INTERVAL,
            retention: Retention::default(),
        }
    }

    /// Reads watched projects every `interval` (clamped to 15 s–1 h).
    pub fn with_poll_interval(mut self, interval: std::time::Duration) -> Self {
        self.poll_interval = clamp_interval(interval);
        self
    }

    /// Listens on `port` (always on `127.0.0.1`).
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

/// Why the standalone Studio could not start.
#[derive(Debug)]
#[non_exhaustive]
pub enum RemoteStudioError {
    /// The operating system gave no randomness for the form token.
    Randomness,
    /// The HTTP client (TLS stack) could not be created.
    Client,
    /// Listening or serving failed.
    Io(std::io::Error),
}

impl fmt::Display for RemoteStudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Randomness => formatter.write_str("operating-system randomness is unavailable"),
            Self::Client => formatter.write_str("the HTTP client could not be created"),
            Self::Io(error) => write!(formatter, "Studio could not listen: {error}"),
        }
    }
}

impl std::error::Error for RemoteStudioError {}

fn random_hex(bytes: usize) -> Result<String, RemoteStudioError> {
    let mut buffer = vec![0_u8; bytes];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut buffer)
        .map_err(|_| RemoteStudioError::Randomness)?;
    Ok(buffer.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) async fn state(
    options: &RemoteStudioOptions,
) -> Result<router::RemoteState, RemoteStudioError> {
    let history = history::History::open(&options.paths.history, options.retention)
        .await
        .ok()
        .map(Arc::new);
    let monitoring = monitoring::Monitoring::default();
    if let Some(history) = &history {
        monitoring.set_open_alerts(history.unacknowledged().await.unwrap_or(0));
    }
    let store = projects::ProjectStore::new(options.paths.clone());
    let proxy = store
        .settings()
        .map(|settings| settings.proxy)
        .unwrap_or_default();
    let plan = proxy::ProxyPlan::from_settings(&proxy);
    Ok(router::RemoteState {
        launch: launch::LaunchKey::new(random_hex(32)?, options.port),
        store,
        client: std::sync::RwLock::new(
            client::MonitorClient::with_proxy(options.limits, &plan)
                .map_err(|_| RemoteStudioError::Client)?,
        ),
        limits: options.limits,
        form_token: random_hex(32)?,
        monitoring,
        history,
        webhooks: std::sync::RwLock::new(webhook::WebhookClient::new(&plan)),
        poll_interval: clamp_interval(options.poll_interval),
        notifier: notify::Notifier::new(random_hex(8)?),
    })
}

/// Serves the remote Studio on `127.0.0.1:<port>` until the process stops.
///
/// The port is taken first, so a port in use fails before anything else
/// starts, with an error naming the address. Once it listens, the Studio
/// address and the project list's path are printed to standard output.
pub async fn run_remote_studio(options: RemoteStudioOptions) -> Result<(), RemoteStudioError> {
    let listener = listen(options.port).await?;
    let state = Arc::new(state(&options).await?);
    if let Some(history) = state.history.clone() {
        tokio::spawn(poller::run(state.clone(), history, state.poll_interval));
    }
    let address = state.launch.address();
    let app = router::router(state);
    announce(&address, &options.paths.projects);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(RemoteStudioError::Io)
}

/// Takes `127.0.0.1:<port>`; the error of a port in use names it.
async fn listen(port: u16) -> Result<tokio::net::TcpListener, RemoteStudioError> {
    tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| {
            RemoteStudioError::Io(std::io::Error::new(
                error.kind(),
                format!("127.0.0.1:{port}: {error}"),
            ))
        })
}

/// Prints where the Studio listens, with this start's key. A closed
/// standard output is ignored rather than stopping a Studio that is already
/// serving.
fn announce(address: &str, projects: &std::path::Path) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "Rullst Studio (remote projects) on {address}");
    let _ = writeln!(stdout, "Project list: {}", projects.display());
}
