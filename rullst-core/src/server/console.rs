//! Console output that never panics when stdout or stderr has gone away.
//!
//! `println!`/`eprintln!` panic on any write error other than `EBADF`, such as
//! `EPIPE` after the reader of a stdout pipe exits or `EIO` after the
//! controlling terminal hangs up. On the request path that panic would reset
//! the connection after the handler's side effects already ran, so these
//! helpers drop the diagnostic line instead.

use super::ServerError;
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

/// Whether a completed request is access-logged: never the development HMR
/// channel and polls, and a `/health` or `/ready` request below status 400
/// only when `log_health_probes` is set.
pub(super) fn should_log(path: &str, status: u16, log_health_probes: bool) -> bool {
    is_logged(path) && (log_health_probes || status >= 400 || !HEALTH_PROBE_PATHS.contains(&path))
}

/// Prints one completed request that [`should_log`] admits. The development
/// telemetry recorder is fed by its own outermost layer
/// (`dev_telemetry::record_responses`), not from here.
pub(crate) fn log_request(
    method: &str,
    path: &str,
    status: u16,
    elapsed: std::time::Duration,
    log_health_probes: bool,
) {
    if should_log(path, status, log_health_probes) {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        stdout_line(format_args!(
            "[HTTP] {method} {path} -> {status} ({elapsed_ms:.2} ms)"
        ));
    }
}

/// Whether `path` is access-logged (and counted by development telemetry).
pub(super) fn is_logged(path: &str) -> bool {
    !path.starts_with("/_rullst_hmr") && !UNLOGGED_DEVELOPMENT_PATHS.contains(&path)
}

/// Access-log middleware for the static server; its state is
/// [`log_health_probes`].
pub(crate) async fn access_log_middleware(
    State(log_health_probes): State<bool>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    log_request(
        method.as_str(),
        &path,
        response.status().as_u16(),
        start.elapsed(),
        log_health_probes,
    );
    response
}

#[cfg(test)]
#[test]
fn development_polls_are_not_access_logged() {
    assert!(!is_logged("/_rullst/dev-generation"));
    assert!(!is_logged("/_rullst/dev-reload.js"));
    assert!(!is_logged("/_rullst/dev-telemetry"));
    assert!(!is_logged("/_rullst_hmr"));
    assert!(is_logged("/orders"));
    assert!(is_logged("/_rullst/dev-generation/extra"));
}

#[cfg(test)]
#[test]
fn quiet_health_probes_still_log_failures_and_other_paths() {
    for path in ["/health", "/ready"] {
        assert!(!should_log(path, 200, false), "{path}");
        assert!(!should_log(path, 399, false), "{path}");
        assert!(should_log(path, 400, false), "{path}");
        assert!(should_log(path, 503, false), "{path}");
        assert!(should_log(path, 200, true), "{path}");
    }
    assert!(should_log("/health/details", 200, false));
    assert!(should_log("/orders", 200, false));
    assert!(!should_log("/_rullst/dev-generation", 500, true));
}

