//! Consumer-group subscriptions, bounded receive requests, leased deliveries
//! and retry outcomes.

use super::MessageEnvelope;
use crate::validation::{MAX_BATCH_SIZE, MAX_LEASE_MILLIS, MIN_LEASE_MILLIS};
use crate::{
    AckToken, ConsumerGroup, ConsumerName, MessagingError, Result, StartPosition, TopicName,
};
use std::fmt;
use std::time::Duration;

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
