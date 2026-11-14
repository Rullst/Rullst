#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::dashboard::{Dashboard, HISTORY_LEN, devices_view, history_view};
use crate::gateway::{Action, plan, readings};
use crate::line::{Parsed, Reject, SensorLine, parse_line};
use crate::registry::IotDevice;
use rullst::db::RowSnapshot;
use rullst::iot::SensorTelemetry;
use rullst::iot_ingest::TelemetryBatch;
use rullst::realtime::{BroadcastManager, RealtimeSse, RealtimeSseConfig};
use std::collections::BTreeMap;
use std::sync::Arc;

fn with_xor(body: &str) -> String {
    let xor = body.bytes().fold(0_u8, |acc, byte| acc ^ byte);
    format!("{body} xor={xor:02X}")
}

fn board_line(device: &str, seq: u32) -> String {
    with_xor(&format!(
        "dev={device} seq={seq} temp_raw=333 temp_c=7.1 sine=0.59"
    ))
}

#[test]
fn board_lines_parse_with_their_device_id() {
    let line = board_line("uno-b", 12) + "\r";
    assert_eq!(
        parse_line(line.as_bytes()),
        Ok(Parsed::Reading(SensorLine {
            device_id: "uno-b".into(),
            seq: 12,
            temp_raw: 333,
            temp_c: 7.1,
            sine: 0.59,
        }))
    );
    assert_eq!(
        parse_line(b"# rullst-uno-sensor fw=2 dev=uno-b\r"),
        Ok(Parsed::Info {
            text: "rullst-uno-sensor fw=2 dev=uno-b".into(),
            firmware: Some("rullst-uno-sensor-2".into()),
        })
    );
    assert_eq!(
        parse_line(b"# hello"),
        Ok(Parsed::Info {
            text: "hello".into(),
            firmware: None
        })
    );
}

