#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::resilience::{RateLimitConfig, RateLimiter, rate_limit_middleware};
use axum::body::{Body, to_bytes};
use axum::extract::Extension;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Router, middleware};
use tower::ServiceExt;

const PROXY: &str = "10.0.0.1:443";

async fn echo(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    client: Option<Extension<ClientAddr>>,
) -> String {
    match client {
        Some(Extension(client)) => format!(
            "{peer}|{}|{}|{}|{:?}",
            client.ip(),
            client.peer(),
            client.via_trusted_proxy(),
            client.forwarded_proto()
        ),
        None => format!("{peer}|none"),
    }
}

fn proxies() -> TrustedProxyConfig {
    TrustedProxyConfig::new(["10.0.0.0/8"])
        .unwrap()
        .trust_forwarded_proto(true)
}

fn request(peer: Option<&str>, forwarded_for: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().uri("/");
    if let Some(value) = forwarded_for {
        request = request
            .header("x-forwarded-for", value)
            .header("x-forwarded-proto", "https");
    }
    let mut request = request.body(Body::empty()).unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    }
    request
}

async fn call(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn layer_replaces_connect_info_only_for_trusted_peers() {
    let app = Router::new()
        .route("/", get(echo))
        .layer(TrustedProxyLayer::new(proxies()));

    let (status, body) = call(&app, request(Some(PROXY), Some("203.0.113.5, 10.0.0.2"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        "203.0.113.5:0|203.0.113.5|10.0.0.1:443|true|Some(Https)"
    );

    // TM-CORE-03: a direct client cannot replace its identity with a header.
    let (_, body) = call(
        &app,
        request(Some("203.0.113.9:5000"), Some("198.51.100.1")),
    )
    .await;
    assert_eq!(
        body,
        "203.0.113.9:5000|203.0.113.9|203.0.113.9:5000|false|None"
    );

    // A malformed chain from the trusted proxy keeps the peer and still serves.
    let (status, body) = call(&app, request(Some(PROXY), Some("not-an-ip"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "10.0.0.1:443|10.0.0.1|10.0.0.1:443|false|Some(Https)");
}

#[tokio::test]
async fn layer_uses_mock_connect_info_and_skips_requests_without_a_peer() {
    let mocked = Router::new()
        .route("/", get(echo))
        .layer(TrustedProxyLayer::new(proxies()))
        .layer(axum::extract::connect_info::MockConnectInfo(
            PROXY.parse::<SocketAddr>().unwrap(),
        ));
    let (_, body) = call(&mocked, request(None, Some("203.0.113.5"))).await;
    assert!(body.starts_with("203.0.113.5:0|203.0.113.5|"), "{body}");

    let observed = tower::service_fn(|request: Request<Body>| async move {
        Ok::<_, std::convert::Infallible>(request.extensions().get::<ClientAddr>().copied())
    });
    let client = TrustedProxyLayer::new(proxies())
        .layer(observed)
        .oneshot(request(None, Some("203.0.113.5")))
        .await
        .unwrap();
    assert_eq!(client, None);
    assert!(TrustedProxyLayer::new(proxies()).config().is_enabled());
}

#[tokio::test]
async fn core_rate_limiter_buckets_each_client_behind_a_trusted_proxy() {
    let limiter = RateLimiter::new(RateLimitConfig::new(1.0, 0.001));
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(middleware::from_fn(move |request, next| {
            rate_limit_middleware(limiter.clone(), request, next)
        }))
        .layer(TrustedProxyLayer::new(proxies()));

    let status = async |peer, client| call(&app, request(Some(peer), Some(client))).await.0;
    assert_eq!(status(PROXY, "203.0.113.10").await, StatusCode::OK);
    assert_eq!(
        status(PROXY, "203.0.113.10").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    // Another client behind the same proxy keeps its own budget.
    assert_eq!(status(PROXY, "203.0.113.20").await, StatusCode::OK);

    // A direct client rotating forged headers stays in its socket bucket.
    assert_eq!(
        status("203.0.113.30:1000", "198.51.100.1").await,
        StatusCode::OK
    );
    assert_eq!(
        status("203.0.113.30:1001", "198.51.100.2").await,
        StatusCode::TOO_MANY_REQUESTS
    );
}
