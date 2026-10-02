//! Opt-in relay from the relational ORM outbox to a broker contract.

use crate::{
    MessageBroker, MessagingError, Namespace, PublishReceipt, PublishRequest, Result, TopicName,
};
use rullst_orm::{ClaimedOutboxEvent, Outbox};
use sha2::{Digest, Sha256};
use std::fmt;

const MAX_CLAIM_ATTEMPTS: i32 = 100;
const MAX_CLAIM_KEY_BYTES: usize = 128;
const MAX_OUTBOX_PAYLOAD_BYTES: usize = 1024 * 1024;

/// Result of publishing one claimed ORM outbox event and then attempting its ACK.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxRelayReceipt {
    publication: PublishReceipt,
    outbox_acknowledged: bool,
}

impl OutboxRelayReceipt {
    /// Returns the broker's original-or-replayed publication receipt.
    pub fn publication(&self) -> &PublishReceipt {
        &self.publication
    }

    /// Returns whether the exact still-live ORM claim was acknowledged.
    pub fn outbox_acknowledged(&self) -> bool {
        self.outbox_acknowledged
    }
}

/// Bounded relay failures that do not expose payloads, event keys, or claim tokens.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum OrmOutboxRelayError {
    /// A public or persisted claim cannot form the configured publication.
    #[error("ORM outbox claim is invalid for the configured messaging relay")]
    InvalidClaim {
        /// Secret-free validation failure.
        #[source]
        source: MessagingError,
    },
    /// The broker rejected or could not persist the publication.
    #[error("ORM outbox relay publication failed")]
    Publication {
        /// Secret-free broker failure.
        #[source]
        source: MessagingError,
    },
    /// The broker accepted the event but ORM acknowledgement could not be evaluated.
    #[error("ORM outbox acknowledgement failed after broker publication")]
    AcknowledgementUnavailable {
        /// Receipt proving whether the broker treated this attempt as a replay.
        publication: PublishReceipt,
    },
}

impl OrmOutboxRelayError {
    /// Returns a broker receipt when publication succeeded before an ACK backend failure.
    pub fn accepted_publication(&self) -> Option<&PublishReceipt> {
        match self {
            Self::AcknowledgementUnavailable { publication } => Some(publication),
            Self::InvalidClaim { .. } | Self::Publication { .. } => None,
        }
    }
}

/// Static-dispatch bridge from one exact ORM outbox stream to one broker topic.
///
/// The application must enqueue its domain mutation and [`Outbox`] event in the
/// same database transaction, then supervise claiming and retries. Publication
/// and ORM acknowledgement are necessarily two operations. A crash between
/// them republishes the same event under the same broker idempotency key,
/// which the broker treats as an exact idempotent replay when its content is
/// unchanged.
///
/// The ORM makes `event_key` unique only within its stream, while a broker
/// deduplicates per topic. The broker idempotency key is therefore the
/// lowercase hex SHA-256 of the stream, `/`, then the event key, so relays of
/// different streams can share a topic without suppressing or conflicting
/// with each other's events.
pub struct OrmOutboxRelay<B> {
    stream: Namespace,
    topic: TopicName,
    broker: B,
}

impl<B> OrmOutboxRelay<B> {
    /// Binds one validated ORM stream and broker topic to a concrete broker.
    pub fn try_new(stream: impl Into<String>, topic: impl Into<String>, broker: B) -> Result<Self> {
        Ok(Self {
            stream: Namespace::try_new(stream)?,
            topic: TopicName::try_new(topic)?,
            broker,
        })
    }

    /// Returns the configured ORM outbox stream.
    pub fn stream(&self) -> &str {
        self.stream.as_str()
    }

    /// Returns the configured broker topic.
    pub fn topic(&self) -> &TopicName {
        &self.topic
    }

    /// Borrows the concrete broker for application-specific administration.
    pub fn broker(&self) -> &B {
        &self.broker
    }
}

