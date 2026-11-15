//! A `rullst-security` layer and Studio share one security store: Studio
//! reads `rullst_core::security::SecurityStore`, which the layer records into
//! through its `rullst_security` re-export. Its own test binary, because it
//! records into the process-wide store.
#![cfg(all(debug_assertions, feature = "local"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
    routing::get,
};
use rullst_security::{HoneypotLayer, HoneypotState};
use rullst_studio::{LocalStudioAccess, Studio};
use std::net::SocketAddr;
use tower::ServiceExt;

const PEER: &str = "198.51.100.23";

fn request(uri: &str, peer: &str) -> Request<Body> {
    let mut request = Request::builder()
        .uri(uri)
        .header(header::HOST, "127.0.0.1:5555")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    request
}

async fn body(router: &Router, uri: &str) -> String {
    let response = router
        .clone()
        .oneshot(request(uri, "127.0.0.1:42000"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{uri}");
    let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn a_honeypot_hit_recorded_by_rullst_security_appears_in_studio() {
    assert!(std::ptr::eq(
        rullst_security::SecurityStore::global(),
        rullst_core::security::SecurityStore::global(),
    ));
    let studio = Studio::new()
        .into_router(LocalStudioAccess::loopback_only())
        .expect("debug Studio router");
    let stats = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();
    let before = stats(&body(&studio, "/studio/security/stats").await);

    let app = Router::new()
        .route("/{*path}", get(|| async { StatusCode::OK }))
        .layer(HoneypotLayer::new(HoneypotState::new(vec![
            "/.env".to_string(),
        ])));
    let trap = app
        .oneshot(request("/.env", &format!("{PEER}:4100")))
        .await
        .unwrap();
    assert_eq!(trap.status(), StatusCode::FORBIDDEN);

    let after = stats(&body(&studio, "/studio/security/stats").await);
    let count = |stats: &serde_json::Value, field: &str| stats[field].as_u64().unwrap();
    assert_eq!(
        count(&after, "honeypot_traps_blocked"),
        count(&before, "honeypot_traps_blocked") + 1
    );
    assert_eq!(
        count(&after, "active_ip_bans"),
        count(&before, "active_ip_bans") + 1
    );
    let details = format!("Peer {PEER} accessed trap route /.env");
    let event = &after["live_events"][0];
    assert_eq!(event["event_type"], "HONEYPOT_TRAP_TRIGGERED");
    assert_eq!(event["client_ip"], PEER);
    assert_eq!(event["details"], details.as_str());
    assert_eq!(event["verified_hmac"], false);

    let page = body(&studio, "/studio/security").await;
    assert!(page.contains("HONEYPOT_TRAP_TRIGGERED"), "{page}");
    assert!(page.contains(&details), "{page}");
}
