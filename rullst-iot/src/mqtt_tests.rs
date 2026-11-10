use super::*;
use alloc::vec;

#[test]
fn encodes_qos_zero_and_variable_remaining_length() {
    let packet = MqttPublish::new("sensors/1", [0x18])
        .and_then(|publish| publish.encode())
        .expect("bounded packet should encode");
    assert_eq!(
        packet,
        [
            0x30, 0x0d, 0x00, 0x09, b's', b'e', b'n', b's', b'o', b'r', b's', b'/', b'1', 0x00,
            0x18,
        ]
    );

    let packet = MqttPublish::new("a", vec![7; 128])
        .and_then(|publish| publish.encode())
        .expect("two-byte remaining length should encode");
    assert_eq!(&packet[..3], &[0x30, 0x84, 0x01]);
}

#[test]
fn reliable_packet_requires_identifier_and_controls_flags() {
    assert_eq!(
        MqttPublish::reliable("a", b"x".to_vec(), MqttQos::AtLeastOnce, 0),
        Err(MqttCodecError::PacketIdentifierRequired)
    );
    let packet = MqttPublish::reliable("a", b"x".to_vec(), MqttQos::ExactlyOnce, 0x1234)
        .and_then(|publish| publish.retained(true).duplicate(true))
        .and_then(|publish| publish.encode())
        .expect("QoS 2 retry should encode");
    assert_eq!(packet, [0x3d, 0x07, 0, 1, b'a', 0x12, 0x34, 0, b'x']);
}

#[test]
fn rejects_wildcards_controls_and_unbounded_packets() {
    assert_eq!(
        MqttPublish::new("sensors/+", Vec::<u8>::new()),
        Err(MqttCodecError::InvalidTopic)
    );
    assert_eq!(
        MqttPublish::new("sensors\0temperature", Vec::<u8>::new()),
        Err(MqttCodecError::InvalidTopic)
    );
    assert_eq!(
        MqttPublish::new("a", vec![0; MAX_MQTT_PACKET_BYTES]).and_then(|publish| publish.encode()),
        Err(MqttCodecError::PacketTooLarge)
    );
    assert_eq!(
        MqttPublish::new("a", Vec::<u8>::new()).and_then(|publish| publish.duplicate(true)),
        Err(MqttCodecError::DuplicateRequiresReliableQos)
    );
}

