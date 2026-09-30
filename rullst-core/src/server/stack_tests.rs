#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Router;
use crate::lifecycle::ApplicationLifecycle;
use crate::resilience::{RateLimitConfig, RateLimiter};
use crate::security::ClientAddr;
use axum::body::{Body, to_bytes};
use axum::extract::{ConnectInfo, Extension};
use axum::http::{Request, StatusCode};
use std::net::SocketAddr;
use tower::ServiceExt;

const PROXY: &str = "10.0.0.1:443";

fn application() -> Router {
    Router::new().route(
        "/",
        axum::routing::get(
            |ConnectInfo(peer): ConnectInfo<SocketAddr>,
             Extension(client): Extension<ClientAddr>| async move {
                format!("{peer}|{}", client.via_trusted_proxy())
            },
        ),
    )
}

async fn send(app: &axum::Router, peer: &str, client: &str) -> (StatusCode, String) {
    let mut request = Request::builder()
        .uri("/")
        .header("x-forwarded-for", client)
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn trusted_proxy_is_the_outermost_server_layer() {
    // TM-CORE-03: the rate limiter, lifecycle and production baseline all run
    // inside the trusted-proxy layer and observe the resolved client.
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let app = Server::new(application())
        .rate_limit(RateLimiter::new(RateLimitConfig::new(1.0, 0.001)))
        .with_lifecycle(lifecycle)
        .trusted_proxies(TrustedProxyConfig::new(["10.0.0.0/8"]).unwrap())
        .into_static_app(SecurityConfig::default(), Environment::Production)
        .unwrap();

    let (status, body) = send(&app, PROXY, "203.0.113.10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "203.0.113.10:0|true");
    assert_eq!(
        send(&app, PROXY, "203.0.113.10").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(send(&app, PROXY, "203.0.113.20").await.0, StatusCode::OK);

    // Forged headers from a direct client are ignored; it keeps its socket bucket.
    let (status, body) = send(&app, "203.0.113.30:1000", "198.51.100.1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "203.0.113.30:1000|false");
    assert_eq!(
        send(&app, "203.0.113.30:1001", "198.51.100.2").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[test]
fn toml_policy_applies_unless_the_builder_selects_one() {
    let mut security = SecurityConfig {
        trusted_proxies: vec!["10.0.0.0/8".to_string()],
        ..SecurityConfig::default()
    };

    let mut from_toml = Server::new(Router::new());
    from_toml.resolve_trusted_proxy(&security).unwrap();
    let layer = from_toml
        .trusted_proxy_layer()
        .expect("enabled TOML policy");
    assert!(layer.config().is_trusted("10.1.2.3".parse().unwrap()));

    let mut explicit = Server::new(Router::new()).trusted_proxies(TrustedProxyConfig::default());
    explicit.resolve_trusted_proxy(&security).unwrap();
    assert!(explicit.trusted_proxy_layer().is_none());

    let mut unset = Server::new(Router::new());
    unset
        .resolve_trusted_proxy(&SecurityConfig::default())
        .unwrap();
    assert!(unset.trusted_proxy_layer().is_none());

    security.trusted_proxies = vec!["0.0.0.0/0".to_string()];
    assert!(matches!(
        Server::new(Router::new()).resolve_trusted_proxy(&security),
        Err(ServerError::Configuration(_))
    ));
}

#[tokio::test]
async fn hot_reload_service_resolves_clients_behind_trusted_proxies() {
    use crate::server::hotswap::{ConnectedHotSwap, HotSwapService};
    use std::sync::{Arc, Mutex, RwLock};
    use tower_service::Service;

    let router: axum::Router = application().into();
    let inner = HotSwapService {
        current_router: Arc::new(RwLock::new(router)),
        active_libraries: Arc::new(Mutex::new(Vec::new())),
        hmr_sender: tokio::sync::broadcast::channel(4).0,
        reload_lock: Arc::new(tokio::sync::Mutex::new(())),
        reload_token: Arc::from("0".repeat(64)),
        lib_path: String::new(),
        is_dev: true,
        shield: None,
        limiter: Some(RateLimiter::new(RateLimitConfig::new(1.0, 0.001))),
        lifecycle: None,
        trusted_proxy: Some(crate::security::TrustedProxyLayer::new(
            TrustedProxyConfig::new(["10.0.0.0/8"]).unwrap(),
        )),
    };
    let mut connection = ConnectedHotSwap {
        inner,
        peer: PROXY.parse().unwrap(),
    };
    let mut status = async |client: &str| {
        let request = Request::builder()
            .uri("/")
            .header("x-forwarded-for", client)
            .body(Body::empty())
            .unwrap();
        let response = connection.call(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    };
    assert_eq!(
        status("203.0.113.10").await,
        (StatusCode::OK, "203.0.113.10:0|true".to_string())
    );
    assert_eq!(
        status("203.0.113.10").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(status("203.0.113.20").await.0, StatusCode::OK);
}