#[cfg(test)]
#[test]
#[allow(clippy::unwrap_used)]
fn health_probe_logging_follows_the_environment_unless_configured() {
    let none = HashMap::new();
    let dotenv =
        |value: &str| HashMap::from([("RULLST_LOG_HEALTH_PROBES".to_string(), value.to_string())]);
    assert_eq!(LOG_HEALTH_PROBES_VARIABLE, "RULLST_LOG_HEALTH_PROBES");
    for (environment, logged) in [
        (Environment::Development, true),
        (Environment::Test, true),
        (Environment::Staging, false),
        (Environment::Production, false),
    ] {
        assert_eq!(log_health_probes(environment, None, &none).unwrap(), logged);
    }
    let production = Environment::Production;
    assert!(log_health_probes(production, Some("true".into()), &none).unwrap());
    assert!(log_health_probes(production, None, &dotenv("on")).unwrap());
    assert!(!log_health_probes(production, Some("0".into()), &dotenv("1")).unwrap());
    assert!(!log_health_probes(Environment::Development, None, &dotenv("false")).unwrap());
    match log_health_probes(production, Some("sometimes".into()), &none) {
        Err(ServerError::Configuration(message)) => {
            assert_eq!(message, "RULLST_LOG_HEALTH_PROBES must be true or false");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read};
    use std::process::{Command, Stdio};

    const CHILD_ENV: &str = "RULLST_CLOSED_STDOUT_CHILD";
    const CHILD_TEST: &str = "server::console::tests::closed_stdout_child";
    const SURVIVED: &str = "rullst-access-log-survived-closed-stdout";

    /// Runs only inside the child spawned by the parent test below.
    #[tokio::test]
    async fn closed_stdout_child() {
        if std::env::var_os(CHILD_ENV).is_none() {
            return;
        }
        // The parent closes the read end of stdout before sending this line.
        let mut go = String::new();
        std::io::stdin().read_line(&mut go).unwrap();

        use tower::ServiceExt;
        let router = axum::Router::new()
            .route("/orders", axum::routing::post(|| async { "created" }))
            .layer(axum::middleware::from_fn_with_state(
                true,
                access_log_middleware,
            ));
        let response = router
            .oneshot(
                axum::http::Request::post("/orders")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        stderr_line(format_args!("{SURVIVED}"));
    }

    #[test]
    fn access_log_survives_a_closed_stdout() {
        if std::env::var_os(CHILD_ENV).is_some() {
            return;
        }
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=2"])
            .env(CHILD_ENV, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        // Wait for the harness header so its own output is already written,
        // then close the pipe the access log will write to.
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        while !line.starts_with("running 1 test") {
            line.clear();
            assert!(
                stdout.read_line(&mut line).unwrap() > 0,
                "child exited early"
            );
        }
        drop(stdout);
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"go\n").unwrap();
        drop(stdin);

        let mut child_stderr = child.stderr.take().unwrap();
        let reader = std::thread::spawn(move || {
            let mut stderr = String::new();
            let _ = child_stderr.read_to_string(&mut stderr);
            stderr
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("the closed-stdout child exceeded its deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let stderr = reader.join().unwrap();
        assert!(
            !stderr.contains("failed printing to stdout"),
            "the access log panicked: {stderr}"
        );
        assert!(
            stderr.contains(SURVIVED),
            "the request did not complete: {stderr}"
        );
    }

    #[test]
    fn write_errors_are_ignored() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        write_line(&mut Broken, format_args!("[HTTP] GET / -> 200"));
    }

    const LOG_CHILD_ENV: &str = "RULLST_ACCESS_LOG_CHILD";
    const LOG_CHILD_TEST: &str = "server::console::tests::quiet_probe_access_log_child";

    /// Runs only inside the child spawned by the parent test below, whose
    /// stdout the access log writes to.
    #[tokio::test]
    async fn quiet_probe_access_log_child() {
        if std::env::var_os(LOG_CHILD_ENV).is_none() {
            return;
        }
        use tower::ServiceExt;
        let router = axum::Router::new()
            .route("/orders", axum::routing::get(|| async { "orders" }))
            .route("/health", axum::routing::get(|| async { "ok" }))
            .route(
                "/ready",
                axum::routing::get(|| async { axum::http::StatusCode::SERVICE_UNAVAILABLE }),
            )
            .layer(axum::middleware::from_fn_with_state(
                false,
                access_log_middleware,
            ));
        for (path, status) in [("/orders", 200), ("/health", 200), ("/ready", 503)] {
            let response = router
                .clone()
                .oneshot(
                    axum::http::Request::get(path)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), status, "{path}");
        }
    }

    #[test]
    fn quiet_probe_access_log_prints_requests_and_failing_probes() {
        if std::env::var_os(LOG_CHILD_ENV).is_some() {
            return;
        }
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", LOG_CHILD_TEST, "--nocapture", "--test-threads=1"])
            .env(LOG_CHILD_ENV, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut child_stdout = child.stdout.take().unwrap();
        let reader = std::thread::spawn(move || {
            let mut stdout = String::new();
            let _ = child_stdout.read_to_string(&mut stdout);
            stdout
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("the access-log child exceeded its deadline");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let stdout = reader.join().unwrap();
        assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
        assert!(stdout.contains("[HTTP] GET /orders -> 200 ("), "{stdout}");
        assert!(stdout.contains("[HTTP] GET /ready -> 503 ("), "{stdout}");
        assert!(!stdout.contains("/health"), "{stdout}");
    }
}
