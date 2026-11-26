//! The loopback policy refuses relayed requests and requests a staging or
//! production `Server` received, even from a loopback peer.
#![allow(clippy::expect_used)]

use super::*;
use axum::{
    Router,
    http::{Request, Version},
    middleware,
    routing::get,
};
use tower::ServiceExt;

fn local_request() -> Request<Body> {
    let mut request = Request::builder()
        .uri("/")
        .header(header::HOST, "127.0.0.1:3000")
        .body(Body::empty())
        .expect("valid test request");
    request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:41000"
            .parse::<SocketAddr>()
            .expect("valid loopback peer"),
    ));
    request
}

async fn status(request: Request<Body>) -> StatusCode {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(loopback_only_middleware))
        .oneshot(request)
        .await
        .expect("router response")
        .status()
}

#[tokio::test]
// TM-NEXUS-01: a same-host proxy or tunnel connects from loopback.
async fn requests_relayed_by_a_same_host_proxy_are_refused() {
    assert_eq!(status(local_request()).await, StatusCode::OK);
    for (name, value) in [
        ("forwarded", "for=203.0.113.7"),
        ("x-forwarded-for", "203.0.113.7"),
        ("x-forwarded-host", "admin.example"),
        ("x-forwarded-proto", "https"),
        ("x-real-ip", "203.0.113.7"),
        ("via", "1.1 tunnel"),
        ("cf-connecting-ip", "203.0.113.7"),
    ] {
        let mut request = local_request();
        request
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
        assert_eq!(status(request).await, StatusCode::FORBIDDEN, "{name}");
    }
    // nginx proxies with HTTP/1.0 and adds no forwarding header by default.
    for (version, expected) in [
        (Version::HTTP_10, StatusCode::FORBIDDEN),
        (Version::HTTP_11, StatusCode::OK),
        (Version::HTTP_2, StatusCode::OK),
    ] {
        let mut request = local_request();
        *request.version_mut() = version;
        assert_eq!(status(request).await, expected, "{version:?}");
    }
}

#[tokio::test]
async fn requests_received_by_a_staging_or_production_server_are_refused() {
    for (environment, expected) in [
        (Environment::Development, StatusCode::OK),
        (Environment::Test, StatusCode::OK),
        (Environment::Staging, StatusCode::FORBIDDEN),
        (Environment::Production, StatusCode::FORBIDDEN),
    ] {
        let mut request = local_request();
        request.extensions_mut().insert(environment);
        assert_eq!(status(request).await, expected, "{environment}");
    }
}

#[test]
fn the_refusal_names_the_environment_rule() {
    assert_eq!(
        NexusBuildError::LocalAccessRequiresDebugBuild.to_string(),
        "Nexus loopback-only access is refused: it runs only in a debug build whose environment (RULLST_ENV, APP_ENV, .env or [app] env) is development or test; configure an authenticated policy for release builds and shared environments"
    );
}
