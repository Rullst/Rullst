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
