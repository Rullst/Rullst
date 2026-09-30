//! Console output that never panics when stdout or stderr has gone away.
//!
//! `println!`/`eprintln!` panic on any write error other than `EBADF`, such as
//! `EPIPE` after the reader of a stdout pipe exits or `EIO` after the
//! controlling terminal hangs up. On the request path that panic would reset
//! the connection after the handler's side effects already ran, so these
//! helpers drop the diagnostic line instead.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
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

/// Records one completed request, except for the development HMR channel.
pub(crate) fn log_request(method: &str, path: &str, status: u16, elapsed_ms: f64) {
    if !path.starts_with("/_rullst_hmr") {
        stdout_line(format_args!(
            "[HTTP] {method} {path} -> {status} ({elapsed_ms:.2} ms)"
        ));
    }
}

/// Access-log middleware for the static server.
pub(crate) async fn access_log_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();
    let response = next.run(request).await;
    log_request(
        method.as_str(),
        &path,
        response.status().as_u16(),
        start.elapsed().as_secs_f64() * 1000.0,
    );
    response
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
            .layer(axum::middleware::from_fn(access_log_middleware));
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
}
