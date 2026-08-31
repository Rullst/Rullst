//! Bounded dead-letter inspection and explicit terminal-message cleanup.

use crate::validation::{MAX_DEAD_LETTER_QUERY, MAX_PURGE_BATCH};
use crate::{ConsumerGroup, FailureCode, MessageEnvelope, MessagingError, Result, TopicName};
use std::fmt;

/// Bounded dead-letter list request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetterQuery {
    topic: TopicName,
    group: ConsumerGroup,
    limit: usize,
}

impl DeadLetterQuery {
    /// Creates a query returning at most 100 records.
    pub fn try_new(
        topic: impl Into<String>,
        group: impl Into<String>,
        limit: usize,
    ) -> Result<Self> {
        if !(1..=MAX_DEAD_LETTER_QUERY).contains(&limit) {
            return Err(MessagingError::Invalid {
                field: "dead-letter query limit",
                reason: "must be between 1 and 100",
            });
        }
        Ok(Self {
            topic: TopicName::try_new(topic)?,
            group: ConsumerGroup::try_new(group)?,
            limit,
        })
    }

    pub(crate) fn topic(&self) -> &TopicName {
        &self.topic
    }

    pub(crate) fn group(&self) -> &ConsumerGroup {
        &self.group
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }
}

/// A terminal failed delivery for one consumer group.
#[derive(Clone, PartialEq, Eq)]
pub struct DeadLetter {
    envelope: MessageEnvelope,
    group: ConsumerGroup,
    attempts: u32,
    failure_code: FailureCode,
    dead_lettered_at_ms: i64,
}

impl DeadLetter {
    pub(crate) fn new(
        envelope: MessageEnvelope,
        group: ConsumerGroup,
        attempts: u32,
        failure_code: FailureCode,
        dead_lettered_at_ms: i64,
    ) -> Self {
        Self {
            envelope,
            group,
            attempts,
            failure_code,
            dead_lettered_at_ms,
        }
    }

    /// Returns the immutable message.
    pub fn envelope(&self) -> &MessageEnvelope {
        &self.envelope
    }

    /// Returns the affected group.
    pub fn group(&self) -> &ConsumerGroup {
        &self.group
    }

    /// Returns the number of deliveries attempted.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Returns the low-cardinality failure code.
    pub fn failure_code(&self) -> &FailureCode {
        &self.failure_code
    }

    /// Returns broker dead-letter time.
    pub fn dead_lettered_at_ms(&self) -> i64 {
        self.dead_lettered_at_ms
    }
}

impl fmt::Debug for DeadLetter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeadLetter")
            .field("message_id", self.envelope.id())
            .field("group", &self.group)
            .field("attempts", &self.attempts)
            .field("failure_code", &self.failure_code)
            .field("dead_lettered_at_ms", &self.dead_lettered_at_ms)
            .finish()
    }
}

/// Explicit bounded terminal-message cleanup request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeRequest {
    topic: TopicName,
    limit: usize,
}

impl PurgeRequest {
    /// Creates a cleanup request for at most 1,000 messages.
    pub fn try_new(topic: impl Into<String>, limit: usize) -> Result<Self> {
        if !(1..=MAX_PURGE_BATCH).contains(&limit) {
            return Err(MessagingError::Invalid {
                field: "purge limit",
                reason: "must be between 1 and 1000",
            });
        }
        Ok(Self {
            topic: TopicName::try_new(topic)?,
            limit,
        })
    }

    pub(crate) fn topic(&self) -> &TopicName {
        &self.topic
    }

    pub(crate) fn limit(&self) -> usize {
        self.limit
    }
}

/// Result of explicit terminal-message cleanup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeReceipt {
    removed: usize,
}

impl PurgeReceipt {
    pub(crate) fn new(removed: usize) -> Self {
        Self { removed }
    }

    /// Returns the number of retained messages removed.
    pub fn removed(&self) -> usize {
        self.removed
    }
}
