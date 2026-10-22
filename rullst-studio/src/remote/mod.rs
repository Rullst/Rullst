//! Remote projects: the standalone Studio on a developer's machine.
//!
//! [`run_remote_studio`] (the `rullst-studio` binary) serves Studio on
//! `127.0.0.1` without an application process. It keeps a list of projects
//! (name, base URL, monitor token) and renders each project's read-only
//! `rullst.monitor.v1` document, which an application serves with
//! `rullst_core::Server::with_monitor`. Studio only connects outwards; the
//! data browser, `.env` viewer, AI and API playgrounds and migration pages are
//! not part of this mode.
//!
//! The project list is an owner-only file in the user's configuration
//! directory. Each token is encrypted with AES-256-GCM under a key kept in a
//! separate owner-only file in the user's data directory, bound to its project
//! name and base URL. Requests go only to `https://` origins, or `http://` to a
//! loopback address (an SSH tunnel), follow no redirects and are bounded in
//! size and time. Threat model: `TM-STUDIO-3`. Added in v13.

mod client;
mod crypto;
mod document;
mod monitoring;
mod pages;
mod paths;
mod projects;
mod router;
mod secure_file;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod router_tests;
#[cfg(test)]
mod server_test;
#[cfg(test)]
mod store_tests;

pub use client::{FetchError, FetchLimits};
pub use paths::StudioPaths;
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
}

impl RemoteStudioOptions {
    /// Options with the default port and limits.
    pub fn new(paths: StudioPaths) -> Self {
        Self {
            port: DEFAULT_REMOTE_PORT,
            paths,
            limits: FetchLimits::default(),
        }
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

pub(crate) fn state(
    options: &RemoteStudioOptions,
) -> Result<router::RemoteState, RemoteStudioError> {
    Ok(router::RemoteState {
        store: projects::ProjectStore::new(options.paths.clone()),
        client: client::MonitorClient::new(options.limits)
            .map_err(|_| RemoteStudioError::Client)?,
        form_token: random_hex(32)?,
        monitoring: monitoring::Monitoring::default(),
    })
}

/// Serves the remote Studio on `127.0.0.1:<port>` until the process stops.
pub async fn run_remote_studio(options: RemoteStudioOptions) -> Result<(), RemoteStudioError> {
    let state = Arc::new(state(&options)?);
    let app = router::router(state);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", options.port))
        .await
        .map_err(RemoteStudioError::Io)?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .map_err(RemoteStudioError::Io)
}
