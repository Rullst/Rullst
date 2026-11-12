//! Boundary, configuration and response-body tests of the receiver.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::tests::{NOW, accept, json_headers, reading, receiver};
use super::*;
use rullst_core::web::axum::http::HeaderValue;
use rullst_core::web::axum::response::IntoResponse;
use std::time::{Duration, Instant};

#[tokio::test]
async fn boundaries_are_inclusive() {
    let receiver = receiver(TelemetryReceiverConfig::new().with_max_body_bytes(1024));
    assert_eq!(receiver.config().max_body_bytes(), 1024);
    let json = json_headers();
    let one = serde_json::to_string(&reading("uno-a", &"m".repeat(64), 1.0, NOW)).unwrap();
    // A body of exactly the limit is accepted; one byte more is not.
    let exact = format!("{one:<1024}");
    assert_eq!(exact.len(), 1024);
    assert!(accept(&receiver, &json, &exact).await.is_ok());
    assert_eq!(
        accept(&receiver, &json, &format!("{exact} ")).await,
        Err(IngestError::PayloadTooLarge)
    );

    let mut firmware = json_headers();
    firmware.insert(
        FIRMWARE_HEADER,
        HeaderValue::from_str(&"v".repeat(64)).unwrap(),
    );
    assert!(accept(&receiver, &firmware, &one).await.is_ok());
    for bad in ["v".repeat(65), String::new()] {
        firmware.insert(FIRMWARE_HEADER, HeaderValue::from_str(&bad).unwrap());
        assert_eq!(
            accept(&receiver, &firmware, &one).await,
            Err(IngestError::InvalidFirmware),
            "{bad:?}"
        );
    }
    let mut twice = json_headers();
    twice.append(FIRMWARE_HEADER, HeaderValue::from_static("v1"));
    twice.append(FIRMWARE_HEADER, HeaderValue::from_static("v2"));
    assert_eq!(
        accept(&receiver, &twice, &one).await,
        Err(IngestError::InvalidFirmware)
    );
}

#[tokio::test]
async fn timestamp_windows_follow_the_configuration() {
    let config = TelemetryReceiverConfig::new()
        .with_timestamp_window(Duration::from_secs(10), Duration::from_secs(120));
    let receiver = receiver(config);
    let json = json_headers();
    let at = |ts: u64| serde_json::to_string(&reading("uno-a", "t", 1.0, ts)).unwrap();
    assert!(accept(&receiver, &json, &at(NOW + 10)).await.is_ok());
    assert!(accept(&receiver, &json, &at(NOW - 120)).await.is_ok());
    assert!(accept(&receiver, &json, &at(NOW + 11)).await.is_err());
    assert!(accept(&receiver, &json, &at(NOW - 121)).await.is_err());

    // Clamped: at most an hour ahead, between a minute and 30 days old.
    let clamped = receiver_with(
        TelemetryReceiverConfig::new().with_timestamp_window(Duration::MAX, Duration::ZERO),
    );
    assert!(accept(&clamped, &json, &at(NOW + 3600)).await.is_ok());
    assert!(accept(&clamped, &json, &at(NOW + 3601)).await.is_err());
    assert!(accept(&clamped, &json, &at(NOW - 60)).await.is_ok());
    assert!(accept(&clamped, &json, &at(NOW - 61)).await.is_err());
    let long = receiver_with(
        TelemetryReceiverConfig::new().with_timestamp_window(Duration::ZERO, Duration::MAX),
    );
    assert!(accept(&long, &json, &at(NOW + 1)).await.is_err());
    assert!(accept(&long, &json, &at(NOW - 30 * 86_400)).await.is_ok());
    assert!(
        accept(&long, &json, &at(NOW - 30 * 86_400 - 1))
            .await
            .is_err()
    );
}

fn receiver_with(config: TelemetryReceiverConfig) -> TelemetryReceiver<MemoryDeviceDirectory> {
    receiver(config)
}

