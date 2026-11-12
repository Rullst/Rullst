#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use rullst_core::realtime::BroadcastManager;
use rullst_core::web::axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use rullst_core::web::axum::response::IntoResponse;
use rullst_iot::SensorTelemetry;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) const NOW: u64 = 1_800_000_000;

pub(super) fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers
}

pub(super) fn reading(device: &str, metric: &str, value: f64, timestamp: u64) -> SensorTelemetry {
    SensorTelemetry::new(device, metric, value, timestamp)
}

pub(super) fn receiver(
    config: TelemetryReceiverConfig,
) -> TelemetryReceiver<MemoryDeviceDirectory> {
    let directory = MemoryDeviceDirectory::new();
    directory.register("uno-a", "Uno A").unwrap();
    TelemetryReceiver::new(directory, Arc::new(BroadcastManager::new()), config)
}

pub(super) async fn accept(
    receiver: &TelemetryReceiver<MemoryDeviceDirectory>,
    headers: &HeaderMap,
    body: &str,
) -> Result<IngestReceipt, IngestError> {
    let device = AuthenticatedDevice::for_tests("uno-a");
    receiver
        .accept_at(&device, headers, body.as_bytes(), NOW, Instant::now())
        .await
}

#[test]
fn tokens_are_random_prefixed_and_redacted() {
    let first = DeviceToken::generate();
    let second = DeviceToken::generate();
    assert_ne!(first, second);
    assert!(first.expose().starts_with("rdt1_"));
    assert_eq!(first.expose().len(), 37);
    assert!(token::plausible_token(first.expose()));
    assert_eq!(format!("{first:?}"), "DeviceToken(<redacted>)");
    assert_eq!(first.hash(), TokenHash::of(first.expose()));
    assert_ne!(first.hash(), second.hash());
}

