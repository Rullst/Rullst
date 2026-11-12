//! The telemetry receiver behind Rullst's production security baseline:
//! device bearer tokens through a machine endpoint, revocation, ambient
//! browser credentials and a route mounted without the endpoint.

#![cfg(feature = "iot-ingest")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rullst::config::{Environment, SecurityConfig};
use rullst::iot::SensorTelemetry;
use rullst::iot_ingest::{
    FIRMWARE_HEADER, MemoryDeviceDirectory, TelemetryBatch, TelemetryReceiver,
    TelemetryReceiverConfig,
};
use rullst::realtime::BroadcastManager;
use rullst::security::{MachineEndpointPolicy, apply_security_baseline_with_machine_endpoints};
use rullst::web::axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

const PATH: &str = "/iot/telemetry";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn body(device: &str) -> String {
    serde_json::to_string(&vec![
        SensorTelemetry::new(device, "temperature_c", 21.5, now()),
        SensorTelemetry::new(device, "sine", 0.25, now()),
    ])
    .unwrap()
}

fn post(token: Option<&str>, payload: String) -> Request<Body> {
    let mut request = Request::post(PATH)
        .header(header::CONTENT_TYPE, "application/json")
        .header(FIRMWARE_HEADER, "uno-sensor-1.0");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    request.body(Body::from(payload)).unwrap()
}

async fn status_and_text(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// Two devices behind the production baseline.
fn production_app() -> (
    Router,
    TelemetryReceiver<MemoryDeviceDirectory>,
    Arc<BroadcastManager>,
    [String; 2],
) {
    let directory = MemoryDeviceDirectory::new();
    let first = directory.register("uno-a", "Uno A").unwrap();
    let second = directory.register("uno-b", "Uno B").unwrap();
    let channels = Arc::new(BroadcastManager::new());
    let receiver = TelemetryReceiver::new(
        directory,
        Arc::clone(&channels),
        TelemetryReceiverConfig::new(),
    );
    let policy =
        MachineEndpointPolicy::new(vec![receiver.machine_endpoint(PATH).unwrap()]).unwrap();
    let app = apply_security_baseline_with_machine_endpoints(
        receiver.routes(PATH).into_axum(),
        SecurityConfig::default(),
        Environment::Production,
        policy,
    )
    .unwrap();
    let tokens = [first.expose().to_owned(), second.expose().to_owned()];
    (app, receiver, channels, tokens)
}

#[tokio::test]
async fn devices_stream_with_their_own_tokens_until_revoked() {
    let (app, receiver, channels, [token_a, token_b]) = production_app();
    let mut subscriber = channels.get_or_create("iot.telemetry").subscribe();

    let (status, text) = status_and_text(&app, post(Some(&token_a), body("uno-a"))).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{text}");
    assert_eq!(text, r#"{"accepted":2}"#);
    let (status, _) = status_and_text(&app, post(Some(&token_b), body("uno-b"))).await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let first: TelemetryBatch =
        serde_json::from_str(&subscriber.recv().await.unwrap().payload).unwrap();
    let second: TelemetryBatch =
        serde_json::from_str(&subscriber.recv().await.unwrap().payload).unwrap();
    assert_eq!(
        (first.device_id.as_str(), second.device_id.as_str()),
        ("uno-a", "uno-b")
    );
    assert_eq!(first.firmware_version.as_deref(), Some("uno-sensor-1.0"));
    let record = receiver.directory().get("uno-b").unwrap();
    assert!(record.last_seen_at.is_some());

    // One device's token cannot speak for another.
    let (status, text) = status_and_text(&app, post(Some(&token_a), body("uno-b"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(text, r#"{"error":"device_mismatch","index":0}"#);

    // A revoked token is refused at once; the other device keeps working.
    assert_eq!(receiver.directory().revoke("uno-a"), Ok(true));
    let (status, _) = status_and_text(&app, post(Some(&token_a), body("uno-a"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = status_and_text(&app, post(Some(&token_b), body("uno-b"))).await;
    assert_eq!(status, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn missing_wrong_duplicate_or_ambient_credentials_are_refused() {
    let (app, _receiver, _channels, [token, _]) = production_app();
    let wrong = "rdt1_".to_owned() + &"Z".repeat(32);
    for request in [
        post(None, body("uno-a")),
        post(Some(&wrong), body("uno-a")),
        post(Some("short"), body("uno-a")),
    ] {
        let (status, _) = status_and_text(&app, request).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    let mut duplicate = post(Some(&token), body("uno-a"));
    duplicate.headers_mut().append(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    assert_eq!(
        status_and_text(&app, duplicate).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Browser credentials never substitute for the device token.
    for (name, value) in [
        (header::COOKIE.as_str(), "rullst_session=abc"),
        (header::ORIGIN.as_str(), "https://evil.example"),
        ("sec-fetch-site", "cross-site"),
    ] {
        let mut request = post(Some(&token), body("uno-a"));
        request.headers_mut().insert(name, value.parse().unwrap());
        assert_eq!(
            status_and_text(&app, request).await.0,
            StatusCode::UNAUTHORIZED,
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_route_without_its_machine_endpoint_accepts_nothing() {
    let directory = MemoryDeviceDirectory::new();
    let token = directory.register("uno-a", "Uno A").unwrap();
    let receiver = TelemetryReceiver::new(
        directory,
        Arc::new(BroadcastManager::new()),
        TelemetryReceiverConfig::new(),
    );
    // Mounted without the security baseline or a policy: the handler
    // still refuses, because no verifier attached a device identity.
    let bare = receiver.routes(PATH).into_axum();
    let (status, text) = status_and_text(&bare, post(Some(token.expose()), body("uno-a"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(text, r#"{"error":"unauthenticated"}"#);
    assert!(
        receiver
            .directory()
            .get("uno-a")
            .unwrap()
            .last_seen_at
            .is_none()
    );
}
