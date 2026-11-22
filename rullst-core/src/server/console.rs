//! Console output that never panics when stdout or stderr has gone away.
//!
//! `println!`/`eprintln!` panic on any write error other than `EBADF`, such as
//! `EPIPE` after the reader of a stdout pipe exits or `EIO` after the
//! controlling terminal hangs up. On the request path that panic would reset
//! the connection after the handler's side effects already ran, so these
//! helpers drop the diagnostic line instead.

use super::ServerError;
use super::access_log::Refusal;
pub(crate) use super::access_log::{AccessLog, Recording};
use crate::config::Environment;
use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use std::collections::HashMap;
use std::fmt::Arguments;
use std::io::Write;

/// Writes one line to stdout and ignores write failures.
pub(crate) fn stdout_line(line: Arguments<'_>) {
    write_line(&mut std::io::stdout().lock(), line);
}

/// Writes one line to stderr and ignores write failures.
pub(crate) fn stderr_line(line: Arguments<'_>) {
    write_line(&mut std::io::stderr().lock(), line);
}

fn write_line(output: &mut impl Write, line: Arguments<'_>) {
    let _ = output
        .write_fmt(line)
        .and_then(|()| output.write_all(b"\n"));
}

/// Development polling paths that would otherwise log twice a second per
/// open page (and once a second for `cargo rullst dash`).
const UNLOGGED_DEVELOPMENT_PATHS: [&str; 3] = [
    "/_rullst/dev-generation",
    "/_rullst/dev-reload.js",
    super::dev_telemetry::PATH,
];

/// Variable that turns the access log of successful `/health` and `/ready`
/// requests on or off, from the process environment and then `.env`.
pub(super) const LOG_HEALTH_PROBES_VARIABLE: &str = "RULLST_LOG_HEALTH_PROBES";

/// The probes of `rullst::health::health_router`.
const HEALTH_PROBE_PATHS: [&str; 2] = ["/health", "/ready"];

/// Whether successful health probes are access-logged:
/// `RULLST_LOG_HEALTH_PROBES` (the process value, else the `.env` value) when
/// set; otherwise everywhere except staging and production, where probes
/// every few seconds would fill the log.
pub(super) fn log_health_probes(
    environment: Environment,
    process: Option<String>,
    dotenv: &HashMap<String, String>,
) -> Result<bool, ServerError> {
    match process.or_else(|| dotenv.get(LOG_HEALTH_PROBES_VARIABLE).cloned()) {
        Some(value) => super::builder::parse_switch(LOG_HEALTH_PROBES_VARIABLE, &value),
        None => Ok(!environment.requires_secure_defaults()),
    }
}

/// Variable that chooses which requests the access log writes
/// ([`Recording`]), from the process environment and then `.env`.
pub(super) const ACCESS_LOG_VARIABLE: &str = "RULLST_ACCESS_LOG";

/// The access log's [`Recording`]: `RULLST_ACCESS_LOG` (the process value,
/// else the `.env` value) when set, otherwise [`Recording::All`]. It accepts
/// the on/off values of `RULLST_LOG_HEALTH_PROBES`, `errors`, and `1/N` for
/// one in `N` requests below status 400 (`N` from 1 to 1000000).
pub(super) fn access_log_recording(
    process: Option<String>,
    dotenv: &HashMap<String, String>,
) -> Result<Recording, ServerError> {
    let Some(value) = process.or_else(|| dotenv.get(ACCESS_LOG_VARIABLE).cloned()) else {
        return Ok(Recording::All);
    };
    let value = value.trim().to_ascii_lowercase();
    if value == "errors" {
        return Ok(Recording::Errors);
    }
    if let Some(interval) = value.strip_prefix("1/") {
        return match interval.parse::<std::num::NonZeroU32>() {
            Ok(interval) if interval.get() == 1 => Ok(Recording::All),
            Ok(interval) if interval.get() <= super::access_log::MAX_SAMPLE_INTERVAL => {
                Ok(Recording::Sample(interval))
            }
            _ => Err(recording_error()),
        };
    }
    match super::builder::parse_switch(ACCESS_LOG_VARIABLE, &value) {
        Ok(true) => Ok(Recording::All),
        Ok(false) => Ok(Recording::Off),
        Err(_) => Err(recording_error()),
    }
}

fn recording_error() -> ServerError {
    ServerError::Configuration(format!(
        "{ACCESS_LOG_VARIABLE} must be on, off, errors or 1/N with N from 1 to {}",
        super::access_log::MAX_SAMPLE_INTERVAL
    ))
}

/// Whether a completed request is access-logged: never the development HMR
/// channel and polls, and a `/health` or `/ready` request below status 400
/// only when `log_health_probes` is set.
pub(super) fn should_log(path: &str, status: u16, log_health_probes: bool) -> bool {
    is_logged(path) && (log_health_probes || status >= 400 || !HEALTH_PROBE_PATHS.contains(&path))
}

