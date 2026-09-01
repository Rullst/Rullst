use crate::{
    Clock, DeadLetter, DeadLetterQuery, FailureCode, MessagingError, PurgeReceipt, PurgeRequest,
    Result,
};

use super::SqliteBroker;
use super::codec::{EnvelopeRow, attempt, decode_envelope};
use super::transaction::{finish, storage_error};

type DeadLetterRow = (
    i64,
    String,
    i64,
    String,
    String,
    String,
    String,
    Vec<u8>,
    i64,
);

impl<C: Clock> SqliteBroker<C> {
    pub(super) async fn dead_letters_inner(
        &self,
        query: DeadLetterQuery,
    ) -> Result<Vec<DeadLetter>> {
        let subscription: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM rullst_messaging_subscriptions WHERE namespace = ? AND topic = ? AND group_name = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(query.topic().as_str())
        .bind(query.group().as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| storage_error("lookup dead-letter subscription"))?;
        if subscription.is_none() {
            return Err(MessagingError::SubscriptionNotFound);
        }
        let limit = i64::try_from(query.limit()).map_err(|_| MessagingError::InternalState {
            context: "dead-letter limit conversion",
        })?;
        let rows: Vec<DeadLetterRow> = sqlx::query_as(
            "SELECT d.attempt, d.failure_code, d.dead_lettered_at_ms, m.message_id, m.event_kind, m.content_type, m.headers_json, m.payload, m.published_at_ms FROM rullst_messaging_deliveries d JOIN rullst_messaging_messages m ON m.namespace = d.namespace AND m.topic = d.topic AND m.sequence = d.sequence WHERE d.namespace = ? AND d.topic = ? AND d.group_name = ? AND d.state = 'dead' ORDER BY d.sequence LIMIT ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(query.topic().as_str())
        .bind(query.group().as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| storage_error("list dead letters"))?;
        let mut dead_letters = Vec::with_capacity(rows.len());
        for row in rows {
            if row.2 < 0 {
                return Err(MessagingError::CorruptStorage {
                    context: "dead-letter timestamp",
                });
            }
            let failure_code =
                FailureCode::try_new(row.1).map_err(|_| MessagingError::CorruptStorage {
                    context: "dead-letter failure code",
                })?;
            let envelope_row: EnvelopeRow = (row.3, row.4, row.5, row.6, row.7, row.8);
            let envelope = decode_envelope(
                self.config.namespace(),
                query.topic().as_str(),
                envelope_row,
                self.config.max_payload_bytes(),
            )?;
            dead_letters.push(DeadLetter::new(
                envelope,
                query.group().clone(),
                attempt(row.0)?,
                failure_code,
                row.2,
            ));
        }
        Ok(dead_letters)
    }

    pub(super) async fn purge_terminal_inner(&self, request: PurgeRequest) -> Result<PurgeReceipt> {
        let mut connection = self.begin_write("begin terminal purge").await?;
        let limit = i64::try_from(request.limit()).map_err(|_| MessagingError::InternalState {
            context: "purge limit conversion",
        })?;
        let result = async {
            let sequences: Vec<(i64,)> = sqlx::query_as(
                "SELECT m.sequence FROM rullst_messaging_messages m WHERE m.namespace = ? AND m.topic = ? AND EXISTS (SELECT 1 FROM rullst_messaging_subscriptions s0 WHERE s0.namespace = m.namespace AND s0.topic = m.topic) AND NOT EXISTS (SELECT 1 FROM rullst_messaging_subscriptions s WHERE s.namespace = m.namespace AND s.topic = m.topic AND m.sequence >= s.start_sequence AND NOT EXISTS (SELECT 1 FROM rullst_messaging_deliveries d WHERE d.namespace = s.namespace AND d.topic = s.topic AND d.group_name = s.group_name AND d.sequence = m.sequence AND d.state IN ('acked','dead'))) ORDER BY m.sequence LIMIT ?",
            )
            .bind(self.config.namespace().as_str())
            .bind(request.topic().as_str())
            .bind(limit)
            .fetch_all(&mut *connection)
            .await
            .map_err(|_| storage_error("select terminal messages"))?;
            for sequence in &sequences {
                let deleted = sqlx::query(
                    "DELETE FROM rullst_messaging_messages WHERE namespace = ? AND topic = ? AND sequence = ?",
                )
                .bind(self.config.namespace().as_str())
                .bind(request.topic().as_str())
                .bind(sequence.0)
                .execute(&mut *connection)
                .await
                .map_err(|_| storage_error("purge terminal message"))?;
                if deleted.rows_affected() != 1 {
                    return Err(MessagingError::CorruptStorage {
                        context: "terminal message purge",
                    });
                }
            }
            Ok(PurgeReceipt::new(sequences.len()))
        }
        .await;
        finish(&mut connection, result, "finish terminal purge").await
    }
}
