use super::*;
use crate::{
    BrokerConfig, DeadLetterQuery, IdempotencyKey, MessageAdmin, MessageBroker, MessageId,
    MessagingError, MessagingKeyring, PublishReceipt, PublishRequest, SqliteBroker, StartPosition,
    SubscriptionRequest,
};

pub(super) const EVENTS: &str = "rullst.webhook.events.v1";
pub(super) const GROUP: &str = "rullst.webhook.receiver.v1";
const CONTROL: &str = "rullst.webhook.control.v1";

/// Private encrypted SQLite outbox for one immutable destination and namespace.
/// Independent processes may share the same local file. Network filesystems and
/// application-domain transaction atomicity are not provided by this profile.
#[derive(Clone)]
pub struct WebhookOutbox<C = SystemClock> {
    pub(super) broker: SqliteBroker<C>,
    pub(super) config: WebhookConfig,
    pub(super) key: WebhookSigningKey,
    pub(super) clock: C,
}
impl<C: Clock> WebhookOutbox<C> {
    /// Opens/creates the durable outbox and binds its immutable destination/key.
    /// Signing credentials and encrypted-storage keys must be different secrets.
    pub async fn open(
        database_url: impl Into<String>,
        config: WebhookConfig,
        key: WebhookSigningKey,
        storage: MessagingKeyring,
        clock: C,
    ) -> Result<Self> {
        if config.production && key.offline {
            return Err(WebhookError::Configuration);
        }
        now(&clock)?;
        bounded(async {
            let limits = BrokerConfig::try_new(config.namespace())
                .and_then(|c| c.with_limits(config.retained + 1, 1, 10, MAX_BODY))
                .map_err(map)?;
            let broker = SqliteBroker::connect_encrypted_with_clock(
                database_url,
                limits,
                storage,
                clock.clone(),
            )
            .await
            .map_err(map)?;
            let setup = async {
                let control = PublishRequest::try_new(
                    CONTROL,
                    "webhook.configuration",
                    "immutable-v1",
                    config.binding(&key)?,
                )
                .map_err(map)?;
                broker.publish(control).await.map_err(map)?;
                broker
                    .subscribe(
                        SubscriptionRequest::try_new(EVENTS, GROUP, StartPosition::Earliest)
                            .map_err(map)?,
                    )
                    .await
                    .map_err(map)?;
                Ok(())
            }
            .await;
            if let Err(error) = setup {
                broker.close().await;
                return Err(error);
            }
            Ok(Self {
                broker,
                config,
                key,
                clock,
            })
        })
        .await
    }
    /// Publishes exact JSON bytes under a caller-owned idempotency key.
    /// Reusing a retained key with different kind/body fails atomically.
    pub async fn enqueue(
        &self,
        event_key: impl Into<String>,
        kind: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Result<PublishReceipt> {
        let event_key = IdempotencyKey::try_new(event_key).map_err(map)?;
        let body = body.into();
        if body.len() > MAX_BODY || serde_json::from_slice::<serde_json::Value>(&body).is_err() {
            return Err(WebhookError::InvalidInput("bounded JSON body"));
        }
        now(&self.clock)?;
        let key = self
            .key
            .event_tag(self.config.namespace(), event_key.as_str())?;
        let request = PublishRequest::try_new(EVENTS, kind, key, body)
            .and_then(|r| r.with_content_type("application/json"))
            .map_err(map)?;
        bounded(async { self.broker.publish(request).await.map_err(map) }).await
    }
    /// Permanently fences unpublished delivery. In-flight receiver side effects cannot be recalled.
    pub async fn cancel(&self, delivery_id: impl Into<String>) -> Result<()> {
        let id = MessageId::from_stored(delivery_id.into()).map_err(map)?;
        now(&self.clock)?;
        bounded(async {
            self.broker
                .cancel_webhook(id.as_str(), EVENTS, GROUP)
                .await
                .map_err(map)
        })
        .await
    }
    /// Starts a new explicit operator attempt budget within the original delivery window.
    /// Preserves the receiver's stable ID and never revives cancellation or acknowledgement.
    pub async fn retry_failed(&self, delivery_id: impl Into<String>) -> Result<()> {
        let id = MessageId::from_stored(delivery_id.into()).map_err(map)?;
        now(&self.clock)?;
        bounded(async {
            self.broker
                .retry_webhook_failure(id.as_str(), EVENTS, GROUP, self.config.window_ms)
                .await
                .map_err(map)
        })
        .await
    }
    /// Inspect at most 100 terminal failures without returning bodies, URLs or secrets.
    pub async fn failed(&self, limit: usize) -> Result<Vec<WebhookFailure>> {
        let query = DeadLetterQuery::try_new(EVENTS, GROUP, limit).map_err(map)?;
        bounded(async {
            Ok(self
                .broker
                .dead_letters(query)
                .await
                .map_err(map)?
                .into_iter()
                .map(|item| WebhookFailure {
                    id: item.envelope().id().as_str().to_owned(),
                    kind: item.envelope().event_kind().as_str().to_owned(),
                    attempts: item.attempts(),
                    code: item.failure_code().as_str().to_owned(),
                    at: item.dead_lettered_at_ms(),
                })
                .collect())
        })
        .await
    }
    /// Removes at most 100 terminal events older than the creation cutoff, never the control record.
    /// The cutoff must precede now by at least the configured full delivery window.
    pub async fn purge_terminal(&self, created_before_ms: i64, limit: usize) -> Result<u64> {
        if !(1..=100).contains(&limit) {
            return Err(WebhookError::InvalidInput("purge limit"));
        }
        now(&self.clock)?;
        bounded(async {
            self.broker
                .purge_webhooks_before(
                    EVENTS,
                    GROUP,
                    created_before_ms,
                    self.config.window_ms,
                    limit as i64,
                )
                .await
                .map_err(map)
        })
        .await
    }
    pub async fn close(self) {
        self.broker.close().await;
    }
}
impl<C> std::fmt::Debug for WebhookOutbox<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookOutbox([REDACTED])")
    }
}
pub(super) fn map(error: MessagingError) -> WebhookError {
    match error {
        MessagingError::Invalid { field, .. } => WebhookError::InvalidInput(field),
        MessagingError::IdempotencyConflict => WebhookError::Conflict,
        MessagingError::ConfigurationConflict => WebhookError::Configuration,
        MessagingError::CapacityExceeded { .. } => WebhookError::Capacity,
        MessagingError::ClockOutOfRange => WebhookError::Clock,
        MessagingError::LeaseExpired | MessagingError::LeaseNotFound => WebhookError::InvalidLease,
        _ => WebhookError::Storage,
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebhookFailure {
    id: String,
    kind: String,
    attempts: u32,
    code: String,
    at: i64,
}
impl WebhookFailure {
    pub fn delivery_id(&self) -> &str {
        &self.id
    }
    pub fn event_kind(&self) -> &str {
        &self.kind
    }
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
    pub fn failure_code(&self) -> &str {
        &self.code
    }
    pub fn failed_at_ms(&self) -> i64 {
        self.at
    }
}
