//! The access log in child processes, whose stdout the tests read.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::*;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};

/// Waits until the access log's queued lines are on stdout.
fn flush_access_log() -> bool {
    crate::server::access_log_writer::flush(std::time::Duration::from_secs(10))
}

const CHILD_ENV: &str = "RULLST_CLOSED_STDOUT_CHILD";
const CHILD_TEST: &str = "server::console::tests::unix::closed_stdout_child";
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
            AccessLog {
                log_health_probes: true,
                route_paths: false,
                recording: Recording::All,
            },
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
    // The writer thread meets the closed pipe, and goes on.
    assert!(flush_access_log());
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

const ELAPSED_CHILD_ENV: &str = "RULLST_ACCESS_LOG_ELAPSED_CHILD";
const ELAPSED_CHILD_TEST: &str = "server::console::tests::unix::elapsed_access_log_child";

/// Runs only inside the child spawned by the parent test below.
#[test]
fn elapsed_access_log_child() {
    if std::env::var_os(ELAPSED_CHILD_ENV).is_none() {
        return;
    }
    let elapsed = std::time::Duration::from_micros(1_500_250);
    let all = (true, Recording::All);
    log_request("POST", ("/courses", "/courses"), (201, None), elapsed, all);
    let errors = (true, Recording::Errors);
    log_request("GET", ("/hidden", "/hidden"), (200, None), elapsed, errors);
    log_request(
        "GET",
        ("/missing", "/missing"),
        (404, None),
        elapsed,
        errors,
    );
    assert!(flush_access_log());
}

/// Runs the test `name` of this binary in a child process with `env` set
/// and returns its standard output, where the access log writes.
fn child_stdout(name: &str, env: &str) -> String {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(env, "1")
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
    reader.join().unwrap()
}

#[test]
fn access_log_lines_report_the_elapsed_milliseconds() {
    if std::env::var_os(ELAPSED_CHILD_ENV).is_some() {
        return;
    }
    let stdout = child_stdout(ELAPSED_CHILD_TEST, ELAPSED_CHILD_ENV);
    assert!(
        stdout.contains("[HTTP] POST /courses -> 201 (1500.25 ms)\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("[HTTP] GET /missing -> 404 (1500.25 ms)\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("/hidden"), "{stdout}");
}

const LOG_CHILD_ENV: &str = "RULLST_ACCESS_LOG_CHILD";
const LOG_CHILD_TEST: &str = "server::console::tests::unix::quiet_probe_access_log_child";

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
            AccessLog {
                log_health_probes: false,
                route_paths: false,
                recording: Recording::All,
            },
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
    assert!(flush_access_log());
}

#[test]
fn quiet_probe_access_log_prints_requests_and_failing_probes() {
    if std::env::var_os(LOG_CHILD_ENV).is_some() {
        return;
    }
    let stdout = child_stdout(LOG_CHILD_TEST, LOG_CHILD_ENV);
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
    assert!(stdout.contains("[HTTP] GET /orders -> 200 ("), "{stdout}");
    assert!(stdout.contains("[HTTP] GET /ready -> 503 ("), "{stdout}");
    assert!(!stdout.contains("/health"), "{stdout}");
}

const ROUTE_CHILD_ENV: &str = "RULLST_ACCESS_LOG_ROUTE_CHILD";
const ROUTE_CHILD_TEST: &str = "server::console::tests::unix::route_access_log_child";

/// Runs only inside the child spawned by the parent test below.
#[tokio::test]
async fn route_access_log_child() {
    if std::env::var_os(ROUTE_CHILD_ENV).is_none() {
        return;
    }
    use tower::ServiceExt;
    let router = axum::Router::new()
        // rullst-access: public — test fixture; the token is the link's capability.
        .route("/booking/{token}", axum::routing::get(|| async { "ok" }))
        .layer(axum::middleware::from_fn_with_state(
            AccessLog {
                log_health_probes: true,
                route_paths: true,
                recording: Recording::All,
            },
            access_log_middleware,
        ));
    for (uri, status) in [("/booking/s3cr3t?ref=x", 200), ("/missing/k3y", 404)] {
        let response = router
            .clone()
            .oneshot(
                axum::http::Request::get(uri)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status, "{uri}");
    }
    assert!(flush_access_log());
}

#[test]
fn route_access_log_writes_templates_instead_of_request_paths() {
    if std::env::var_os(ROUTE_CHILD_ENV).is_some() {
        return;
    }
    let stdout = child_stdout(ROUTE_CHILD_TEST, ROUTE_CHILD_ENV);
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
    assert!(
        stdout.contains("[HTTP] GET /booking/{token} -> 200 ("),
        "{stdout}"
    );
    assert!(
        stdout.contains("[HTTP] GET /missing/k3y -> 404 ("),
        "{stdout}"
    );
    assert!(
        !stdout.contains("s3cr3t") && !stdout.contains("ref="),
        "{stdout}"
    );
}
