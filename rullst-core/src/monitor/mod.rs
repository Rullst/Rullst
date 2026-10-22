//! Opt-in, read-only production monitoring endpoint (`GET /_rullst/monitor/v1`).
//!
//! [`Server::with_monitor`](crate::Server::with_monitor) mounts a JSON
//! document, versioned as [`MONITOR_SCHEMA`], that a monitoring client (the
//! Studio "remote projects" mode, a script, an uptime tool) reads over HTTPS:
//! the application's name and version, uptime, the process radar, request
//! counts, rates and latency percentiles per **route pattern**, status
//! classes, the queues the application registers, and possible N+1 query
//! fingerprints. It never carries raw paths, query strings, headers,
//! cookies, client addresses, user identifiers, bodies, SQL text or values,
//! error messages, environment or configuration values.
//!
//! The endpoint is off by default. It is mounted only when the application
//! calls `Server::with_monitor` **and** a credential is configured: a
//! monitor token (from [`MONITOR_TOKENS_ENV`] or [`Monitor::with_token`]) or
//! an application authorizer ([`Monitor::with_authorizer`]). Without one the
//! path answers `404` and request handling is unchanged.
//!
//! ```rust,no_run
//! use rullst_core::monitor::Monitor;
//! use rullst_core::{Server, routes, routing::get};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! Server::new(routes![get("/" => || async { "OK" })])
//!     // Mounted once RULLST_MONITOR_TOKENS holds at least one token.
//!     .with_monitor(Monitor::new().with_application(
//!         env!("CARGO_PKG_NAME"),
//!         env!("CARGO_PKG_VERSION"),
//!     ))
//!     .run(3000)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! Requests carrying an `Origin` header or a cross-site `Sec-Fetch-Site`
//! are refused, in staging and production a request needs TLS evidence
//! ([`MonitorVerifiedTls`] or HTTPS reported by a trusted proxy), and each
//! client has a request budget and a failed-credential lockout. See the
//! production monitoring guide and threat model `TM-STUDIO-2`. Added in v13.

mod auth;
mod collector;
mod endpoint;
mod payload;
mod record;
mod token;
mod window;

#[cfg(test)]
mod collector_tests;
#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod endpoint_tests;
#[cfg(test)]
mod payload_tests;
#[cfg(test)]
mod record_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod token_tests;
#[cfg(test)]
mod window_tests;

pub use auth::{MonitorAccess, MonitorAuthorizer, MonitorAuthorizerError};

use crate::queue::Queue;
use auth::ErasedAuthorizer;
use std::fmt;
use std::sync::Arc;

/// Path of the monitoring document. It sits in Core's reserved
/// `/_rullst/*` namespace, so it cannot collide with application routes, and
/// carries the schema's major version, so a later version can be served next
/// to it.
pub const MONITOR_PATH: &str = "/_rullst/monitor/v1";
/// Value of the document's `schema` field.
pub const MONITOR_SCHEMA: &str = "rullst.monitor.v1";
/// Environment variable (process environment first, then `.env`) holding
/// the accepted monitor tokens, separated by commas. Several tokens let you
/// rotate without downtime.
pub const MONITOR_TOKENS_ENV: &str = "RULLST_MONITOR_TOKENS";
/// Most monitor tokens accepted at once.
pub const MAX_MONITOR_TOKENS: usize = 8;
/// Most queues a monitor reports.
pub const MAX_MONITOR_QUEUES: usize = 16;

/// Invalid monitor configuration. Server start-up fails with
/// [`crate::ServerError::Configuration`] carrying this message; no message
/// contains a token.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MonitorError {
    /// A token is shorter than 32 bytes of entropy: at least 43 base64 or
    /// base64url characters, or 64 hexadecimal characters.
    #[error(
        "monitor token {position} is too short: use at least 32 random bytes (for example `openssl rand -base64 32`)"
    )]
    WeakToken {
        /// Position of the token in the configured list, from 1.
        position: usize,
    },
    /// A token contains characters outside base64, base64url and hexadecimal,
    /// or more than 512 characters.
    #[error(
        "monitor token {position} must contain only base64, base64url or hexadecimal characters, at most 512 of them"
    )]
    InvalidToken {
        /// Position of the token in the configured list, from 1.
        position: usize,
    },
    /// A token repeats too few characters, or looks like a documentation
    /// placeholder.
    #[error("monitor token {position} is not random enough or is a placeholder")]
    PlaceholderToken {
        /// Position of the token in the configured list, from 1.
        position: usize,
    },
    /// More than [`MAX_MONITOR_TOKENS`] tokens are configured.
    #[error("at most {maximum} monitor tokens may be configured")]
    TooManyTokens {
        /// The limit.
        maximum: usize,
    },
    /// The application name or version is empty, longer than 128 bytes or
    /// contains control characters.
    #[error(
        "the monitor's application name and version must be 1 to 128 bytes without control characters"
    )]
    InvalidApplication,
    /// A queue name is not 1 to 64 ASCII letters, digits, `.`, `_` or `-`.
    #[error("monitor queue names must be 1 to 64 ASCII letters, digits, '.', '_' or '-'")]
    InvalidQueueName,
    /// Two queues were registered under the same name.
    #[error("a monitor queue name is registered twice")]
    DuplicateQueue,
    /// More than [`MAX_MONITOR_QUEUES`] queues are registered.
    #[error("at most {maximum} monitor queues may be registered")]
    TooManyQueues {
        /// The limit.
        maximum: usize,
    },
}

