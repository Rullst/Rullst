#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Requests that a framework layer refuses outside the access log (CSRF,
//! the WAF, the rate limiter, Traffic Shield, lifecycle admission, machine
//! endpoints) get an access-log line naming that layer, in every
//! environment (#845).

use super::*;
use crate::Router;
use crate::lifecycle::ApplicationLifecycle;
use crate::resilience::{RateLimitConfig, RateLimiter, TrafficShield, TrafficShieldConfig};
use crate::security::{MachineEndpoint, MachineEndpointPolicy};
use crate::server::console::Recording;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, StatusCode, header};
use std::net::SocketAddr;
use tower::ServiceExt;

/// The lines this test's thread wrote from `run` on.
async fn lines_of<F: std::future::Future<Output = StatusCode>>(
    run: F,
) -> (StatusCode, Vec<String>) {
    let lines =
        || crate::server::access_log_writer::LINES_HERE.with(|lines| lines.borrow().clone());
    let before = lines().len();
    let status = run.await;
    (status, lines()[before..].to_vec())
}

fn application() -> Router {
    Router::new()
        .route("/", axum::routing::get(|| async { "home" }))
        .route("/submit", axum::routing::post(|| async { "saved" }))
        // rullst-access: public — test fixture whose WAF refusal shows the route, not the path.
        .route("/files/{name}", axum::routing::get(|| async { "file" }))
        .route(
            "/webhooks/billing",
            axum::routing::post(|| async { "processed" }),
        )
}

async fn send(app: &axum::Router, request: Request<Body>) -> StatusCode {
    let mut request = request;
    request.extensions_mut().insert(ConnectInfo(
        "203.0.113.9:4000".parse::<SocketAddr>().unwrap(),
    ));
    app.clone().oneshot(request).await.unwrap().status()
}

fn app(server: Server, environment: Environment) -> axum::Router {
    server
        .into_static_app(SecurityConfig::default(), environment)
        .unwrap()
}

/// `[HTTP] <request> -> <status> (<elapsed> ms) refused by <layer>`.
fn assert_refusal(lines: &[String], request: &str, status: u16, layer: &str) {
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert!(
        line.starts_with(&format!("[HTTP] {request} -> {status} (")),
        "{line}"
    );
    assert!(
        line.ends_with(&format!(" ms) refused by {layer}")),
        "{line}"
    );
}

#[tokio::test]
async fn csrf_refusals_are_logged_in_every_environment() {
    for environment in [
        Environment::Development,
        Environment::Test,
        Environment::Staging,
        Environment::Production,
    ] {
        let app = app(Server::new(application()), environment);
        let (status, lines) = lines_of(send(
            &app,
            Request::post("/submit").body(Body::empty()).unwrap(),
        ))
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{environment:?}");
        assert_refusal(&lines, "POST /submit", 403, "csrf");
        // A request the application answers gets one line, without a layer.
        let (status, lines) =
            lines_of(send(&app, Request::get("/").body(Body::empty()).unwrap())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("[HTTP] GET / -> 200 ("), "{lines:?}");
        assert!(lines[0].ends_with(" ms)"), "{lines:?}");
    }
}

#[tokio::test]
async fn waf_rate_limit_shield_lifecycle_and_machine_refusals_name_their_layer() {
    // The WAF refuses in staging and production.
    let waf = app(Server::new(application()), Environment::Production);
    let (status, lines) = lines_of(send(
        &waf,
        Request::get("/")
            .header(header::USER_AGENT, "Mozilla/5.0 (compatible; GPTBot/1.2)")
            .body(Body::empty())
            .unwrap(),
    ))
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_refusal(&lines, "GET /", 403, "waf");

    let limited = app(
        Server::new(application()).rate_limit(RateLimiter::new(RateLimitConfig::new(1.0, 0.001))),
        Environment::Production,
    );
    let first = lines_of(send(
        &limited,
        Request::get("/").body(Body::empty()).unwrap(),
    ))
    .await;
    assert_eq!(first.0, StatusCode::OK);
    let (status, lines) = lines_of(send(
        &limited,
        Request::get("/").body(Body::empty()).unwrap(),
    ))
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_refusal(&lines, "GET /", 429, "rate-limit");

    let shield = TrafficShield::new(
        TrafficShieldConfig::new()
            .with_db_probe(false)
            .with_max_active_requests(0),
    );
    let shed = app(
        Server::new(application()).shield(shield),
        Environment::Production,
    );
    let (status, lines) =
        lines_of(send(&shed, Request::get("/").body(Body::empty()).unwrap())).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_refusal(&lines, "GET /", 503, "traffic-shield");

    let starting = app(
        Server::new(application()).with_lifecycle(ApplicationLifecycle::new()),
        Environment::Production,
    );
    let (status, lines) = lines_of(send(
        &starting,
        Request::get("/").body(Body::empty()).unwrap(),
    ))
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_refusal(&lines, "GET /", 503, "lifecycle");

    let machines = app(
        Server::new(application()).with_machine_endpoints(
            MachineEndpointPolicy::new(vec![
                MachineEndpoint::bearer(
                    Method::POST,
                    "/webhooks/billing",
                    "refusal-log-machine-token-0123456789abcdef",
                )
                .unwrap(),
            ])
            .unwrap(),
        ),
        Environment::Production,
    );
    let (status, lines) = lines_of(send(
        &machines,
        Request::post("/webhooks/billing")
            .body(Body::empty())
            .unwrap(),
    ))
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_refusal(&lines, "POST /webhooks/billing", 401, "machine-endpoint");
}

#[tokio::test]
async fn refusal_lines_follow_the_access_log_settings() {
    // The matched route, never the request path, under route paths.
    let mut server = Server::new(application());
    server.access_log_route_paths = true;
    let routes = app(server, Environment::Production);
    let (status, lines) = lines_of(send(
        &routes,
        Request::get("/files/secret-token")
            .header(header::USER_AGENT, "Mozilla/5.0 (compatible; GPTBot/1.2)")
            .body(Body::empty())
            .unwrap(),
    ))
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_refusal(&lines, "GET /files/{name}", 403, "waf");

    // `errors` keeps refusals; `off` installs neither log.
    for (recording, written) in [(Recording::Errors, 1), (Recording::Off, 0)] {
        let mut server = Server::new(application());
        server.access_log_recording = recording;
        let app = app(server, Environment::Production);
        let (status, lines) = lines_of(send(
            &app,
            Request::post("/submit").body(Body::empty()).unwrap(),
        ))
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(lines.len(), written, "{recording:?} {lines:?}");
        if written == 1 {
            assert_refusal(&lines, "POST /submit", 403, "csrf");
        }
    }
}

/// A refusal inside the application's own router, such as a second
/// `csrf_middleware`, is written once, by the access log it passes.
#[tokio::test]
async fn a_refusal_inside_the_router_is_written_once() {
    let router = application().layer(axum::middleware::from_fn(
        |request: axum::extract::Request, next: axum::middleware::Next| async move {
            let limiter = RateLimiter::new(RateLimitConfig::new(0.0, 0.001));
            crate::resilience::rate_limit_middleware(limiter, request, next).await
        },
    ));
    let app = app(Server::new(router), Environment::Production);
    let (status, lines) =
        lines_of(send(&app, Request::get("/").body(Body::empty()).unwrap())).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_refusal(&lines, "GET /", 429, "rate-limit");
}
