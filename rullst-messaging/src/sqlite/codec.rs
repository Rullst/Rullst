use crate::model::StoredEnvelopeParts;
use crate::{
    ContentType, EventKind, MessageEnvelope, MessageHeaders, MessageId, MessagingError, Namespace,
    Result, TopicName,
};
use std::collections::BTreeMap;

pub(super) type EnvelopeRow = (String, String, String, String, Vec<u8>, i64);

pub(super) fn encode_headers(headers: &MessageHeaders) -> Result<String> {
    serde_json::to_string(headers).map_err(|_| MessagingError::InternalState {
        context: "durable header serialization",
    })
}

pub(super) fn decode_envelope(
    namespace: &Namespace,
    topic: &str,
    row: EnvelopeRow,
    max_payload_bytes: usize,
) -> Result<MessageEnvelope> {
    let (message_id, event_kind, content_type, headers_json, payload, published_at_ms) = row;
    if published_at_ms < 0 || payload.len() > max_payload_bytes {
        return Err(MessagingError::CorruptStorage {
            context: "message bounds",
        });
    }
    let header_map: BTreeMap<String, String> =
        serde_json::from_str(&headers_json).map_err(|_| MessagingError::CorruptStorage {
            context: "message header encoding",
        })?;
    let headers = MessageHeaders::from_stored(header_map)?;
    let topic = TopicName::try_new(topic).map_err(|_| MessagingError::CorruptStorage {
        context: "message topic",
    })?;
    let event_kind =
        EventKind::try_new(event_kind).map_err(|_| MessagingError::CorruptStorage {
            context: "message event kind",
        })?;
    let content_type =
        ContentType::try_new(content_type).map_err(|_| MessagingError::CorruptStorage {
            context: "message content type",
        })?;
    Ok(MessageEnvelope::from_stored(StoredEnvelopeParts {
        id: MessageId::from_stored(message_id)?,
        namespace: namespace.clone(),
        topic,
        event_kind,
        content_type,
        headers,
        payload,
        published_at_ms,
    }))
}

pub(super) fn fingerprint(bytes: Vec<u8>) -> Result<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| MessagingError::CorruptStorage {
            context: "publication fingerprint",
        })
}

pub(super) fn attempt(value: i64) -> Result<u32> {
    u32::try_from(value).map_err(|_| MessagingError::CorruptStorage {
        context: "delivery attempt",
    })
}
