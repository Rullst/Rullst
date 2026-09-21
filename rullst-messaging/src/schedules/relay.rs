use super::*;
use crate::{MessageBroker, MessagingError, PublishRequest};
use sqlx::Row;
use std::time::Duration;

impl<C: Clock> PostgresRecurringStore<C> {
    async fn dispatch_request(&self, lease: &OccurrenceLease) -> Result<PublishRequest> {
        bounded(async {
            let (mut tx, now) = self.begin().await?;
            let row = self.verify(&mut tx, now, lease).await?;
            let bytes: Vec<u8> = row
                .try_get("content")
                .map_err(|_| RecurringError::Encryption)?;
            let message: ScheduledMessage = serde_json::from_slice(&crypto::open(
                &self.keys,
                self.config.namespace(),
                "occurrence",
                &lease.metadata.id,
                &bytes,
            )?)
            .map_err(|_| RecurringError::Encryption)?;
            let request = message.request(&format!("rullst-recurring-v1-{}", lease.metadata.id))?;
            self.commit(tx, now, Some(lease.expires)).await?;
            Ok(request)
        })
        .await
    }
    /// Checks the lease, publishes frozen content, then acknowledges broker acceptance.
    /// The host must select the same broker/namespace on every retry. Consumers must
    /// deduplicate effects; cancellation after the final check cannot recall publication.
    pub async fn relay<B: MessageBroker>(
        &self,
        lease: &OccurrenceLease,
        broker: &B,
    ) -> std::result::Result<RecurringRelayReceipt, RecurringRelayError> {
        let request = self
            .dispatch_request(lease)
            .await
            .map_err(RecurringRelayError::BeforePublication)?;
        let now = current(&self.clock).map_err(RecurringRelayError::BeforePublication)?;
        if now >= lease.expires {
            return Err(RecurringRelayError::BeforePublication(
                RecurringError::InvalidLease,
            ));
        }
        let timeout = Duration::from_millis((lease.expires - now).min(10_000) as u64);
        let result = tokio::time::timeout(timeout, broker.publish(request))
            .await
            .unwrap_or({
                Err(MessagingError::StorageUnavailable {
                    operation: "bounded recurring publication",
                })
            });
        match result {
            Ok(publication) => {
                self.finish(lease, true).await.map_err(|source| {
                    RecurringRelayError::Acknowledgement {
                        publication: publication.clone(),
                        source,
                    }
                })?;
                Ok(RecurringRelayReceipt {
                    publication,
                    acknowledged: true,
                })
            }
            Err(error) => {
                // If state is unavailable/expired, the lease remains recoverable by claim.
                // A timeout can follow acceptance: retries always retain the same broker key.
                let _ = self.finish(lease, false).await;
                Err(RecurringRelayError::Publication(error))
            }
        }
    }
}
