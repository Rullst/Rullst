//! MQTT 5 PUBLISH and CoAP request bytes, and the anomaly detector.

use super::Ctx;
use crate::{Ms, expected as x, outln, timed};
use alloc::{vec, vec::Vec};
use rullst_iot::{
    AnomalyDetector, AnomalyState, CoapCodecError, CoapMessageType, CoapMethod, CoapRequest,
    MqttCodecError, MqttPublish, MqttQos,
};

pub(super) fn mqtt(ctx: &mut Ctx) {
    let (readme, cycles) = timed(|| {
        MqttPublish::reliable(
            "factory/line-1/temperature",
            b"24.5".to_vec(),
            MqttQos::AtLeastOnce,
            7,
        )
        .and_then(|p| p.encode())
    });
    match readme {
        Ok(bytes) => {
            ctx.bytes("mqtt_qos1_readme", &bytes, x::MQTT_README_QOS1);
            outln!(
                "INFO time mqtt_encode_37B {} ({} cycles)",
                Ms(cycles),
                cycles
            );
        }
        Err(e) => ctx.check("mqtt_qos1_readme", false, format_args!("{:?}", e)),
    }
    let retained = MqttPublish::new("sensors/microbit-v2/temp", br#"{"t":23.75}"#.to_vec())
        .map(|p| p.retained(true))
        .and_then(|p| p.encode());
    match retained {
        Ok(bytes) => ctx.bytes("mqtt_qos0_retained", &bytes, x::MQTT_QOS0_RETAINED),
        Err(e) => ctx.check("mqtt_qos0_retained", false, format_args!("{:?}", e)),
    }
    let payload: Vec<u8> = (0..200u32).map(|i| (i * 7) as u8).collect();
    let qos2 = MqttPublish::reliable("a/b", payload, MqttQos::ExactlyOnce, 0xBEEF)
        .and_then(|p| p.duplicate(true))
        .and_then(|p| p.encode());
    match qos2 {
        Ok(bytes) => ctx.bytes("mqtt_qos2_dup_varint_len", &bytes, x::MQTT_QOS2_DUP_200B),
        Err(e) => ctx.check("mqtt_qos2_dup_varint_len", false, format_args!("{:?}", e)),
    }
    ctx.expect_err(
        "mqtt_reject_wildcard",
        MqttPublish::new("sensors/+", Vec::<u8>::new()),
        MqttCodecError::InvalidTopic,
    );
    ctx.expect_err(
        "mqtt_reject_zero_packet_id",
        MqttPublish::reliable("a", b"x".to_vec(), MqttQos::AtLeastOnce, 0),
        MqttCodecError::PacketIdentifierRequired,
    );
    ctx.expect_err(
        "mqtt_reject_qos0_dup",
        MqttPublish::new("a", Vec::<u8>::new()).and_then(|p| p.duplicate(true)),
        MqttCodecError::DuplicateRequiresReliableQos,
    );
}

pub(super) fn coap(ctx: &mut Ctx) {
    let (readme, cycles) = timed(|| {
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Post,
            42,
            [0x01, 0x02],
        )
        .and_then(|r| r.path_segment("telemetry"))
        .map(|r| r.content_format(50))
        .and_then(|r| r.payload(br#"{"temperature":24.5}"#.to_vec()))
        .and_then(|r| r.encode())
    });
    match readme {
        Ok(bytes) => {
            ctx.bytes("coap_con_post_readme", &bytes, x::COAP_README_POST);
            outln!(
                "INFO time coap_encode_39B {} ({} cycles)",
                Ms(cycles),
                cycles
            );
        }
        Err(e) => ctx.check("coap_con_post_readme", false, format_args!("{:?}", e)),
    }
    let get = CoapRequest::new(
        CoapMessageType::NonConfirmable,
        CoapMethod::Get,
        0xA1B2,
        Vec::<u8>::new(),
    )
    .and_then(|r| r.path_segment("devices"))
    .and_then(|r| r.path_segment("microbit-v2-serial-0001"))
    .and_then(|r| r.path_segment("state"))
    .and_then(|r| r.encode());
    match get {
        Ok(bytes) => ctx.bytes("coap_non_get_ext_option_len", &bytes, x::COAP_NON_GET_3SEG),
        Err(e) => ctx.check(
            "coap_non_get_ext_option_len",
            false,
            format_args!("{:?}", e),
        ),
    }
    let put = CoapRequest::new(
        CoapMessageType::Confirmable,
        CoapMethod::Put,
        0xFFFF,
        [9u8; 8],
    )
    .and_then(|r| r.path_segment("fw"))
    .map(|r| r.content_format(11542))
    .and_then(|r| r.payload(vec![0xAB; 300]))
    .and_then(|r| r.encode());
    match put {
        Ok(bytes) => ctx.bytes("coap_con_put_cf2b_300B", &bytes, x::COAP_CON_PUT_CF2B_300B),
        Err(e) => ctx.check("coap_con_put_cf2b_300B", false, format_args!("{:?}", e)),
    }
    let delete = CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Delete, 1, [0x7f])
        .map(|r| r.content_format(0))
        .and_then(|r| r.encode());
    match delete {
        Ok(bytes) => ctx.bytes("coap_delete_cf0_empty_value", &bytes, x::COAP_DELETE_CF0),
        Err(e) => ctx.check(
            "coap_delete_cf0_empty_value",
            false,
            format_args!("{:?}", e),
        ),
    }
    ctx.expect_err(
        "coap_reject_token_9B",
        CoapRequest::new(CoapMessageType::Confirmable, CoapMethod::Get, 1, [0u8; 9]),
        CoapCodecError::TokenTooLong,
    );
    let base = || {
        CoapRequest::new(
            CoapMessageType::Confirmable,
            CoapMethod::Get,
            1,
            Vec::<u8>::new(),
        )
    };
    ctx.expect_err(
        "coap_reject_dotdot",
        base().and_then(|r| r.path_segment("..")),
        CoapCodecError::InvalidPathSegment,
    );
    ctx.expect_err(
        "coap_reject_segment_256B",
        base().and_then(|r| {
            r.path_segment(alloc::string::String::from_utf8(vec![b'a'; 256]).unwrap_or_default())
        }),
        CoapCodecError::OptionTooLarge,
    );
    ctx.expect_err(
        "coap_reject_payload_1153B",
        base().and_then(|r| r.payload(vec![0u8; 1153])),
        CoapCodecError::DatagramTooLarge,
    );
}

pub(super) fn anomaly(ctx: &mut Ctx) {
    let series: [f64; 12] = [
        25.0,
        26.0,
        29.99,
        30.0,
        30.01,
        35.0,
        35.01,
        15.0,
        14.99,
        -1.0e9,
        f64::NAN,
        f64::NEG_INFINITY,
    ];
    let detector = AnomalyDetector::new(25.0, 5.0);
    let mut states = [0u8; 12];
    let (_, cycles) = timed(|| {
        for (slot, value) in states.iter_mut().zip(series) {
            *slot = match detector.evaluate(value) {
                AnomalyState::Normal => 0,
                AnomalyState::Warning => 1,
                AnomalyState::CriticalAnomaly => 2,
            };
        }
    });
    let count = |s| states.iter().filter(|&&v| v == s).count();
    ctx.bytes("anomaly_series_matches_host", &states, x::ANOMALY_STATES);
    outln!(
        "INFO anomaly normal={} warning={} critical={} time_12_samples={} ({} cycles)",
        count(0),
        count(1),
        count(2),
        Ms(cycles),
        cycles
    );
    let bad_config = AnomalyDetector::new(25.0, -1.0).evaluate(25.0)
        == AnomalyState::CriticalAnomaly
        && AnomalyDetector::new(f64::NAN, 1.0).evaluate(25.0) == AnomalyState::CriticalAnomaly;
    ctx.check(
        "anomaly_bad_config_fails_closed",
        bad_config,
        format_args!("negative tolerance / NaN mean -> CriticalAnomaly"),
    );
}