#[test]
fn garbled_lines_are_rejected_without_panicking() {
    let good = board_line("uno-a", 1);
    let flipped = good.replacen("seq=1", "seq=2", 1);
    let cases: Vec<(Vec<u8>, Reject)> = vec![
        (b"".to_vec(), Reject::Empty),
        (b"\r".to_vec(), Reject::Empty),
        (flipped.into_bytes(), Reject::BadChecksum),
        (
            b"dev=uno-a seq=99 temp_raw=3\r".to_vec(),
            Reject::NoChecksum,
        ),
        (b"\xff\xfe\x00\x80seq=\xc3\r".to_vec(), Reject::NotUtf8),
        (vec![b'A'; 121], Reject::TooLong(121)),
        (
            with_xor("dev=uno-a seq=5 temp_raw=4000 temp_c=1.0 sine=0.00").into_bytes(),
            Reject::OutOfRange("temp_raw"),
        ),
        (
            with_xor("dev=uno-a seq=5 temp_raw=1 temp_c=900 sine=0.00").into_bytes(),
            Reject::OutOfRange("temp_c"),
        ),
        (
            with_xor("dev=uno-a seq=5 temp_raw=1 temp_c=1 sine=2").into_bytes(),
            Reject::OutOfRange("sine"),
        ),
        (
            with_xor("dev=bad/id seq=5 temp_raw=1 temp_c=1 sine=0").into_bytes(),
            Reject::Field("dev"),
        ),
        (
            with_xor("seq=5 temp_raw=1 temp_c=1 sine=0").into_bytes(),
            Reject::Field("dev"),
        ),
        (
            with_xor("dev=uno-a seq=-5 temp_raw=1 temp_c=1 sine=0").into_bytes(),
            Reject::Field("seq"),
        ),
        (
            with_xor("dev=uno-a seq=5 temp_raw=1 temp_c=inf sine=0").into_bytes(),
            Reject::Field("temp_c"),
        ),
        (
            with_xor("dev=uno-a seq=5 temp_raw=1 temp_c=1 sine=0 extra=1").into_bytes(),
            Reject::Trailing,
        ),
        (b"# bell\x07".to_vec(), Reject::Field("info")),
    ];
    for (bytes, reject) in cases {
        assert_eq!(
            parse_line(&bytes),
            Err(reject.clone()),
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        assert!(!reject.to_string().is_empty());
    }
}

#[test]
fn the_gateway_posts_only_its_own_board() {
    let line = board_line("uno-a", 7);
    match plan(line.as_bytes(), line.len(), "uno-a", 1_800_000_000) {
        Action::Post(readings) => {
            let metrics: Vec<_> = readings.iter().map(|r| r.metric.as_str()).collect();
            assert_eq!(
                metrics,
                [
                    "temperature_c_uncalibrated",
                    "temperature_adc_raw",
                    "sine",
                    "sequence"
                ]
            );
            assert!(readings.iter().all(|r| r.device_id == "uno-a"));
            assert!(readings.iter().all(|r| r.timestamp == 1_800_000_000));
            assert_eq!(readings[3].value, 7.0);
            assert_eq!(readings[1].value, 333.0);
        }
        other => panic!("expected readings, got {other:?}"),
    }
    assert!(matches!(
        plan(line.as_bytes(), line.len(), "uno-b", 0),
        Action::Skip(reason) if reason.contains("uno-a")
    ));
    assert!(matches!(
        plan(b"# rullst-uno-sensor fw=2 dev=uno-a", 34, "uno-a", 0),
        Action::Firmware(version) if version == "rullst-uno-sensor-2"
    ));
    assert!(
        matches!(plan(b"# hello", 7, "uno-a", 0), Action::Skip(reason) if reason == "info: hello")
    );
    // A line cut at the parser's limit reports its real length.
    assert!(matches!(
        plan(&[b'A'; 121], 500, "uno-a", 0),
        Action::Skip(reason) if reason.contains("(500)")
    ));
}

fn device(device_id: &str, revoked: bool, seen: bool) -> IotDevice {
    IotDevice {
        id: 1,
        device_id: device_id.into(),
        name: format!("Board {device_id}"),
        token_hash: "0".repeat(64),
        firmware_version: seen.then(|| "rullst-uno-sensor-2".to_owned()),
        last_seen_at: seen.then_some(1_800_000_000),
        registered_at: 1_799_999_000,
        revoked_at: revoked.then_some(1_800_000_100),
        snapshot: RowSnapshot::default(),
    }
}

fn batch(device_id: &str, seq: f64) -> TelemetryBatch {
    let line = SensorLine {
        device_id: device_id.into(),
        seq: seq as u32,
        temp_raw: 333,
        temp_c: 7.1,
        sine: -0.5,
    };
    let readings: Vec<SensorTelemetry> = readings(&line, 1_800_000_000);
    serde_json::from_value(serde_json::json!({
        "device_id": device_id,
        "received_at": 1_800_000_000 + seq as u64,
        "firmware_version": null,
        "readings": readings,
    }))
    .unwrap()
}

#[test]
fn the_dashboard_shows_every_device_and_the_latest_batches() {
    let dashboard = Dashboard::new(
        Arc::new(BroadcastManager::new()),
        RealtimeSse::new(RealtimeSseConfig::new()),
    );
    for seq in 0..25 {
        dashboard.apply(&batch(
            if seq % 2 == 0 { "uno-a" } else { "uno-b" },
            f64::from(seq),
        ));
    }
    let devices = [
        device("uno-a", false, true),
        device("uno-b", true, true),
        device("uno-c", false, false),
    ];
    let (cards, history) = dashboard.fragments(&devices);
    let cards = cards.into_string();
    for expected in [
        r#"data-device="uno-a""#,
        r#"data-device="uno-b""#,
        r#"data-device="uno-c""#,
        "badge badge-active",
        "badge badge-revoked",
        "never seen",
        "No telemetry since the server started.",
        "Firmware rullst-uno-sensor-2.",
        "13 batches since the server started",
        "<dd data-seq>24</dd>",
    ] {
        assert!(cards.contains(expected), "{expected}\n{cards}");
    }
    let history = history.into_string();
    assert_eq!(history.matches("<tr data-device=").count(), HISTORY_LEN);
    assert!(history.contains("Last 20 batches, newest first"));
    assert!(history.find(">24<").unwrap() < history.find(">23<").unwrap());

    let empty = devices_view(&[], &BTreeMap::new()).into_string();
    assert!(empty.contains("No device is registered yet"));
    assert!(
        history_view(&Default::default())
            .into_string()
            .contains("Last 0 batches")
    );
}

#[test]
fn the_static_assets_are_the_scaffold_copies() {
    let blueprint = |name: &str| -> Vec<u8> {
        let path = format!(
            "{}/../../cargo-rullst/src/blueprints/assets/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(path).unwrap()
    };
    for name in [
        "htmx-2.0.11.min.js",
        "htmx-ext-sse-2.2.4.js",
        "HTMX-LICENSE",
        "HTMX-EXT-SSE-LICENSE",
    ] {
        let copy = std::fs::read(format!("{}/static/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(
            copy == blueprint(name),
            "static/{name} differs from the scaffold asset"
        );
    }
}
