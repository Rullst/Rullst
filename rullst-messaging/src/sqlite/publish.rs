use crate::{
    Clock, MessageId, MessagingError, PublishReceipt, PublishRequest, Result, StartPosition,
    SubscriptionReceipt, SubscriptionRequest,
};
use sqlx::SqliteConnection;

use super::SqliteBroker;
use super::codec::fingerprint;
use super::storage::MessageBinding;
use super::transaction::{finish, storage_error};

impl<C: Clock> SqliteBroker<C> {
    pub(super) async fn publish_inner(&self, request: PublishRequest) -> Result<PublishReceipt> {
        request.validate_payload(self.config.max_payload_bytes())?;
        let now = self.now()?;
        let mut connection = self.begin_write("begin publication").await?;
        let result = self
            .publish_in_transaction(&mut connection, request, now)
            .await;
        finish(connection, result, "finish publication").await
    }

    async fn publish_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        request: PublishRequest,
        now: i64,
    ) -> Result<PublishReceipt> {
        let proposed_fingerprint = request.fingerprint()?;
        let existing: Option<(Vec<u8>, String, i64)> = sqlx::query_as(
            "SELECT fingerprint, message_id, published_at_ms FROM rullst_messaging_messages WHERE namespace = ? AND topic = ? AND idempotency_key = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.idempotency_key().as_str())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| storage_error("lookup idempotent publication"))?;
        if let Some((stored_fingerprint, message_id, published_at_ms)) = existing {
            if fingerprint(stored_fingerprint)? != proposed_fingerprint {
                return Err(MessagingError::IdempotencyConflict);
            }
            if published_at_ms < 0 {
                return Err(MessagingError::CorruptStorage {
                    context: "publication timestamp",
                });
            }
            return Ok(PublishReceipt::new(
                MessageId::from_stored(message_id)?,
                true,
                published_at_ms,
            ));
        }

        let retained: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rullst_messaging_messages WHERE namespace = ?")
                .bind(self.config.namespace().as_str())
                .fetch_one(&mut *connection)
                .await
                .map_err(|_| storage_error("count retained messages"))?;
        let retained = usize::try_from(retained.0).map_err(|_| MessagingError::CorruptStorage {
            context: "retained message count",
        })?;
        if retained >= self.config.max_retained_messages() {
            return Err(MessagingError::CapacityExceeded {
                resource: "retained messages",
                limit: self.config.max_retained_messages(),
            });
        }

        ensure_topic(
            connection,
            self.config.namespace().as_str(),
            request.topic().as_str(),
        )
        .await?;
        let sequence: (i64,) = sqlx::query_as(
            "SELECT next_sequence FROM rullst_messaging_topics WHERE namespace = ? AND topic = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| storage_error("read topic sequence"))?;
        if sequence.0 <= 0 || sequence.0 == i64::MAX {
            return Err(MessagingError::CapacityExceeded {
                resource: "topic sequence",
                limit: usize::MAX,
            });
        }
        let advanced = sqlx::query(
            "UPDATE rullst_messaging_topics SET next_sequence = next_sequence + 1 WHERE namespace = ? AND topic = ? AND next_sequence = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(sequence.0)
        .execute(&mut *connection)
        .await
        .map_err(|_| storage_error("advance topic sequence"))?;
        if advanced.rows_affected() != 1 {
            return Err(MessagingError::CorruptStorage {
                context: "topic sequence advance",
            });
        }

        let message_id = MessageId::random();
        let binding = MessageBinding::message(
            self.config.namespace(),
            request.topic().as_str(),
            sequence.0,
            message_id.as_str(),
            request.event_kind().as_str(),
            request.content_type().as_str(),
            now,
        );
        let (headers_json, stored_payload) =
            self.storage
                .encode_message(binding, request.headers(), request.payload())?;
        sqlx::query("INSERT INTO rullst_messaging_messages (namespace, topic, sequence, message_id, event_kind, content_type, headers_json, payload, published_at_ms, idempotency_key, fingerprint) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(self.config.namespace().as_str())
            .bind(request.topic().as_str())
            .bind(sequence.0)
            .bind(message_id.as_str())
            .bind(request.event_kind().as_str())
            .bind(request.content_type().as_str())
            .bind(headers_json)
            .bind(stored_payload)
            .bind(now)
            .bind(request.idempotency_key().as_str())
            .bind(proposed_fingerprint.as_slice())
            .execute(&mut *connection)
            .await
            .map_err(|_| storage_error("persist publication"))?;
        sqlx::query("INSERT INTO rullst_messaging_deliveries (namespace, topic, group_name, sequence, state, available_at_ms, attempt) SELECT namespace, topic, group_name, ?, 'pending', ?, 0 FROM rullst_messaging_subscriptions WHERE namespace = ? AND topic = ?")
            .bind(sequence.0)
            .bind(now)
            .bind(self.config.namespace().as_str())
            .bind(request.topic().as_str())
            .execute(&mut *connection)
            .await
            .map_err(|_| storage_error("fan out publication"))?;
        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %request.topic(),
            duplicate = false,
            payload_bytes = request.payload().len(),
            "durable messaging publication accepted"
        );
        Ok(PublishReceipt::new(message_id, false, now))
    }

    pub(super) async fn subscribe_inner(
        &self,
        request: SubscriptionRequest,
    ) -> Result<SubscriptionReceipt> {
        let mut connection = self.begin_write("begin subscription").await?;
        let result = self
            .subscribe_in_transaction(&mut connection, request)
            .await;
        finish(connection, result, "finish subscription").await
    }

    async fn subscribe_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        request: SubscriptionRequest,
    ) -> Result<SubscriptionReceipt> {
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM rullst_messaging_deliveries WHERE namespace = ? AND topic = ? AND group_name = ? AND state = 'pending' AND EXISTS (SELECT 1 FROM rullst_messaging_subscriptions WHERE namespace = ? AND topic = ? AND group_name = ?)",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.group().as_str())
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.group().as_str())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| storage_error("lookup subscription"))?;
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT start_sequence FROM rullst_messaging_subscriptions WHERE namespace = ? AND topic = ? AND group_name = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.group().as_str())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| storage_error("lookup subscription"))?;
        if exists.is_some() {
            let pending = existing.and_then(|row| usize::try_from(row.0).ok()).ok_or(
                MessagingError::CorruptStorage {
                    context: "pending subscription count",
                },
            )?;
            return Ok(SubscriptionReceipt::new(false, pending));
        }

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rullst_messaging_subscriptions WHERE namespace = ?",
        )
        .bind(self.config.namespace().as_str())
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| storage_error("count subscriptions"))?;
        let count = usize::try_from(count.0).map_err(|_| MessagingError::CorruptStorage {
            context: "subscription count",
        })?;
        if count >= self.config.max_subscriptions() {
            return Err(MessagingError::CapacityExceeded {
                resource: "message subscriptions",
                limit: self.config.max_subscriptions(),
            });
        }
        ensure_topic(
            connection,
            self.config.namespace().as_str(),
            request.topic().as_str(),
        )
        .await?;
        let next: (i64,) = sqlx::query_as(
            "SELECT next_sequence FROM rullst_messaging_topics WHERE namespace = ? AND topic = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| storage_error("read subscription cursor"))?;
        let start_sequence = match request.start() {
            StartPosition::Earliest => {
                let first: (Option<i64>,) = sqlx::query_as(
                    "SELECT MIN(sequence) FROM rullst_messaging_messages WHERE namespace = ? AND topic = ?",
                )
                .bind(self.config.namespace().as_str())
                .bind(request.topic().as_str())
                .fetch_one(&mut *connection)
                .await
                .map_err(|_| storage_error("read earliest sequence"))?;
                first.0.unwrap_or(next.0)
            }
            StartPosition::Latest => next.0,
        };
        if start_sequence <= 0 {
            return Err(MessagingError::CorruptStorage {
                context: "subscription cursor",
            });
        }
        sqlx::query("INSERT INTO rullst_messaging_subscriptions (namespace, topic, group_name, start_sequence) VALUES (?, ?, ?, ?)")
            .bind(self.config.namespace().as_str())
            .bind(request.topic().as_str())
            .bind(request.group().as_str())
            .bind(start_sequence)
            .execute(&mut *connection)
            .await
            .map_err(|_| storage_error("persist subscription"))?;
        let pending = if request.start() == StartPosition::Earliest {
            let inserted = sqlx::query("INSERT INTO rullst_messaging_deliveries (namespace, topic, group_name, sequence, state, available_at_ms, attempt) SELECT namespace, topic, ?, sequence, 'pending', published_at_ms, 0 FROM rullst_messaging_messages WHERE namespace = ? AND topic = ? AND sequence >= ? ORDER BY sequence")
                .bind(request.group().as_str())
                .bind(self.config.namespace().as_str())
                .bind(request.topic().as_str())
                .bind(start_sequence)
                .execute(&mut *connection)
                .await
                .map_err(|_| storage_error("initialize subscription"))?;
            usize::try_from(inserted.rows_affected()).map_err(|_| {
                MessagingError::CorruptStorage {
                    context: "subscription initialization count",
                }
            })?
        } else {
            0
        };
        Ok(SubscriptionReceipt::new(true, pending))
    }
}

async fn ensure_topic(
    connection: &mut SqliteConnection,
    namespace: &str,
    topic: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO rullst_messaging_topics (namespace, topic, next_sequence) VALUES (?, ?, 1)",
    )
    .bind(namespace)
    .bind(topic)
    .execute(connection)
    .await
    .map_err(|_| storage_error("register topic"))?;
    Ok(())
}

#[cfg(test)]
#[path = "publish_tests.rs"]
mod tests;
