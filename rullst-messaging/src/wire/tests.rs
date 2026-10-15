use super::*;
use crate::TraceContext;
use crate::model::StoredEnvelopeParts;
use sha2::{Digest, Sha256};

const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

fn fixture() -> (BrokerConfig, MessageEnvelope) {
    let config = BrokerConfig::try_new("wire").expect("config");
    let mut headers = MessageHeaders::new();
    headers
        .try_insert("tenant-hint", "opaque")
        .expect("tenant header");
    let trace = TraceContext::try_with_state(TRACEPARENT, "vendor=value").expect("trace");
    trace.insert_into(&mut headers).expect("trace headers");
    let envelope = MessageEnvelope::from_stored(StoredEnvelopeParts {
        id: MessageId::from_stored("msg_0123456789abcdef0123456789abcdef".to_string())
            .expect("message id"),
        namespace: Namespace::try_new("wire").expect("namespace"),
        topic: TopicName::try_new("events").expect("topic"),
        event_kind: EventKind::try_new("event.created").expect("event kind"),
        content_type: ContentType::try_new("application/json").expect("content type"),
        headers,
        payload: br#"{"id":42}"#.to_vec(),
        published_at_ms: 1_735_689_600_123,
    });
    (config, envelope)
}

#[test]
fn canonical_frame_round_trips_every_bounded_envelope_field() {
    let (config, envelope) = fixture();
    let encoded = WireEnvelopeCodec::encode(&envelope, &config).expect("encode");
    assert_eq!(encoded.get(..8), Some(b"RLMWIRE\x01".as_slice()));
    assert_eq!(
        WireEnvelopeCodec::decode(&encoded, &config).expect("decode"),
        envelope
    );
    assert_eq!(
        WireEnvelopeCodec::encode(
            &WireEnvelopeCodec::decode(&encoded, &config).expect("decode twice"),
            &config,
        )
        .expect("canonical re-encode"),
        encoded
    );
}

#[test]
fn version_truncation_namespace_and_trailing_bytes_fail_closed() {
    // TM-MESSAGING-04: untrusted wire frames fail closed before delivery.
    let (config, envelope) = fixture();
    let encoded = WireEnvelopeCodec::encode(&envelope, &config).expect("encode");
    for length in 0..encoded.len() {
        assert!(WireEnvelopeCodec::decode(&encoded[..length], &config).is_err());
    }
    let mut wrong_magic = encoded.clone();
    wrong_magic[0] ^= 1;
    assert_eq!(
        WireEnvelopeCodec::decode(&wrong_magic, &config),
        Err(MessagingError::InvalidWireEnvelope)
    );
    let mut future = encoded.clone();
    future[7] = 2;
    assert_eq!(
        WireEnvelopeCodec::decode(&future, &config),
        Err(MessagingError::UnsupportedWireVersion)
    );
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(
        WireEnvelopeCodec::decode(&trailing, &config),
        Err(MessagingError::InvalidWireEnvelope)
    );
    let other = BrokerConfig::try_new("other").expect("other config");
    assert_eq!(
        WireEnvelopeCodec::decode(&encoded, &other),
        Err(MessagingError::InvalidWireEnvelope)
    );
}

#[test]
fn compatibility_digest_and_secret_free_errors_are_stable() {
    let (config, envelope) = fixture();
    let encoded = WireEnvelopeCodec::encode(&envelope, &config).expect("encode");
    let digest = Sha256::digest(&encoded);
    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        actual,
        "164123c0133fa2521071c7dc0ac92afd38bcfddd95d66afb37ab8d72a8b03803"
    );
    let error =
        WireEnvelopeCodec::decode(b"private-payload", &config).expect_err("malformed frame");
    assert!(!error.to_string().contains("private-payload"));
}

/// Every field at its bound: 32 headers of 256 bytes and a 16-byte payload.
fn maximal(published_at_ms: i64, payload_bytes: usize) -> MessageEnvelope {
    let mut headers = MessageHeaders::new();
    for index in 0..32 {
        headers
            .try_insert(format!("h{index:02}"), "v".repeat(253))
            .expect("bounded header");
    }
    MessageEnvelope::from_stored(StoredEnvelopeParts {
        id: MessageId::from_stored("msg_0123456789abcdef0123456789abcdef".to_string())
            .expect("message id"),
        namespace: Namespace::try_new("n".repeat(128)).expect("namespace"),
        topic: TopicName::try_new("t".repeat(255)).expect("topic"),
        event_kind: EventKind::try_new("e".repeat(128)).expect("event kind"),
        content_type: ContentType::try_new(format!("a/{}", "b".repeat(125))).expect("type"),
        headers,
        payload: vec![7; payload_bytes],
        published_at_ms,
    })
}

fn sixteen_byte_config(namespace: String) -> BrokerConfig {
    BrokerConfig::try_new(namespace)
        .and_then(|config| config.with_limits(1, 1, 1, 16))
        .expect("config")
}

#[test]
fn the_largest_envelope_fills_the_frame_bound_exactly() {
    let config = sixteen_byte_config("n".repeat(128));
    let envelope = maximal(0, 16);
    let frame = WireEnvelopeCodec::encode(&envelope, &config).expect("encode");
    assert_eq!(frame.len(), 9_042);
    assert_eq!(WireEnvelopeCodec::decode(&frame, &config), Ok(envelope));

    // One byte more than any valid frame is refused before its version is read.
    let mut oversized = b"RLMWIRE\x02".to_vec();
    oversized.resize(9_043, 0);
    assert_eq!(
        WireEnvelopeCodec::decode(&oversized, &config),
        Err(MessagingError::InvalidWireEnvelope)
    );
    oversized.truncate(9_042);
    assert_eq!(
        WireEnvelopeCodec::decode(&oversized, &config),
        Err(MessagingError::UnsupportedWireVersion)
    );
}

#[test]
fn encoding_requires_the_namespace_a_valid_time_and_a_bounded_payload() {
    let config = sixteen_byte_config("n".repeat(128));
    let other = sixteen_byte_config("other".to_string());
    assert_eq!(
        WireEnvelopeCodec::encode(&maximal(0, 16), &other),
        Err(MessagingError::InvalidWireEnvelope)
    );
    assert_eq!(
        WireEnvelopeCodec::encode(&maximal(-1, 16), &config),
        Err(MessagingError::InvalidWireEnvelope)
    );
    assert_eq!(
        WireEnvelopeCodec::encode(&maximal(0, 17), &config),
        Err(MessagingError::InvalidWireEnvelope)
    );
}
