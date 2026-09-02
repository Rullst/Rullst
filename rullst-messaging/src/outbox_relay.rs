//! Opt-in relay from the relational ORM outbox to a broker contract.

use crate::{
    MessageBroker, MessagingError, Namespace, PublishReceipt, PublishRequest, Result, TopicName,
};
use rullst_orm::{ClaimedOutboxEvent, Outbox};
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
/// them republishes the same outbox `event_key`, which the broker treats as an
/// exact idempotent replay when its content is unchanged.
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
        let canonical_json = if claim.payload_json.len() <= MAX_OUTBOX_PAYLOAD_BYTES {
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
            || canonical_json.is_none()
        {
            return Err(invalid_claim(MessagingError::Invalid {
                field: "ORM outbox claim",
                reason: "claim metadata or JSON payload is invalid",
            }));
        }
        let canonical_json = canonical_json.ok_or_else(|| {
            invalid_claim(MessagingError::Invalid {
                field: "ORM outbox claim",
                reason: "claim JSON payload is invalid",
            })
        })?;
        PublishRequest::try_new(
            self.topic.as_str(),
            &claim.event_kind,
            &claim.event_key,
            canonical_json.into_bytes(),
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
