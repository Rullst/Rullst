//! Nexus Basic Auth behind Core's trusted-proxy layer.

use super::*;
use axum::{Router, http::Request, middleware, routing::get};
use rullst_core::security::{TrustedProxyConfig, TrustedProxyLayer};
use tower::ServiceExt;

const PROXY: &str = "10.0.0.1:443";

fn secret(label: &str) -> String {
    format!(
        "dyn_{label}_cred_{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

fn authorization(password: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(format!("ops:{password}"));
    format!("Basic {encoded}")
}

fn proxied_router(credentials: NexusBasicAuth, trust_forwarded_proto: bool) -> Router {
    let proxies = TrustedProxyConfig::new(["10.0.0.0/8"])
        .expect("valid trusted network")
        .trust_forwarded_proto(trust_forwarded_proto);
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .layer(middleware::from_fn(move |request, next| {
            let credentials = credentials.clone();
            async move { basic_auth_middleware(credentials, request, next).await }
        }))
        .layer(TrustedProxyLayer::new(proxies))
}

fn request(peer: &str, client: &str, proto: &str, password: &str) -> Request<Body> {
    let mut request = Request::builder()
        .uri("/")
        .header(header::AUTHORIZATION, authorization(password))
        .header("x-forwarded-for", client)
        .header("x-forwarded-proto", proto)
        .body(Body::empty())
        .expect("valid test request");
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().expect("test peer")));
    request
}

async fn status(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone()
        .oneshot(request)
        .await
        .expect("router response")
        .status()
}

#[tokio::test]
async fn lockout_isolates_clients_behind_a_trusted_proxy() {
    // TM-NEXUS-01 / TM-CORE-03: one client's failures no longer lock every
    // client that shares the reverse proxy's socket address.
    let password = secret("valid");
    let wrong = secret("wrong");
    let app = proxied_router(NexusBasicAuth::new("ops", &password).unwrap(), true);

    for _ in 1..NEXUS_BASIC_AUTH_MAX_FAILURES {
        let attempt = request(PROXY, "203.0.113.10", "https", &wrong);
        assert_eq!(status(&app, attempt).await, StatusCode::UNAUTHORIZED);
    }
    let locking = request(PROXY, "203.0.113.10", "https", &wrong);
    assert_eq!(status(&app, locking).await, StatusCode::TOO_MANY_REQUESTS);
    let locked = request(PROXY, "203.0.113.10", "https", &password);
    assert_eq!(status(&app, locked).await, StatusCode::TOO_MANY_REQUESTS);

    let other_client = request(PROXY, "203.0.113.20", "https", &password);
    assert_eq!(status(&app, other_client).await, StatusCode::OK);
}

#[tokio::test]
async fn only_a_trusted_proxy_https_report_is_tls_evidence() {
    let password = secret("valid");
    let trusting = proxied_router(NexusBasicAuth::new("ops", &password).unwrap(), true);
    let untrusting = proxied_router(NexusBasicAuth::new("ops", &password).unwrap(), false);

    let proxied = request(PROXY, "203.0.113.10", "https", &password);
    assert_eq!(status(&trusting, proxied).await, StatusCode::OK);

    for (app, peer, proto) in [
        // Proto trust was not enabled for the proxy.
        (&untrusting, PROXY, "https"),
        // The trusted proxy reported plaintext HTTP.
        (&trusting, PROXY, "http"),
        // A direct client forging the header is never TLS evidence.
        (&trusting, "203.0.113.9:5000", "https"),
    ] {
        let attempt = request(peer, "203.0.113.10", proto, &password);
        assert_eq!(
            status(app, attempt).await,
            StatusCode::UPGRADE_REQUIRED,
            "{peer} {proto}"
        );
    }
}
