//! A `rullst-security` layer and the Nexus Threat Radar share one security
//! store: Nexus reads `rullst_core::security::SecurityStore`, which the layer
//! records into through its `rullst_security` re-export. Its own test binary,
//! because it records into the process-wide store.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::{authenticated_test_router, local_request};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    routing::get,
};
use rullst_nexus::Nexus;
use rullst_security::{HoneypotLayer, HoneypotState};
use std::net::SocketAddr;
use tower::ServiceExt;

const PEER: &str = "198.51.100.47";

async fn security_page(nexus: &Router) -> String {
    let request = local_request()
        .uri("/security")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();
    let response = nexus.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn a_honeypot_hit_recorded_by_rullst_security_appears_in_the_threat_radar() {
    assert!(std::ptr::eq(
        rullst_security::SecurityStore::global(),
        rullst_core::security::SecurityStore::global(),
    ));
    let nexus = authenticated_test_router(Nexus::new());
    let details = format!("Peer {PEER} accessed trap route /.env");
    let before = security_page(&nexus).await;
    assert!(!before.contains(&details));

    let app = Router::new()
        .route("/{*path}", get(|| async { StatusCode::OK }))
        .layer(HoneypotLayer::new(HoneypotState::new(vec![
            "/.env".to_string(),
        ])));
    let mut trap = Request::builder().uri("/.env").body(Body::empty()).unwrap();
    trap.extensions_mut().insert(ConnectInfo(
        format!("{PEER}:4100").parse::<SocketAddr>().unwrap(),
    ));
    let response = app.oneshot(trap).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let after = security_page(&nexus).await;
    assert!(after.contains("HONEYPOT_TRAP_TRIGGERED"), "{after}");
    assert!(after.contains(&details), "{after}");
    assert!(after.contains("UNSIGNED LOCAL EVENT"), "{after}");
    assert!(
        rullst_core::security::SecurityStore::global().is_banned(PEER),
        "the radar's ban list is the layer's"
    );
}
