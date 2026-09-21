use super::outbox::{EVENTS, GROUP, map};
use super::*;
use crate::{Delivery, FailureCode, MessageBroker, ReceiveRequest, RetryDisposition};

/// Receiver acceptance and durable retry state; no response body or destination appears.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebhookDispatch {
    Idle,
    Accepted {
        delivery_id: String,
        status: u16,
        offline: bool,
    },
    RetryScheduled {
        delivery_id: String,
        attempt: u32,
        available_at_ms: i64,
    },
    DeadLettered {
        delivery_id: String,
        attempt: u32,
    },
}
impl<C: Clock> WebhookOutbox<C> {
    /// Dispatches one claimed event. Run from a supervised, bounded worker loop.
    /// Cancellation/lease expiry after the final check cannot recall receiver effects.
    pub async fn dispatch_next(&self, worker: impl Into<String>) -> Result<WebhookDispatch> {
        now(&self.clock)?;
        let request = ReceiveRequest::try_new(EVENTS, GROUP, worker, 1, Duration::from_secs(30))
            .map_err(map)?;
        let mut deliveries =
            bounded(async { self.broker.receive(request).await.map_err(map) }).await?;
        let Some(delivery) = deliveries.pop() else {
            return Ok(WebhookDispatch::Idle);
        };
        let current = now(&self.clock)?;
        let created = delivery.envelope().published_at_ms();
        let expires = created
            .checked_add(self.config.window_ms)
            .ok_or(WebhookError::Clock)?;
        if current < created {
            return Err(WebhookError::Clock);
        }
        if current >= expires {
            return self.dead(&delivery, "webhook.expired").await;
        }
        let prepared = if self.key.offline {
            None
        } else {
            match transport::PreparedRequest::resolve(&self.config.destination).await {
                Ok(value) => Some(value),
                Err(WebhookError::DestinationDenied | WebhookError::Configuration) => {
                    return self.dead(&delivery, "webhook.destination_denied").await;
                }
                Err(_) => {
                    return self
                        .retry(&delivery, "webhook.resolution", None, expires)
                        .await;
                }
            }
        };
        // The authoritative SQL check follows DNS work and precedes any HTTP bytes.
        let deadline = bounded(async {
            self.broker
                .validate_webhook_delivery(&delivery, EVENTS, GROUP, self.config.window_ms)
                .await
                .map_err(map)
        })
        .await?;
        let current = now(&self.clock)?;
        if current >= deadline || current < created {
            return Err(WebhookError::InvalidLease);
        }
        let signature = self.key.sign(
            delivery.envelope().id().as_str(),
            delivery.envelope().event_kind().as_str(),
            current / 1000,
            delivery.envelope().payload(),
        )?;
        let outcome = if let Some(prepared) = prepared {
            match prepared
                .send(
                    &signature,
                    delivery.envelope().payload(),
                    Duration::from_millis((deadline - current).min(10_000) as u64),
                )
                .await
            {
                Ok(value) => value,
                Err(_) => {
                    return self
                        .retry(&delivery, "webhook.transport", None, expires)
                        .await;
                }
            }
        } else {
            transport::HttpOutcome {
                status: 200,
                retry_after: None,
            }
        };
        if (200..=299).contains(&outcome.status) {
            let id = delivery.envelope().id().as_str().to_owned();
            let acknowledged =
                bounded(async { self.broker.ack(delivery.ack_token()).await.map_err(map) }).await;
            if acknowledged.is_err() {
                return Err(WebhookError::Acknowledgement {
                    delivery_id: id,
                    status: outcome.status,
                });
            }
            return Ok(WebhookDispatch::Accepted {
                delivery_id: id,
                status: outcome.status,
                offline: self.key.offline,
            });
        }
        if matches!(outcome.status, 408 | 425 | 429 | 500..=599) {
            self.retry(
                &delivery,
                "webhook.receiver_transient",
                outcome.retry_after,
                expires,
            )
            .await
        } else {
            self.dead(&delivery, "webhook.receiver_rejected").await
        }
    }
    async fn dead(&self, delivery: &Delivery, code: &str) -> Result<WebhookDispatch> {
        let code = FailureCode::try_new(code).map_err(map)?;
        bounded(async {
            self.broker
                .dead_letter(delivery.ack_token(), code)
                .await
                .map_err(map)
        })
        .await?;
        Ok(WebhookDispatch::DeadLettered {
            delivery_id: delivery.envelope().id().as_str().to_owned(),
            attempt: delivery.attempt(),
        })
    }
    async fn retry(
        &self,
        delivery: &Delivery,
        code: &str,
        retry_after: Option<Duration>,
        expires: i64,
    ) -> Result<WebhookDispatch> {
        let current = now(&self.clock)?;
        let backoff = Duration::from_secs(1u64 << delivery.attempt().min(9));
        let delay = retry_after.map_or(backoff, |value| value.max(backoff));
        let end = current
            .checked_add(delay.as_millis() as i64)
            .ok_or(WebhookError::Clock)?;
        if end >= expires {
            return self.dead(delivery, "webhook.expired").await;
        }
        let code = FailureCode::try_new(code).map_err(map)?;
        let disposition = bounded(async {
            self.broker
                .retry(delivery.ack_token(), delay, code)
                .await
                .map_err(map)
        })
        .await?;
        let id = delivery.envelope().id().as_str().to_owned();
        Ok(match disposition {
            RetryDisposition::Scheduled { available_at_ms } => WebhookDispatch::RetryScheduled {
                delivery_id: id,
                attempt: delivery.attempt(),
                available_at_ms,
            },
            RetryDisposition::DeadLettered => WebhookDispatch::DeadLettered {
                delivery_id: id,
                attempt: delivery.attempt(),
            },
        })
    }
}