#[test]
fn token_hashes_are_sha256_hex() {
    // SHA-256("abc").
    let hash = TokenHash::of("abc");
    assert_eq!(
        hash.as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(TokenHash::from_hex(hash.as_str()), Some(hash.clone()));
    assert_eq!(TokenHash::from_hex(hash.as_str().to_uppercase()), None);
    assert_eq!(TokenHash::from_hex("ab"), None);
    assert!(!token::plausible_token("short"));
    assert!(!token::plausible_token(&format!(
        "{} space",
        "x".repeat(40)
    )));
    assert!(!token::plausible_token(&"x".repeat(201)));
    assert!(token::plausible_token(&"x".repeat(200)));
}

#[tokio::test]
async fn the_memory_directory_registers_finds_and_revokes() {
    let directory = MemoryDeviceDirectory::new();
    let token = directory.register("uno-a", "Uno A").unwrap();
    assert_eq!(
        directory.register("uno-a", "again"),
        Err(DirectoryError::AlreadyRegistered)
    );
    for (id, name) in [("", "x"), ("a b", "x"), ("ok", ""), ("ok", "line\nbreak")] {
        assert_eq!(
            directory.register(id, name),
            Err(DirectoryError::InvalidDevice)
        );
    }
    assert_eq!(
        directory.register("x".repeat(65), "too long"),
        Err(DirectoryError::InvalidDevice)
    );

    let found = directory.find_active(&token.hash()).await.unwrap().unwrap();
    assert_eq!(found.device_id, "uno-a");
    assert!(
        directory
            .find_active(&TokenHash::of("other"))
            .await
            .unwrap()
            .is_none()
    );

    directory
        .record_seen("uno-a", NOW, Some("v1"))
        .await
        .unwrap();
    directory.record_seen("uno-a", NOW + 1, None).await.unwrap();
    let record = directory.get("uno-a").unwrap();
    assert_eq!(record.last_seen_at, Some(NOW + 1));
    assert_eq!(record.firmware_version.as_deref(), Some("v1"));
    assert_eq!(
        directory.record_seen("ghost", NOW, None).await,
        Err(DirectoryError::NotFound)
    );

    assert_eq!(directory.revoke("uno-a"), Ok(true));
    assert_eq!(directory.revoke("uno-a"), Ok(false));
    assert_eq!(directory.revoke("ghost"), Err(DirectoryError::NotFound));
    assert!(
        directory
            .find_active(&token.hash())
            .await
            .unwrap()
            .is_none()
    );
    assert!(directory.list()[0].revoked);
}

#[tokio::test]
async fn accepted_readings_are_published_as_one_batch() {
    let receiver = receiver(TelemetryReceiverConfig::new());
    let mut subscriber = receiver
        .inner_channels()
        .get_or_create("iot.telemetry")
        .subscribe();
    let mut headers = json_headers();
    headers.insert(FIRMWARE_HEADER, HeaderValue::from_static("uno-sensor-1.0"));
    let body = serde_json::to_string(&vec![
        reading("uno-a", "temperature_c", 7.1, NOW),
        reading("uno-a", "sine", -0.5, NOW - 10),
    ])
    .unwrap();

    let receipt = accept(&receiver, &headers, &body).await.unwrap();
    assert_eq!(receipt.accepted, 2);
    assert_eq!(receipt.delivered, 1);
    let message = subscriber.try_recv().unwrap();
    assert_eq!(message.event, "telemetry");
    let batch: TelemetryBatch = serde_json::from_str(&message.payload).unwrap();
    assert_eq!(batch.device_id, "uno-a");
    assert_eq!(batch.received_at, NOW);
    assert_eq!(batch.firmware_version.as_deref(), Some("uno-sensor-1.0"));
    assert_eq!(batch.readings.len(), 2);
    assert_eq!(batch.readings[1].metric, "sine");

    let record = receiver.directory().get("uno-a").unwrap();
    assert_eq!(record.last_seen_at, Some(NOW));
    assert_eq!(record.firmware_version.as_deref(), Some("uno-sensor-1.0"));

    // A single object is one reading; nobody listening is not an error.
    drop(subscriber);
    let one = serde_json::to_string(&reading("uno-a", "sine", 0.0, NOW)).unwrap();
    let receipt = accept(&receiver, &json_headers(), &one).await.unwrap();
    assert_eq!((receipt.accepted, receipt.delivered), (1, 0));
}

#[tokio::test]
async fn invalid_requests_get_typed_refusals() {
    let receiver = receiver(TelemetryReceiverConfig::new().with_max_readings(2));
    let ok = |metric: &str, value: f64, ts: u64| reading("uno-a", metric, value, ts);
    let body = |readings: Vec<SensorTelemetry>| serde_json::to_string(&readings).unwrap();

    let mut text = HeaderMap::new();
    text.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    let one = body(vec![ok("t", 1.0, NOW)]);
    assert_eq!(
        accept(&receiver, &text, &one).await,
        Err(IngestError::UnsupportedMediaType)
    );
    assert_eq!(
        accept(&receiver, &HeaderMap::new(), &one).await,
        Err(IngestError::UnsupportedMediaType)
    );
    let json = json_headers();
    for malformed in ["", "{", "[]", "{\"device_id\":\"uno-a\"}", "42"] {
        assert_eq!(
            accept(&receiver, &json, malformed).await,
            Err(IngestError::Malformed),
            "{malformed:?}"
        );
    }
    let three = body(vec![ok("t", 1.0, NOW); 3]);
    assert_eq!(
        accept(&receiver, &json, &three).await,
        Err(IngestError::TooManyReadings)
    );
    let other = body(vec![ok("t", 1.0, NOW), reading("uno-b", "t", 1.0, NOW)]);
    assert_eq!(
        accept(&receiver, &json, &other).await,
        Err(IngestError::DeviceMismatch { index: 1 })
    );
    for (bad, reason) in [
        (ok("", 1.0, NOW), "metric"),
        (ok("bad metric", 1.0, NOW), "metric"),
        (ok(&"m".repeat(65), 1.0, NOW), "metric"),
        (ok("t", 1.0, NOW + 301), "future"),
        (ok("t", 1.0, NOW - 86_401), "old"),
    ] {
        let refused = accept(&receiver, &json, &body(vec![bad])).await;
        match refused {
            Err(IngestError::InvalidReading {
                index: 0,
                reason: text,
            }) => {
                assert!(text.contains(reason), "{text}")
            }
            other => panic!("expected an invalid reading, got {other:?}"),
        }
    }
    assert!(
        accept(&receiver, &json, &body(vec![ok("t", 1.0, NOW + 300)]))
            .await
            .is_ok()
    );
    assert!(
        accept(&receiver, &json, &body(vec![ok("t", 1.0, NOW - 86_400)]))
            .await
            .is_ok()
    );

    // JSON has no infinity: serde writes `null`, which is not a number.
    let infinite = body(vec![ok("t", f64::INFINITY, NOW)]);
    assert_eq!(
        accept(&receiver, &json, &infinite).await,
        Err(IngestError::Malformed)
    );
    assert_eq!(
        accept(
            &receiver,
            &json,
            r#"{"device_id":"uno-a","metric":"t","value":1e999,"timestamp":1800000000}"#
        )
        .await,
        Err(IngestError::Malformed)
    );

    let mut firmware = json_headers();
    firmware.insert(FIRMWARE_HEADER, HeaderValue::from_static("has space"));
    assert_eq!(
        accept(&receiver, &firmware, &one).await,
        Err(IngestError::InvalidFirmware)
    );
    let big = format!("[{}]", " ".repeat(16 * 1024));
    assert_eq!(
        accept(&receiver, &json, &big).await,
        Err(IngestError::PayloadTooLarge)
    );
}

#[tokio::test]
async fn each_device_is_rate_limited_in_its_window() {
    let config = TelemetryReceiverConfig::new().with_rate_limit(2, Duration::from_secs(10));
    let receiver = receiver(config);
    receiver.directory().register("uno-b", "Uno B").unwrap();
    let body = serde_json::to_string(&reading("uno-a", "t", 1.0, NOW)).unwrap();
    let json = json_headers();
    let start = Instant::now();
    let device = AuthenticatedDevice::for_tests("uno-a");
    let at = |offset: u64| start + Duration::from_secs(offset);
    for offset in [0, 1] {
        receiver
            .accept_at(&device, &json, body.as_bytes(), NOW, at(offset))
            .await
            .unwrap();
    }
    assert_eq!(
        receiver
            .accept_at(&device, &json, body.as_bytes(), NOW, at(4))
            .await,
        Err(IngestError::RateLimited {
            retry_after: Duration::from_secs(6)
        })
    );
    // Another device has its own budget; the window then starts over.
    let other = AuthenticatedDevice::for_tests("uno-b");
    let other_body = serde_json::to_string(&reading("uno-b", "t", 1.0, NOW)).unwrap();
    assert!(
        receiver
            .accept_at(&other, &json, other_body.as_bytes(), NOW, at(4))
            .await
            .is_ok()
    );
    assert!(
        receiver
            .accept_at(&device, &json, body.as_bytes(), NOW, at(10))
            .await
            .is_ok()
    );
}

#[test]
fn refusals_have_stable_codes_statuses_and_bodies() {
    let cases = [
        (
            IngestError::Unauthenticated,
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            IngestError::UnsupportedMediaType,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
        ),
        (
            IngestError::PayloadTooLarge,
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
        ),
        (
            IngestError::TooManyReadings,
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_many_readings",
        ),
        (IngestError::Malformed, StatusCode::BAD_REQUEST, "malformed"),
        (
            IngestError::InvalidFirmware,
            StatusCode::BAD_REQUEST,
            "invalid_firmware",
        ),
        (
            IngestError::DeviceMismatch { index: 3 },
            StatusCode::FORBIDDEN,
            "device_mismatch",
        ),
        (
            IngestError::InvalidReading {
                index: 1,
                reason: "why",
            },
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_reading",
        ),
        (
            IngestError::RateLimited {
                retry_after: Duration::from_millis(1500),
            },
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
        ),
        (
            IngestError::Unavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
        ),
    ];
    for (error, status, code) in cases {
        assert_eq!(error.status(), status);
        assert_eq!(error.code(), code);
        assert!(!error.to_string().is_empty());
        let response = error.clone().into_response();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            response.headers().contains_key(header::RETRY_AFTER),
            matches!(error, IngestError::RateLimited { .. })
        );
    }
    let response = IngestError::RateLimited {
        retry_after: Duration::ZERO,
    }
    .into_response();
    assert_eq!(response.headers()[header::RETRY_AFTER], "1");
    assert_eq!(
        IngestError::DeviceMismatch { index: 3 }.to_string(),
        "reading 3 belongs to another device"
    );
}