impl<B: MessageBroker> OrmOutboxRelay<B> {
    /// Publishes a claim without acknowledging it in the ORM outbox.
    ///
    /// This split form lets an application compose custom acknowledgement and
    /// telemetry policy. Prefer [`Self::relay_and_ack`] for the common order.
    pub async fn publish_claim(
        &self,
        claim: &ClaimedOutboxEvent,
    ) -> std::result::Result<PublishReceipt, OrmOutboxRelayError> {
        let request = self.request_from_claim(claim)?;
        self.broker
            .publish(request)
            .await
            .map_err(|source| OrmOutboxRelayError::Publication { source })
    }

    /// Publishes, then acknowledges only the exact ORM lease represented by the claim.
    pub async fn relay_and_ack(
        &self,
        claim: ClaimedOutboxEvent,
    ) -> std::result::Result<OutboxRelayReceipt, OrmOutboxRelayError> {
        let publication = self.publish_claim(&claim).await?;
        let outbox_acknowledged = Outbox::acknowledge(claim.id, claim.claim_key)
            .await
            .map_err(|_| OrmOutboxRelayError::AcknowledgementUnavailable {
                publication: publication.clone(),
            })?;
        Ok(OutboxRelayReceipt {
            publication,
            outbox_acknowledged,
        })
    }

    fn request_from_claim(
        &self,
        claim: &ClaimedOutboxEvent,
    ) -> std::result::Result<PublishRequest, OrmOutboxRelayError> {
        let normalized_json = if claim.payload_json.len() <= MAX_OUTBOX_PAYLOAD_BYTES {
            serde_json::from_str::<serde_json::Value>(&claim.payload_json)
                .and_then(|value| serde_json::to_string(&value))
                .ok()
        } else {
            None
        };
        if claim.stream != self.stream.as_str()
            || claim.id <= 0
            || !(1..=MAX_CLAIM_ATTEMPTS).contains(&claim.attempts)
            || claim.claim_expires_at_epoch <= 0
            || !valid_claim_key(&claim.claim_key)
            || normalized_json.is_none()
        {
            return Err(invalid_claim(MessagingError::Invalid {
                field: "ORM outbox claim",
                reason: "claim metadata or JSON payload is invalid",
            }));
        }
        let normalized_json = normalized_json.ok_or_else(|| {
            invalid_claim(MessagingError::Invalid {
                field: "ORM outbox claim",
                reason: "claim JSON payload is invalid",
            })
        })?;
        PublishRequest::try_new(
            self.topic.as_str(),
            &claim.event_kind,
            broker_idempotency_key(self.stream.as_str(), &claim.event_key),
            normalized_json.into_bytes(),
        )
        .and_then(|request| request.with_content_type("application/json"))
        .map_err(invalid_claim)
    }
}

impl<B> fmt::Debug for OrmOutboxRelay<B> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrmOutboxRelay")
            .field("stream", &"[REDACTED]")
            .field("topic", &self.topic)
            .field("broker", &std::any::type_name::<B>())
            .finish()
    }
}

/// Scopes the stream-unique outbox event key to its stream.
///
/// The fixed-length digest keeps the result within the 255-byte idempotency
/// bound for the longest stream and event key (64 + 1 + 128 bytes) and makes
/// the separator unambiguous.
fn broker_idempotency_key(stream: &str, event_key: &str) -> String {
    let mut key = String::with_capacity(65 + event_key.len());
    for byte in Sha256::digest(stream.as_bytes()) {
        key.push(hex_digit(byte >> 4));
        key.push(hex_digit(byte & 0x0f));
    }
    key.push('/');
    key.push_str(event_key);
    key
}

fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).unwrap_or('0')
}

fn valid_claim_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CLAIM_KEY_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

const fn invalid_claim(source: MessagingError) -> OrmOutboxRelayError {
    OrmOutboxRelayError::InvalidClaim { source }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_key_scopes_the_event_key_by_stream_within_the_bound() {
        let key = broker_idempotency_key("tenant-a", "order-42");
        assert_eq!(
            key,
            "80a707af7dc77ee1228f9127180f3964835e5beb4c4ab0d812f0fe7593579b3a/order-42"
        );
        assert_ne!(key, broker_idempotency_key("tenant-b", "order-42"));
        let longest = broker_idempotency_key(&"s".repeat(128), &"k".repeat(128));
        assert_eq!(longest.len(), 64 + 1 + 128);
        assert!(crate::IdempotencyKey::try_new(longest).is_ok());
    }
}
