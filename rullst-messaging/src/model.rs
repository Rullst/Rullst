//! Broker-neutral requests, receipts, deliveries, and administration views.

use crate::{
    ContentType, EventKind, IdempotencyKey, MessageHeaders, MessageId, MessagingError, Namespace,
    Result, TopicName,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;

mod consume;
pub use consume::{
    Delivery, ReceiveRequest, RetryDisposition, SubscriptionReceipt, SubscriptionRequest,
};

/// Immutable message proposed to a broker.
#[derive(Clone, PartialEq, Eq)]
pub struct PublishRequest {
    topic: TopicName,
    event_kind: EventKind,
    idempotency_key: IdempotencyKey,
    content_type: ContentType,
    headers: MessageHeaders,
    payload: Vec<u8>,
}

impl PublishRequest {
    /// Creates a binary message request with an application-owned idempotency key.
    pub fn try_new(
        topic: impl Into<String>,
        event_kind: impl Into<String>,
        idempotency_key: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self> {
        Ok(Self {
            topic: TopicName::try_new(topic)?,
            event_kind: EventKind::try_new(event_kind)?,
            idempotency_key: IdempotencyKey::try_new(idempotency_key)?,
            content_type: ContentType::binary(),
            headers: MessageHeaders::new(),
            payload: payload.into(),
        })
    }

    /// Replaces the default `application/octet-stream` MIME type.
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Result<Self> {
        self.content_type = ContentType::try_new(content_type)?;
        Ok(self)
    }

    /// Adds one unique, bounded metadata header.
    pub fn with_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self> {
        self.headers.try_insert(name, value)?;
        Ok(self)
    }

    /// Adds one validated W3C trace context without propagating baggage.
    pub fn with_trace_context(mut self, context: &crate::TraceContext) -> Result<Self> {
        context.insert_into(&mut self.headers)?;
        Ok(self)
    }

    /// Returns the destination topic.
    pub fn topic(&self) -> &TopicName {
        &self.topic
    }

    /// Returns the event kind.
    pub fn event_kind(&self) -> &EventKind {
        &self.event_kind
    }

    /// Returns the MIME type.
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// Returns the bounded metadata collection.
    pub fn headers(&self) -> &MessageHeaders {
        &self.headers
    }

    /// Returns the opaque payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub(crate) fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    pub(crate) fn validate_payload(&self, max_payload_bytes: usize) -> Result<()> {
        if self.payload.len() > max_payload_bytes {
            return Err(MessagingError::CapacityExceeded {
                resource: "message payload bytes",
                limit: max_payload_bytes,
            });
        }
        Ok(())
    }

    pub(crate) fn fingerprint(&self) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        hasher.update(b"rullst.messaging.publish.v1\0");
        hash_field(&mut hasher, self.topic.as_str().as_bytes())?;
        hash_field(&mut hasher, self.event_kind.as_str().as_bytes())?;
        hash_field(&mut hasher, self.content_type.as_str().as_bytes())?;
        for (name, value) in self.headers.iter() {
            hash_field(&mut hasher, name.as_bytes())?;
            hash_field(&mut hasher, value.as_bytes())?;
        }
        hash_field(&mut hasher, &self.payload)?;
        Ok(hasher.finalize().into())
    }
}

impl fmt::Debug for PublishRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublishRequest")
            .field("topic", &self.topic)
            .field("event_kind", &self.event_kind)
            .field("idempotency_key", &self.idempotency_key)
            .field("content_type", &self.content_type)
            .field("headers", &self.headers)
            .field("payload_bytes", &self.payload.len())
            .finish()
    }
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) -> Result<()> {
    let length = u64::try_from(bytes.len()).map_err(|_| MessagingError::InternalState {
        context: "publish fingerprint length",
    })?;
    hasher.update(length.to_be_bytes());
    hasher.update(bytes);
    Ok(())
}