#[test]
fn every_limit_is_clamped_and_names_are_checked() {
    let defaults = TelemetryReceiverConfig::default();
    assert_eq!(defaults, TelemetryReceiverConfig::new());
    assert_eq!(defaults.channel(), "iot.telemetry");
    assert_eq!(defaults.event(), "telemetry");
    assert_eq!(defaults.max_body_bytes(), 16 * 1024);
    assert_eq!(defaults.max_readings(), 64);
    assert_eq!(defaults.requests_per_window(), 120);
    assert_eq!(defaults.window(), Duration::from_secs(60));

    let low = TelemetryReceiverConfig::new()
        .with_max_body_bytes(0)
        .with_max_readings(0)
        .with_rate_limit(0, Duration::ZERO)
        .with_channel("bad channel")
        .with_event("");
    assert_eq!(low.max_body_bytes(), 256);
    assert_eq!(low.max_readings(), 1);
    assert_eq!(low.requests_per_window(), 1);
    assert_eq!(low.window(), Duration::from_secs(1));
    assert_eq!(low.channel(), "iot.telemetry");
    assert_eq!(low.event(), "telemetry");

    let high = TelemetryReceiverConfig::new()
        .with_max_body_bytes(usize::MAX)
        .with_max_readings(usize::MAX)
        .with_rate_limit(u32::MAX, Duration::MAX)
        .with_channel("plant/line-1")
        .with_event("reading.v1");
    assert_eq!(high.max_body_bytes(), 1024 * 1024);
    assert_eq!(high.max_readings(), 1024);
    assert_eq!(high.requests_per_window(), 100_000);
    assert_eq!(high.window(), Duration::from_secs(3600));
    assert_eq!(high.channel(), "plant/line-1");
    assert_eq!(high.event(), "reading.v1");
}

#[test]
fn directory_errors_never_name_a_token() {
    assert_eq!(
        DirectoryError::Unavailable("pool closed".into()).to_string(),
        "device directory unavailable: pool closed"
    );
    assert_eq!(DirectoryError::NotFound.to_string(), "device not found");
    assert_eq!(
        DirectoryError::AlreadyRegistered.to_string(),
        "device is already registered"
    );
    assert_eq!(
        DirectoryError::InvalidDevice.to_string(),
        "invalid device id or name"
    );
}
