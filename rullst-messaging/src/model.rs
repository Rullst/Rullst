//! Broker-neutral requests, receipts, deliveries, and administration views.

use crate::validation::{MAX_BATCH_SIZE, MAX_LEASE_MILLIS, MIN_LEASE_MILLIS};
use crate::{
    AckToken, ConsumerGroup, ConsumerName, ContentType, EventKind, IdempotencyKey, MessageHeaders,
    MessageId, MessagingError, Namespace, Result, StartPosition, TopicName,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::time::Duration;

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

/// Registers one consumer-group view of a topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionRequest {
    topic: TopicName,
    group: ConsumerGroup,
    start: StartPosition,
}

impl SubscriptionRequest {
    /// Creates a bounded subscription request.
    pub fn try_new(
        topic: impl Into<String>,
        group: impl Into<String>,
        start: StartPosition,
    ) -> Result<Self> {
        Ok(Self {
            topic: TopicName::try_new(topic)?,
            group: ConsumerGroup::try_new(group)?,
            start,
        })
    }

    /// Returns the topic.
    pub fn topic(&self) -> &TopicName {
        &self.topic
    }

    /// Returns the group.
    pub fn group(&self) -> &ConsumerGroup {
        &self.group
    }

    /// Returns the position used only if this call first creates the group.
    pub fn start(&self) -> StartPosition {
        self.start
    }
}

/// Result of idempotent subscription registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionReceipt {
    created: bool,
    pending_messages: usize,
}

impl SubscriptionReceipt {
    pub(crate) fn new(created: bool, pending_messages: usize) -> Self {
        Self {
            created,
            pending_messages,
        }
    }

    /// Returns whether this call created the group.
    pub fn was_created(&self) -> bool {
        self.created
    }

    /// Returns messages initially visible to the group.
    pub fn pending_messages(&self) -> usize {
        self.pending_messages
    }
}

/// Bounded pull request for one registered consumer group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveRequest {
    topic: TopicName,
    group: ConsumerGroup,
    consumer: ConsumerName,
    max_messages: usize,
    lease_millis: u64,
}

impl ReceiveRequest {
    /// Creates a request for 1–100 messages and a lease between one second and one hour.
    pub fn try_new(
        topic: impl Into<String>,
        group: impl Into<String>,
        consumer: impl Into<String>,
        max_messages: usize,
        lease: Duration,
    ) -> Result<Self> {
        if !(1..=MAX_BATCH_SIZE).contains(&max_messages) {
            return Err(MessagingError::Invalid {
                field: "receive batch size",
                reason: "must be between 1 and 100",
            });
        }
        let lease_millis =
            u64::try_from(lease.as_millis()).map_err(|_| MessagingError::Invalid {
                field: "message lease",
                reason: "duration is outside the supported range",
            })?;
        if !(MIN_LEASE_MILLIS..=MAX_LEASE_MILLIS).contains(&lease_millis) {
            return Err(MessagingError::Invalid {
                field: "message lease",
                reason: "must be between one second and one hour",
            });
        }
        Ok(Self {
            topic: TopicName::try_new(topic)?,
            group: ConsumerGroup::try_new(group)?,
            consumer: ConsumerName::try_new(consumer)?,
            max_messages,
            lease_millis,
        })
    }

    /// Returns the topic.
    pub fn topic(&self) -> &TopicName {
        &self.topic
    }

    /// Returns the group.
    pub fn group(&self) -> &ConsumerGroup {
        &self.group
    }

    /// Returns the consumer identity.
    pub fn consumer(&self) -> &ConsumerName {
        &self.consumer
    }

    /// Returns the requested batch bound.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    pub(crate) fn lease_millis(&self) -> u64 {
        self.lease_millis
    }
}

/// One leased at-least-once delivery.
#[derive(Clone, PartialEq, Eq)]
pub struct Delivery {
    envelope: MessageEnvelope,
    group: ConsumerGroup,
    consumer: ConsumerName,
    attempt: u32,
    lease_expires_at_ms: i64,
    ack_token: AckToken,
}

impl Delivery {
    pub(crate) fn new(
        envelope: MessageEnvelope,
        group: ConsumerGroup,
        consumer: ConsumerName,
        attempt: u32,
        lease_expires_at_ms: i64,
        ack_token: AckToken,
    ) -> Self {
        Self {
            envelope,
            group,
            consumer,
            attempt,
            lease_expires_at_ms,
            ack_token,
        }
    }

    /// Returns the immutable envelope.
    pub fn envelope(&self) -> &MessageEnvelope {
        &self.envelope
    }

    /// Returns the consumer group.
    pub fn group(&self) -> &ConsumerGroup {
        &self.group
    }

    /// Returns the consumer that owns this lease.
    pub fn consumer(&self) -> &ConsumerName {
        &self.consumer
    }

    /// Returns the one-based delivery attempt.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Returns the absolute lease expiry.
    pub fn lease_expires_at_ms(&self) -> i64 {
        self.lease_expires_at_ms
    }

    /// Returns the redacted acknowledgement capability.
    pub fn ack_token(&self) -> &AckToken {
        &self.ack_token
    }
}

impl fmt::Debug for Delivery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Delivery")
            .field("envelope", &self.envelope)
            .field("group", &self.group)
            .field("consumer", &self.consumer)
            .field("attempt", &self.attempt)
            .field("lease_expires_at_ms", &self.lease_expires_at_ms)
            .field("ack_token", &self.ack_token)
            .finish()
    }
}

/// Result of returning a delivery for retry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryDisposition {
    /// The same group can receive it after the indicated timestamp.
    Scheduled { available_at_ms: i64 },
    /// The configured attempt ceiling moved it to the dead-letter view.
    DeadLettered,
}
