//! Bounded MQTT 5 PUBLISH encoding without network transport.
//!
//! The encoder emits a single MQTT 5 PUBLISH packet with an empty property
//! section. It does not open a connection, negotiate broker limits, retry,
//! subscribe, or implement the acknowledgement state machines.

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{fmt, num::NonZeroU16};

#[cfg(feature = "experimental-simulators")]
use crate::SensorTelemetry;
#[cfg(feature = "experimental-simulators")]
use alloc::string::ToString;

/// Local allocation ceiling for one encoded MQTT packet.
pub const MAX_MQTT_PACKET_BYTES: usize = 1024 * 1024;

const MQTT_MAX_VARIABLE_BYTE_INTEGER: usize = 268_435_455;

/// MQTT quality-of-service bits supported by the PUBLISH encoder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttQos {
    /// Best-effort delivery; no packet identifier is encoded.
    AtMostOnce,
    /// Broker acknowledgement is required; the caller owns PUBACK handling.
    AtLeastOnce,
    /// Two-phase acknowledgement is required; the caller owns the state machine.
    ExactlyOnce,
}

impl MqttQos {
    const fn bits(self) -> u8 {
        match self {
            Self::AtMostOnce => 0,
            Self::AtLeastOnce => 1,
            Self::ExactlyOnce => 2,
        }
    }
}

/// Fail-closed MQTT packet construction errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MqttCodecError {
    /// PUBLISH topic names cannot be empty in this no-alias encoder.
    EmptyTopic,
    /// Topic contains a wildcard, null, control, or noncharacter code point.
    InvalidTopic,
    /// MQTT's two-byte UTF-8 length cannot represent the topic.
    TopicTooLong,
    /// Payload exceeds Rullst's bounded local allocation policy.
    PayloadTooLarge,
    /// Encoded packet exceeds Rullst's ceiling or MQTT's remaining-length limit.
    PacketTooLarge,
    /// QoS 1 and QoS 2 require a non-zero packet identifier.
    PacketIdentifierRequired,
    /// The reliable constructor accepts only QoS 1 or QoS 2.
    ReliableQosRequired,
    /// DUP is rejected for the QoS 0 helper because no acknowledgement state exists.
    DuplicateRequiresReliableQos,
}

impl fmt::Display for MqttCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyTopic => "MQTT PUBLISH topic cannot be empty",
            Self::InvalidTopic => "MQTT PUBLISH topic contains a disallowed character",
            Self::TopicTooLong => "MQTT PUBLISH topic exceeds 65535 UTF-8 bytes",
            Self::PayloadTooLarge => "MQTT PUBLISH payload exceeds the local packet ceiling",
            Self::PacketTooLarge => "MQTT PUBLISH packet exceeds an encoding limit",
            Self::PacketIdentifierRequired => {
                "MQTT QoS 1 or QoS 2 requires a non-zero packet identifier"
            }
            Self::ReliableQosRequired => "MQTT reliable publishing requires QoS 1 or QoS 2",
            Self::DuplicateRequiresReliableQos => {
                "MQTT DUP requires QoS 1 or QoS 2 in this bounded encoder"
            }
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MqttCodecError {}

/// Owned, bounded MQTT 5 PUBLISH packet builder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MqttPublish {
    topic: String,
    payload: Vec<u8>,
    qos: MqttQos,
    packet_identifier: Option<NonZeroU16>,
    retain: bool,
    duplicate: bool,
}

