//! The production monitor inside the server stack (`TM-STUDIO-2`,
//! `MONITOR-01`, `-05`, `-06`, `-07`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use super::*;
use crate::Router;
use crate::monitor::{MONITOR_PATH, Monitor, MonitorVerifiedTls};
use crate::resilience::{RateLimitConfig, RateLimiter};
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use std::net::SocketAddr;
use tower::ServiceExt;

/// `openssl rand -base64 32` output.
const TOKEN: &str = "NXRt2EGKIzhSo4cMVHuHW4oQ8r80oD7vX6VV1XZPpUM=";

fn application() -> Router {
    // rullst-access: public — test fixture without data.
    Router::new().route("/items/{id}", axum::routing::get(|| async { "item" }))
}

async fn call(
    app: &axum::Router,
    uri: &str,
    peer: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, HeaderMap, String) {
    let mut builder = Request::builder().uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    request
        .extensions_mut()
        .insert(MonitorVerifiedTls::from_trusted_tls_termination());
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

fn authorization() -> String {
    format!("Bearer {TOKEN}")
}

// TM-MONITOR-01: no monitor, or one without a credential, mounts nothing.
#[tokio::test]
async fn the_monitor_is_off_unless_mounted_with_a_credential() {
    let authorization = authorization();
    for server in [
        Server::new(application()),
        Server::new(application()).with_monitor(Monitor::new()),
    ] {
        let app = server
            .into_static_app(SecurityConfig::default(), Environment::Production)
            .unwrap();
        let (status, _, body) = call(
            &app,
            MONITOR_PATH,
            "192.0.2.1:4000",
            &[("authorization", &authorization)],
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!body.contains("rullst.monitor"));
    }
}

#[tokio::test]
async fn an_invalid_monitor_configuration_stops_start_up_without_echoing_the_token() {
    let result = Server::new(application())
        .with_monitor(Monitor::new().with_token("weak-secret-value"))
        .into_static_app(SecurityConfig::default(), Environment::Production);
    match result {
        Err(ServerError::Configuration(message)) => {
            assert!(message.contains("monitor token 1"), "{message}");
            assert!(!message.contains("weak-secret-value"));
        }
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("a weak token must stop start-up"),
    }
}

#[tokio::test]
async fn production_serves_the_document_under_the_security_baseline_and_counts_refusals() {
    let app = Server::new(application())
        .rate_limit(RateLimiter::new(RateLimitConfig::new(2.0, 0.001)))
        .with_monitor(Monitor::new().with_token(TOKEN))
        .into_static_app(SecurityConfig::default(), Environment::Production)
        .unwrap();
    let client = "198.51.100.20:5000";
    assert_eq!(
        call(&app, "/items/1?q=a", client, &[]).await.0,
        StatusCode::OK
    );
    assert_eq!(call(&app, "/items/2", client, &[]).await.0, StatusCode::OK);
    // The server's own limiter refuses the third; the monitor counts it.
    assert_eq!(
        call(&app, "/items/3", client, &[]).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );

    let authorization = authorization();
    let (status, headers, body) = call(
        &app,
        MONITOR_PATH,
        "192.0.2.50:4000",
        &[("authorization", &authorization)],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
    assert!(headers.contains_key(header::STRICT_TRANSPORT_SECURITY));
    let document: serde_json::Value = serde_json::from_str(&body).unwrap();
    let routes = document["http"]["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["method"], "GET");
    assert_eq!(routes[0]["route"], "/items/{id}");
    assert_eq!(routes[0]["status_total"]["2xx"], 2);
    assert_eq!(routes[0]["status_total"]["4xx"], 1);
    assert!(!body.contains("/items/1"));
}

#[tokio::test]
async fn an_application_cors_allowlist_never_releases_the_document_to_a_page() {
    let mut security = SecurityConfig::default();
    security.cors_allow_origins = vec!["https://admin.example".to_string()];
    let app = Server::new(application())
        .with_monitor(Monitor::new().with_token(TOKEN))
        .into_static_app(security, Environment::Production)
        .unwrap();
    let authorization = authorization();
    let (status, _, body) = call(
        &app,
        MONITOR_PATH,
        "192.0.2.60:4000",
        &[
            ("authorization", &authorization),
            ("origin", "https://admin.example"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.is_empty());
}