/// Versioned immutable message delivered to a consumer.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct MessageEnvelope {
    schema: &'static str,
    id: MessageId,
    namespace: Namespace,
    topic: TopicName,
    event_kind: EventKind,
    content_type: ContentType,
    headers: MessageHeaders,
    payload: Vec<u8>,
    published_at_ms: i64,
}

pub(crate) struct StoredEnvelopeParts {
    pub(crate) id: MessageId,
    pub(crate) namespace: Namespace,
    pub(crate) topic: TopicName,
    pub(crate) event_kind: EventKind,
    pub(crate) content_type: ContentType,
    pub(crate) headers: MessageHeaders,
    pub(crate) payload: Vec<u8>,
    pub(crate) published_at_ms: i64,
}

impl MessageEnvelope {
    /// Stable envelope marker.
    pub const SCHEMA: &'static str = "rullst.messaging.v1";

    pub(crate) fn from_request(
        request: &PublishRequest,
        namespace: Namespace,
        id: MessageId,
        published_at_ms: i64,
    ) -> Self {
        Self {
            schema: Self::SCHEMA,
            id,
            namespace,
            topic: request.topic.clone(),
            event_kind: request.event_kind.clone(),
            content_type: request.content_type.clone(),
            headers: request.headers.clone(),
            payload: request.payload.clone(),
            published_at_ms,
        }
    }

    pub(crate) fn from_stored(parts: StoredEnvelopeParts) -> Self {
        Self {
            schema: Self::SCHEMA,
            id: parts.id,
            namespace: parts.namespace,
            topic: parts.topic,
            event_kind: parts.event_kind,
            content_type: parts.content_type,
            headers: parts.headers,
            payload: parts.payload,
            published_at_ms: parts.published_at_ms,
        }
    }

    /// Returns the stable schema marker.
    pub fn schema(&self) -> &'static str {
        self.schema
    }

    /// Returns the broker-assigned ID.
    pub fn id(&self) -> &MessageId {
        &self.id
    }

    /// Returns the immutable broker namespace.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// Returns the topic.
    pub fn topic(&self) -> &TopicName {
        &self.topic
    }

    /// Returns the event kind.
    pub fn event_kind(&self) -> &EventKind {
        &self.event_kind
    }

    /// Returns the content type.
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// Returns the bounded metadata.
    pub fn headers(&self) -> &MessageHeaders {
        &self.headers
    }

    /// Returns the validated W3C trace context carried by allowlisted headers.
    pub fn trace_context(&self) -> Result<Option<crate::TraceContext>> {
        crate::TraceContext::from_headers(&self.headers)
    }

    /// Returns the opaque payload.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Returns broker publication time.
    pub fn published_at_ms(&self) -> i64 {
        self.published_at_ms
    }
}

impl fmt::Debug for MessageEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageEnvelope")
            .field("schema", &self.schema)
            .field("id", &self.id)
            .field("namespace", &self.namespace)
            .field("topic", &self.topic)
            .field("event_kind", &self.event_kind)
            .field("content_type", &self.content_type)
            .field("headers", &self.headers)
            .field("payload_bytes", &self.payload.len())
            .field("published_at_ms", &self.published_at_ms)
            .finish()
    }
}

/// Result of idempotent publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReceipt {
    id: MessageId,
    duplicate: bool,
    published_at_ms: i64,
}

impl PublishReceipt {
    pub(crate) fn new(id: MessageId, duplicate: bool, published_at_ms: i64) -> Self {
        Self {
            id,
            duplicate,
            published_at_ms,
        }
    }

    /// Returns the stable ID, including on an exact replay.
    pub fn id(&self) -> &MessageId {
        &self.id
    }

    /// Returns whether this call replayed an existing exact publication.
    pub fn is_duplicate(&self) -> bool {
        self.duplicate
    }

    /// Returns the original publication time.
    pub fn published_at_ms(&self) -> i64 {
        self.published_at_ms
    }

    pub(crate) fn as_duplicate(&self) -> Self {
        Self {
            id: self.id.clone(),
            duplicate: true,
            published_at_ms: self.published_at_ms,
        }
    }
}