/// Queues one completed request to `path` that [`should_log`] and
/// `recording` admit, showing `shown_path` ([`AccessLog::shown_path`]) and
/// the framework layer that answered it, if one did ([`Refusal`]), for the
/// access log's writer thread ([`super::access_log_writer`]). The
/// development telemetry recorder is fed by its own outermost layer
/// (`dev_telemetry::record_responses`), not from here.
pub(crate) fn log_request(
    method: &str,
    (path, shown_path): (&str, &str),
    (status, refusal): (u16, Option<Refusal>),
    elapsed: std::time::Duration,
    (log_health_probes, recording): (bool, Recording),
) {
    if should_log(path, status, log_health_probes) && recording.records(status) {
        super::access_log_writer::write_line(access_line(
            method,
            shown_path,
            (status, refusal),
            Milliseconds::of(elapsed),
        ));
    }
}

/// One access-log line: `[HTTP] POST /login -> 403 (0.31 ms)`, followed by
/// ` refused by <layer>` when a framework layer answered the request.
pub(super) fn access_line(
    method: &str,
    shown_path: &str,
    (status, refusal): (u16, Option<Refusal>),
    elapsed: Milliseconds,
) -> String {
    let mut line = format!("[HTTP] {method} {shown_path} -> {status} ({elapsed} ms)");
    if let Some(refusal) = refusal {
        line.push_str(" refused by ");
        line.push_str(refusal.layer());
    }
    line
}

/// A duration in milliseconds with two decimals, rounded half up, as an
/// access-log line shows it (`1500.25`). Formatted from integers: a float
/// with a precision costs several times as much, on every request.
pub(super) struct Milliseconds(u64);

impl Milliseconds {
    pub(super) fn of(elapsed: std::time::Duration) -> Self {
        let micros = u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX);
        Self(micros.saturating_add(5) / 10)
    }
}

impl std::fmt::Display for Milliseconds {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}

/// Whether `path` is access-logged (and counted by development telemetry).
pub(super) fn is_logged(path: &str) -> bool {
    !path.starts_with("/_rullst_hmr") && !UNLOGGED_DEVELOPMENT_PATHS.contains(&path)
}

/// Access-log middleware of the static and hot-reload servers. Installed
/// with `Router::layer`, it runs after routing, so the matched route is
/// known.
pub(crate) async fn access_log_middleware(
    State(log): State<AccessLog>,
    request: Request,
    next: Next,
) -> Response {
    with_access_log(log, request, |request| next.run(request)).await
}

/// [`access_log_middleware`] and [`crate::htmx::vary_on_htmx_request`] as
/// one layer, as the static server installs them.
pub(crate) async fn access_log_and_vary_middleware(
    State(log): State<AccessLog>,
    request: Request,
    next: Next,
) -> Response {
    with_access_log(log, request, |request| async move {
        let mut response = next.run(request).await;
        crate::htmx::vary_html_on_htmx_request(response.headers_mut());
        response
    })
    .await
}

/// [`access_log_middleware`] around `next`.
async fn with_access_log<F, Fut>(log: AccessLog, request: Request, next: F) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: std::future::Future<Output = Response>,
{
    if !is_logged(request.uri().path()) {
        return next(request).await;
    }
    let method = request.method().clone();
    // A clone shares the request's bytes.
    let uri = request.uri().clone();
    let shown_path = log.shown_path(uri.path(), request.extensions());
    let start = std::time::Instant::now();
    let mut response = next(request).await;
    // A layer of the application's own router may have refused it; this line
    // names it, and the refusal log outside writes no second one.
    let refusal = response.extensions_mut().remove::<Refusal>();
    log_request(
        method.as_str(),
        (uri.path(), shown_path.as_str()),
        (response.status().as_u16(), refusal),
        start.elapsed(),
        (log.log_health_probes, log.recording),
    );
    response
}

/// The access log of the requests that a framework layer refused before
/// they reached [`access_log_middleware`]: the static and hot-reload servers
/// install it outside the security baseline (CSRF, the WAF), Traffic
/// Shield, the rate limiter and lifecycle admission, whose responses never
/// pass the access log inside them. It writes a line, with the same rules
/// and the refusing layer ([`Refusal`]), only for a response such a layer
/// marked; every other response was logged inside, or is a static file or a
/// CORS preflight, which the access log does not write.
pub(crate) async fn refusal_log_middleware(
    State(log): State<AccessLog>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    // Clones share the request's bytes and the route's name.
    let uri = request.uri().clone();
    let route = request.extensions().get::<MatchedPath>().cloned();
    let start = std::time::Instant::now();
    let mut response = next.run(request).await;
    if let Some(refusal) = response.extensions_mut().remove::<Refusal>() {
        let shown_path = log.shown_path_for(uri.path(), route.as_ref());
        log_request(
            method.as_str(),
            (uri.path(), shown_path.as_str()),
            (response.status().as_u16(), Some(refusal)),
            start.elapsed(),
            (log.log_health_probes, log.recording),
        );
    }
    response
}

#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