#[tokio::test]
async fn rate_state_is_bounded_by_the_tracked_device_limit() {
    let mut config = TelemetryReceiverConfig::new().with_rate_limit(2, Duration::from_secs(10));
    config.max_tracked_devices = 2;
    let receiver = receiver(config);
    for id in ["uno-b", "uno-c"] {
        receiver.directory().register(id, id).unwrap();
    }
    let json = json_headers();
    let start = Instant::now();
    let send = |id: &'static str, offset: u64| {
        let receiver = receiver.clone();
        let json = json.clone();
        async move {
            let body = serde_json::to_string(&reading(id, "t", 1.0, NOW)).unwrap();
            receiver
                .accept_at(
                    &AuthenticatedDevice::for_tests(id),
                    &json,
                    body.as_bytes(),
                    NOW,
                    start + Duration::from_secs(offset),
                )
                .await
        }
    };
    assert!(send("uno-a", 0).await.is_ok());
    assert!(send("uno-b", 0).await.is_ok());
    // At the bound, a device already tracked keeps its own budget...
    assert!(send("uno-a", 1).await.is_ok());
    // ...while a third device waits for a window to close.
    assert_eq!(
        send("uno-c", 1).await,
        Err(IngestError::RateLimited {
            retry_after: Duration::from_secs(10)
        })
    );
    // Once they expire, their state is dropped and the third is admitted.
    assert!(send("uno-c", 10).await.is_ok());
}

#[test]
fn refusal_bodies_name_the_reading() {
    use rullst_core::web::axum::body::to_bytes;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let body = |error: IngestError| {
        runtime.block_on(async {
            let bytes = to_bytes(error.into_response().into_body(), 1024)
                .await
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        })
    };
    assert_eq!(
        body(IngestError::InvalidReading {
            index: 2,
            reason: "why"
        }),
        r#"{"error":"invalid_reading","index":2,"reason":"why"}"#
    );
    assert_eq!(
        body(IngestError::DeviceMismatch { index: 1 }),
        r#"{"error":"device_mismatch","index":1}"#
    );
    assert_eq!(body(IngestError::Malformed), r#"{"error":"malformed"}"#);
    assert_eq!(
        IngestError::InvalidReading {
            index: 2,
            reason: "why"
        }
        .to_string(),
        "reading 2 is invalid: why"
    );
    assert_eq!(
        IngestError::RateLimited {
            retry_after: Duration::from_millis(100)
        }
        .to_string(),
        "rate limited; retry after 1 s"
    );
    assert_eq!(IngestError::Unavailable.to_string(), "unavailable");
}

#[tokio::test]
async fn ids_names_and_listing_follow_their_rules() {
    let directory = MemoryDeviceDirectory::new();
    assert!(directory.register("x".repeat(64), "n".repeat(128)).is_ok());
    assert_eq!(
        directory.register("ok", "n".repeat(129)),
        Err(DirectoryError::InvalidDevice)
    );
    assert_eq!(validate_device("a.b_c-d", "Name"), Ok(()));
    assert_eq!(
        validate_device("a.b_c-d", "   "),
        Err(DirectoryError::InvalidDevice)
    );
    directory.register("b-device", "B").unwrap();
    directory.register("a-device", "A").unwrap();
    let ids: Vec<_> = directory
        .list()
        .into_iter()
        .map(|record| record.device_id)
        .collect();
    assert_eq!(
        ids,
        ["a-device".to_owned(), "b-device".to_owned(), "x".repeat(64)]
    );
    assert!(directory.get("ghost").is_none());
}

#[test]
fn the_authenticated_device_names_its_token_owner() {
    let device = AuthenticatedDevice::for_tests("uno-a");
    assert_eq!(device.device_id(), "uno-a");
    assert_eq!(device, AuthenticatedDevice::for_tests("uno-a"));
}