/// Capability inserted into requests only by an application that terminates
/// TLS itself (for example with a rustls listener), so the monitor accepts
/// them in staging and production.
///
/// Behind a reverse proxy, configure
/// [`TrustedProxyConfig::trust_forwarded_proto`](crate::security::TrustedProxyConfig::trust_forwarded_proto)
/// instead. Never insert this marker from a forwarded header.
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct MonitorVerifiedTls {
    _private: (),
}

impl MonitorVerifiedTls {
    /// Asserts that the application's own listener authenticated the TLS
    /// transport of this request.
    pub const fn from_trusted_tls_termination() -> Self {
        Self { _private: () }
    }
}

/// Configuration of the monitoring endpoint; pass it to
/// [`Server::with_monitor`](crate::Server::with_monitor).
///
/// Token values are kept out of `Debug` output and are validated when the
/// server starts.
#[derive(Clone, Default)]
pub struct Monitor {
    tokens: Vec<String>,
    authorizer: Option<Arc<dyn ErasedAuthorizer>>,
    application: Option<(String, String)>,
    queues: Vec<(String, Arc<Queue>)>,
    loopback_plaintext: bool,
}

impl fmt::Debug for Monitor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Monitor")
            .field("tokens", &self.tokens.len())
            .field("authorizer", &self.authorizer.is_some())
            .field("application", &self.application)
            .field(
                "queues",
                &self
                    .queues
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("loopback_plaintext", &self.loopback_plaintext)
            .finish()
    }
}

impl Monitor {
    /// A monitor whose tokens come from [`MONITOR_TOKENS_ENV`] (read when
    /// the server starts) and from [`Self::with_token`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Accepts `token` as `Authorization: Bearer <token>`, in addition to
    /// the environment's tokens. It must carry at least 32 random bytes (43
    /// base64/base64url or 64 hexadecimal characters); the server refuses to
    /// start otherwise. Load it from a secret store, never from source code.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.tokens.push(token.into());
        self
    }

    /// Lets the application's own sign-in decide about requests without a
    /// valid monitor token; see [`MonitorAuthorizer`].
    pub fn with_authorizer(mut self, authorizer: impl MonitorAuthorizer) -> Self {
        self.authorizer = Some(Arc::new(authorizer));
        self
    }

    /// Reports the application's name and version, usually
    /// `env!("CARGO_PKG_NAME")` and `env!("CARGO_PKG_VERSION")`.
    pub fn with_application(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.application = Some((name.into(), version.into()));
        self
    }

    /// Reports `queue` under `name`: its pending count (read from the
    /// store, with a 250 ms limit) and what this process's workers did with
    /// it since start. Accepts a [`Queue`] or an `Arc<Queue>` the
    /// application shares with its workers.
    pub fn with_queue(mut self, name: impl Into<String>, queue: impl Into<Arc<Queue>>) -> Self {
        self.queues.push((name.into(), queue.into()));
        self
    }

    /// Also admits plain HTTP in staging and production from a direct
    /// loopback peer that names a loopback `Host` and sends no forwarding
    /// headers: an SSH tunnel or a sidecar on the same host. Off by default.
    /// Requests that a same-host reverse proxy forwards carry forwarding
    /// headers and still need TLS evidence.
    pub fn allow_loopback_plaintext(mut self, allowed: bool) -> Self {
        self.loopback_plaintext = allowed;
        self
    }

    /// Adds the comma-separated tokens of [`MONITOR_TOKENS_ENV`]; empty
    /// entries are ignored.
    pub(crate) fn add_environment_tokens(&mut self, value: Option<&str>) {
        let tokens = value
            .into_iter()
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string);
        self.tokens.extend(tokens);
    }
}

pub(crate) use endpoint::Installed;