impl MqttPublish {
    /// Creates a QoS 0 PUBLISH packet with no packet identifier.
    pub fn new(
        topic: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, MqttCodecError> {
        Self::build(topic.into(), payload.into(), MqttQos::AtMostOnce, None)
    }

    /// Creates a QoS 1 or QoS 2 packet with a required non-zero identifier.
    pub fn reliable(
        topic: impl Into<String>,
        payload: impl Into<Vec<u8>>,
        qos: MqttQos,
        packet_identifier: u16,
    ) -> Result<Self, MqttCodecError> {
        if qos == MqttQos::AtMostOnce {
            return Err(MqttCodecError::ReliableQosRequired);
        }
        let packet_identifier =
            NonZeroU16::new(packet_identifier).ok_or(MqttCodecError::PacketIdentifierRequired)?;
        Self::build(topic.into(), payload.into(), qos, Some(packet_identifier))
    }

    fn build(
        topic: String,
        payload: Vec<u8>,
        qos: MqttQos,
        packet_identifier: Option<NonZeroU16>,
    ) -> Result<Self, MqttCodecError> {
        validate_topic(&topic)?;
        if payload.len() > MAX_MQTT_PACKET_BYTES {
            return Err(MqttCodecError::PayloadTooLarge);
        }
        if qos != MqttQos::AtMostOnce && packet_identifier.is_none() {
            return Err(MqttCodecError::PacketIdentifierRequired);
        }
        if qos == MqttQos::AtMostOnce && packet_identifier.is_some() {
            return Err(MqttCodecError::ReliableQosRequired);
        }
        Ok(Self {
            topic,
            payload,
            qos,
            packet_identifier,
            retain: false,
            duplicate: false,
        })
    }

    /// Sets the MQTT RETAIN flag.
    #[must_use]
    pub const fn retained(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    /// Sets DUP only for QoS 1/2 packets whose retry state is owned by the caller.
    pub fn duplicate(mut self, duplicate: bool) -> Result<Self, MqttCodecError> {
        if duplicate && self.qos == MqttQos::AtMostOnce {
            return Err(MqttCodecError::DuplicateRequiresReliableQos);
        }
        self.duplicate = duplicate;
        Ok(self)
    }

    /// Returns the validated topic name.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the application payload without copying it.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes one MQTT 5 PUBLISH control packet with zero properties.
    pub fn encode(&self) -> Result<Vec<u8>, MqttCodecError> {
        let identifier_bytes = usize::from(self.packet_identifier.is_some()) * 2;
        let remaining_length = 2usize
            .checked_add(self.topic.len())
            .and_then(|length| length.checked_add(identifier_bytes))
            .and_then(|length| length.checked_add(1))
            .and_then(|length| length.checked_add(self.payload.len()))
            .ok_or(MqttCodecError::PacketTooLarge)?;

        if remaining_length > MQTT_MAX_VARIABLE_BYTE_INTEGER {
            return Err(MqttCodecError::PacketTooLarge);
        }
        let remaining_bytes = variable_byte_integer(remaining_length)?;
        let packet_length = 1usize
            .checked_add(remaining_bytes.len())
            .and_then(|length| length.checked_add(remaining_length))
            .ok_or(MqttCodecError::PacketTooLarge)?;
        if packet_length > MAX_MQTT_PACKET_BYTES {
            return Err(MqttCodecError::PacketTooLarge);
        }

        let mut packet = Vec::with_capacity(packet_length);
        let mut fixed_header = 0x30 | (self.qos.bits() << 1);
        if self.duplicate {
            fixed_header |= 0x08;
        }
        if self.retain {
            fixed_header |= 0x01;
        }
        packet.push(fixed_header);
        packet.extend_from_slice(&remaining_bytes);
        packet.extend_from_slice(&(self.topic.len() as u16).to_be_bytes());
        packet.extend_from_slice(self.topic.as_bytes());
        if let Some(identifier) = self.packet_identifier {
            packet.extend_from_slice(&identifier.get().to_be_bytes());
        }
        packet.push(0); // MQTT 5 property length: no properties.
        packet.extend_from_slice(&self.payload);
        Ok(packet)
    }
}

fn validate_topic(topic: &str) -> Result<(), MqttCodecError> {
    if topic.is_empty() {
        return Err(MqttCodecError::EmptyTopic);
    }
    if topic.len() > u16::MAX as usize {
        return Err(MqttCodecError::TopicTooLong);
    }
    if topic.chars().any(is_disallowed_topic_character) {
        return Err(MqttCodecError::InvalidTopic);
    }
    Ok(())
}

fn is_disallowed_topic_character(character: char) -> bool {
    let code = character as u32;
    character == '+'
        || character == '#'
        || code == 0
        || (code <= 0x1f)
        || (0x7f..=0x9f).contains(&code)
        || (0xfdd0..=0xfdef).contains(&code)
        || code & 0xffff == 0xfffe
        || code & 0xffff == 0xffff
}

fn variable_byte_integer(mut value: usize) -> Result<Vec<u8>, MqttCodecError> {
    if value > MQTT_MAX_VARIABLE_BYTE_INTEGER {
        return Err(MqttCodecError::PacketTooLarge);
    }
    let mut encoded = Vec::with_capacity(4);
    loop {
        let mut byte = (value % 128) as u8;
        value /= 128;
        if value > 0 {
            byte |= 0x80;
        }
        encoded.push(byte);
        if value == 0 {
            return Ok(encoded);
        }
    }
}

/// Explicitly simulated telemetry value formatter; not an MQTT client.
#[cfg(feature = "experimental-simulators")]
pub struct SimulatedMqttPayloadFormatter;

#[cfg(feature = "experimental-simulators")]
impl SimulatedMqttPayloadFormatter {
    /// Formats only the numeric reading for deterministic fixture assertions.
    #[must_use]
    pub fn format_value(telemetry: &SensorTelemetry) -> String {
        telemetry.value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            MqttPublish::new("a", vec![0; MAX_MQTT_PACKET_BYTES])
                .and_then(|publish| publish.encode()),
            Err(MqttCodecError::PacketTooLarge)
        );
        assert_eq!(
            MqttPublish::new("a", Vec::<u8>::new()).and_then(|publish| publish.duplicate(true)),
            Err(MqttCodecError::DuplicateRequiresReliableQos)
        );
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
}