#[test]
fn codec_errors_have_stable_messages() {
    use alloc::string::ToString;

    for (error, message) in [
        (
            MqttCodecError::EmptyTopic,
            "MQTT PUBLISH topic cannot be empty",
        ),
        (
            MqttCodecError::InvalidTopic,
            "MQTT PUBLISH topic contains a disallowed character",
        ),
        (
            MqttCodecError::TopicTooLong,
            "MQTT PUBLISH topic exceeds 65535 UTF-8 bytes",
        ),
        (
            MqttCodecError::PayloadTooLarge,
            "MQTT PUBLISH payload exceeds the local packet ceiling",
        ),
        (
            MqttCodecError::PacketTooLarge,
            "MQTT PUBLISH packet exceeds an encoding limit",
        ),
        (
            MqttCodecError::PacketIdentifierRequired,
            "MQTT QoS 1 or QoS 2 requires a non-zero packet identifier",
        ),
        (
            MqttCodecError::ReliableQosRequired,
            "MQTT reliable publishing requires QoS 1 or QoS 2",
        ),
        (
            MqttCodecError::DuplicateRequiresReliableQos,
            "MQTT DUP requires QoS 1 or QoS 2 in this bounded encoder",
        ),
        (
            MqttCodecError::AllocationFailed,
            "MQTT PUBLISH packet buffer could not be allocated",
        ),
    ] {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn accessors_return_the_validated_topic_and_payload() {
    let publish = MqttPublish::new("sensors/1", b"21.5".to_vec()).expect("valid topic");
    assert_eq!(publish.topic(), "sensors/1");
    assert_eq!(publish.payload(), b"21.5");
}

#[test]
fn packet_ceiling_admits_exactly_one_mebibyte() {
    // 1 fixed-header byte + 3 remaining-length bytes + 2 topic-length
    // bytes + "a" + 1 property-length byte = 8 bytes around the payload.
    let packet = MqttPublish::new("a", vec![0; MAX_MQTT_PACKET_BYTES - 8])
        .and_then(|publish| publish.encode())
        .expect("a packet at the ceiling should encode");
    assert_eq!(packet.len(), 1_048_576);
    assert_eq!(
        MqttPublish::new("a", vec![0; MAX_MQTT_PACKET_BYTES - 7])
            .and_then(|publish| publish.encode()),
        Err(MqttCodecError::PacketTooLarge)
    );
}

#[test]
fn topic_length_follows_the_two_byte_utf8_prefix() {
    assert!(MqttPublish::new("a".repeat(65_535), Vec::<u8>::new()).is_ok());
    assert_eq!(
        MqttPublish::new("a".repeat(65_536), Vec::<u8>::new()),
        Err(MqttCodecError::TopicTooLong)
    );
}

#[test]
fn rejects_every_disallowed_topic_character_class() {
    for rejected in [
        '+',
        '#',
        '\u{0}',
        '\u{1}',
        '\u{1f}',
        '\u{7f}',
        '\u{85}',
        '\u{9f}',
        '\u{fdd0}',
        '\u{fdef}',
        '\u{fffe}',
        '\u{ffff}',
        '\u{1fffe}',
        '\u{10ffff}',
    ] {
        assert!(
            is_disallowed_topic_character(rejected),
            "U+{:04X} should be rejected",
            rejected as u32
        );
    }
    for accepted in [
        ' ',
        '~',
        '\u{a0}',
        '\u{fdcf}',
        '\u{fdf0}',
        '\u{fffd}',
        '\u{10000}',
        '\u{1fffd}',
    ] {
        assert!(
            !is_disallowed_topic_character(accepted),
            "U+{:04X} should be accepted",
            accepted as u32
        );
    }
}

fn varint(value: usize) -> Result<Vec<u8>, MqttCodecError> {
    variable_byte_integer(value).map(|(bytes, len)| bytes[..len].to_vec())
}

#[test]
fn variable_byte_integer_covers_the_mqtt_range() {
    assert_eq!(varint(0), Ok(vec![0x00]));
    assert_eq!(varint(127), Ok(vec![0x7f]));
    assert_eq!(varint(128), Ok(vec![0x80, 0x01]));
    assert_eq!(varint(16_383), Ok(vec![0xff, 0x7f]));
    assert_eq!(varint(16_384), Ok(vec![0x80, 0x80, 0x01]));
    assert_eq!(varint(2_097_151), Ok(vec![0xff, 0xff, 0x7f]));
    assert_eq!(varint(2_097_152), Ok(vec![0x80, 0x80, 0x80, 0x01]));
    assert_eq!(
        varint(MQTT_MAX_VARIABLE_BYTE_INTEGER),
        Ok(vec![0xff, 0xff, 0xff, 0x7f])
    );
    assert_eq!(
        varint(MQTT_MAX_VARIABLE_BYTE_INTEGER + 1),
        Err(MqttCodecError::PacketTooLarge)
    );
}

#[test]
fn encoded_packet_buffer_has_exactly_its_length() {
    for payload_len in [0, 1, 127, 128, 16_383, 16_384] {
        let packet = MqttPublish::reliable("a/b", vec![1; payload_len], MqttQos::AtLeastOnce, 9)
            .and_then(|publish| publish.encode())
            .expect("bounded packet should encode");
        assert_eq!(packet.capacity(), packet.len(), "payload {payload_len}");
    }
}

#[cfg(feature = "experimental-simulators")]
#[test]
fn simulated_formatter_only_formats_a_value() {
    let telemetry = SensorTelemetry::new("fixture", "temperature", 24.5, 1);
    assert_eq!(
        SimulatedMqttPayloadFormatter::format_value(&telemetry),
        "24.5"
    );
}
