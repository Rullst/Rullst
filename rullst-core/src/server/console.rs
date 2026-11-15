//! Console output that never panics when stdout or stderr has gone away.
//!
//! `println!`/`eprintln!` panic on any write error other than `EBADF`, such as
//! `EPIPE` after the reader of a stdout pipe exits or `EIO` after the
//! controlling terminal hangs up. On the request path that panic would reset
//! the connection after the handler's side effects already ran, so these
//! helpers drop the diagnostic line instead.

use super::ServerError;
pub(crate) use super::access_log::{AccessLog, Recording};
use crate::config::Environment;
use axum::extract::{Request, State};
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
/// `recording` admit, showing `shown_path` ([`AccessLog::shown_path`]), for
/// the access log's writer thread ([`super::access_log_writer`]). The
/// development telemetry recorder is fed by its own outermost layer
/// (`dev_telemetry::record_responses`), not from here.
pub(crate) fn log_request(
    method: &str,
    (path, shown_path): (&str, &str),
    status: u16,
    elapsed: std::time::Duration,
    (log_health_probes, recording): (bool, Recording),
) {
    if should_log(path, status, log_health_probes) && recording.records(status) {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        super::access_log_writer::write_line(format!(
            "[HTTP] {method} {shown_path} -> {status} ({elapsed_ms:.2} ms)"
        ));
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
    let response = next(request).await;
    log_request(
        method.as_str(),
        (uri.path(), shown_path.as_str()),
        response.status().as_u16(),
        start.elapsed(),
        (log.log_health_probes, log.recording),
    );
    response
}

#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
