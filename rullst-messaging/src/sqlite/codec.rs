use super::storage::{MessageBinding, StorageProfile};
use crate::model::StoredEnvelopeParts;
use crate::{
    ContentType, EventKind, MessageEnvelope, MessageId, MessagingError, Namespace, Result,
    TopicName,
};

pub(super) type EnvelopeRow = (String, String, String, String, Vec<u8>, i64);

pub(super) fn decode_envelope(
    storage: &StorageProfile,
    namespace: &Namespace,
    topic: &str,
    sequence: i64,
    row: EnvelopeRow,
    max_payload_bytes: usize,
) -> Result<MessageEnvelope> {
    let (message_id, event_kind, content_type, headers_json, payload, published_at_ms) = row;
    if sequence <= 0 || published_at_ms < 0 {
        return Err(MessagingError::CorruptStorage {
            context: "message bounds",
        });
    }
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
    let id = MessageId::from_stored(message_id)?;
    let binding = MessageBinding::message(
        namespace,
        topic.as_str(),
        sequence,
        id.as_str(),
        event_kind.as_str(),
        content_type.as_str(),
        published_at_ms,
    );
    let (headers, payload) =
        storage.decode_message(binding, headers_json, payload, max_payload_bytes)?;
    Ok(MessageEnvelope::from_stored(StoredEnvelopeParts {
        id,
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
