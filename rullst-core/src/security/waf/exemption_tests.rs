//! Exact-path body exemptions and the development report-only WAF.

#![allow(clippy::field_reassign_with_default)]

use super::*;
use crate::config::{Environment, SecurityConfig};
use axum::{
    Router,
    body::{Bytes, to_bytes},
    http::Request,
    routing::post,
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const EXEMPT: &str = "/lessons/answers";

fn security() -> SecurityConfig {
    let mut security = SecurityConfig::default();
    security.waf_body_exempt_paths = vec![EXEMPT.to_owned()];
    security
}

/// Echoes the request body behind the baseline of `environment`.
fn baseline_app(environment: Environment) -> Router {
    let echo = post(|body: Bytes| async move { body });
    let app = Router::new()
        .route(EXEMPT, echo.clone())
        .route("/lessons/answers/", echo.clone())
        .route("/items", echo);
    crate::security::apply_security_baseline(app, security(), environment).unwrap()
}

/// Echoes the request body behind `waf_middleware` alone.
fn waf_app() -> Router {
    let echo = post(|body: Bytes| async move { body });
    Router::new()
        .route(EXEMPT, echo.clone())
        .route("/lessons/answers/", echo.clone())
        .route("/items", echo)
        .layer(axum::middleware::from_fn(waf_middleware))
        .layer(axum::Extension(security()))
}

fn text_post(path: &str, body: impl Into<Body>) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body.into())
        .unwrap()
}

/// A lesson answer that the body signatures refuse.
const CODE_SAMPLE: &str = "cd ../../src && cat main.rs | grep 'select' -- see <script>";

async fn status_of(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn only_the_body_of_an_exact_exempt_path_skips_inspection() {
    let app = waf_app();
    let status = |request| status_of(&app, request);
    assert_eq!(status(text_post(EXEMPT, CODE_SAMPLE)).await, StatusCode::OK);
    // Not inspected at all: neither its encoding nor its size is checked.
    let encoded = Request::post(EXEMPT)
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_ENCODING, "gzip")
        .header(header::CONTENT_LENGTH, "2000000")
        .body(Body::from("x"))
        .unwrap();
    assert_eq!(status(encoded).await, StatusCode::OK);

    for (case, request) in [
        text_post("/items", CODE_SAMPLE),
        text_post("/lessons/answers/", CODE_SAMPLE),
        text_post("/lessons/answers?draft=%3Cscript%3E", "fine"),
        Request::post(EXEMPT)
            .header(header::COOKIE, "q=../../etc/passwd")
            .body(Body::from("fine"))
            .unwrap(),
        Request::post(EXEMPT)
            .header(header::REFERER, "https://x.example/?q=<script>")
            .body(Body::from("fine"))
            .unwrap(),
        Request::post(EXEMPT)
            .header(header::USER_AGENT, "GPTBot/1.0")
            .body(Body::from("fine"))
            .unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(status(request).await, StatusCode::FORBIDDEN, "case {case}");
    }
}

#[tokio::test]
async fn production_baseline_honours_the_exemption_and_keeps_the_rest() {
    let app = baseline_app(Environment::Production);
    let response = app
        .clone()
        .oneshot(text_post(EXEMPT, CODE_SAMPLE))
        .await
        .unwrap();
    // The WAF passed the body; CSRF still refuses the cookie-less write.
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(&body[..], b"CSRF token cookie missing");

    let refused = app.oneshot(text_post("/items", CODE_SAMPLE)).await.unwrap();
    let body = to_bytes(refused.into_body(), 1024).await.unwrap();
    assert_eq!(
        &body[..],
        b"Access Denied: Malicious pattern detected by Rullst Shield WAF."
    );
}

/// The response to `request` and the formatted WAF `tracing` lines written
/// while it ran.
async fn logged(app: Router, request: Request<Body>) -> (StatusCode, Bytes, String) {
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    drop(guard);
    let logs = String::from_utf8_lossy(&capture.0.lock().unwrap())
        .lines()
        .filter(|line| line.contains("rullst::security::waf"))
        .map(|line| format!("{line}\n"))
        .collect();
    (status, body, logs)
}

#[tokio::test]
async fn development_logs_what_production_would_refuse_without_refusing_it() {
    let app = baseline_app(Environment::Development);

    let (status, body, logs) = logged(app.clone(), text_post("/items", CODE_SAMPLE)).await;
    assert_eq!(
        (status, &body[..]),
        (StatusCode::OK, CODE_SAMPLE.as_bytes())
    );
    assert_eq!(logs.lines().count(), 1, "{logs}");
    assert!(logs.contains("WARN"), "{logs}");
    assert!(logs.contains("rule=\"body-signature\""), "{logs}");
    assert!(logs.contains("path=\"/items\""), "{logs}");
    assert!(
        !logs.contains("main.rs") && !logs.contains("script"),
        "{logs}"
    );

    let request = text_post("/items?q=%3Cscript%3Ealert(1)", "fine");
    let (status, body, logs) = logged(app.clone(), request).await;
    assert_eq!((status, &body[..]), (StatusCode::OK, &b"fine"[..]));
    assert!(logs.contains("rule=\"query-signature\""), "{logs}");
    assert!(logs.contains("path=\"/items\""), "{logs}");
    assert!(!logs.contains("alert"), "{logs}");

    // An exempt body and harmless requests log nothing.
    for request in [text_post(EXEMPT, CODE_SAMPLE), text_post("/items", "fine")] {
        let (status, _, logs) = logged(app.clone(), request).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(logs, "");
    }

    let request = Request::post("/items")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_ENCODING, "br")
        .body(Body::from("fine"))
        .unwrap();
    let (status, _, logs) = logged(app.clone(), request).await;
    assert_eq!(status, StatusCode::OK);
    assert!(logs.contains("rule=\"body-encoding\""), "{logs}");

    let request = text_post("/items", vec![0xff, 0xfe]);
    let (status, body, logs) = logged(app.clone(), request).await;
    assert_eq!((status, &body[..]), (StatusCode::OK, &[0xff, 0xfe][..]));
    assert!(logs.contains("rule=\"body-not-utf8\""), "{logs}");
}

#[tokio::test]
async fn development_replays_bodies_over_the_inspection_limit() {
    let app = baseline_app(Environment::Development);
    for (length, warned) in [(1_048_576, false), (1_048_577, true)] {
        // No Content-Length: the limit is found while reading.
        let (status, body, logs) =
            logged(app.clone(), text_post("/items", vec![b'a'; length])).await;
        assert_eq!((status, body.len()), (StatusCode::OK, length));
        assert!(body.iter().all(|byte| *byte == b'a'));
        assert_eq!(
            logs.contains("rule=\"body-too-large\""),
            warned,
            "{length}: {logs}"
        );
    }
    let declared = Request::post("/items")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_LENGTH, "1048577")
        .body(Body::from(vec![b'a'; 1_048_577]))
        .unwrap();
    let (status, body, logs) = logged(app, declared).await;
    assert_eq!((status, body.len()), (StatusCode::OK, 1_048_577));
    assert!(logs.contains("rule=\"body-too-large\""), "{logs}");
}

#[tokio::test]
async fn the_test_environment_mounts_no_waf() {
    let (status, _, logs) = logged(
        baseline_app(Environment::Test),
        text_post("/items", CODE_SAMPLE),
    )
    .await;
    assert_eq!((status, logs.as_str()), (StatusCode::OK, ""));
}
