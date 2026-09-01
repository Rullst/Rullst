use crate::validation::MAX_RETRY_MILLIS;
use crate::{
    AckToken, Clock, Delivery, FailureCode, MessagingError, ReceiveRequest, Result,
    RetryDisposition,
};
use sqlx::SqliteConnection;
use std::time::Duration;

use super::SqliteBroker;
use super::codec::{EnvelopeRow, attempt, decode_envelope};
use super::transaction::{finish, storage_error};

type LeaseRow = (String, String, i64, i64, i64);

impl<C: Clock> SqliteBroker<C> {
    pub(super) async fn receive_inner(&self, request: ReceiveRequest) -> Result<Vec<Delivery>> {
        let now = self.now()?;
        let expires_at_ms = add_millis(now, request.lease_millis())?;
        let mut connection = self.begin_write("begin receive").await?;
        let result = self
            .receive_in_transaction(&mut connection, request, now, expires_at_ms)
            .await;
        finish(&mut connection, result, "finish receive").await
    }

    async fn receive_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        request: ReceiveRequest,
        now: i64,
        expires_at_ms: i64,
    ) -> Result<Vec<Delivery>> {
        let subscription: Option<(i64,)> = sqlx::query_as(
            "SELECT start_sequence FROM rullst_messaging_subscriptions WHERE namespace = ? AND topic = ? AND group_name = ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.group().as_str())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| storage_error("lookup receive subscription"))?;
        if subscription.is_none() {
            return Err(MessagingError::SubscriptionNotFound);
        }
        expire_group_leases(
            connection,
            self.config.namespace().as_str(),
            request.topic().as_str(),
            request.group().as_str(),
            now,
            self.config.max_attempts(),
        )
        .await?;

        let limit =
            i64::try_from(request.max_messages()).map_err(|_| MessagingError::InternalState {
                context: "receive batch conversion",
            })?;
        let candidates: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT sequence, attempt FROM rullst_messaging_deliveries WHERE namespace = ? AND topic = ? AND group_name = ? AND state = 'pending' AND available_at_ms <= ? ORDER BY sequence LIMIT ?",
        )
        .bind(self.config.namespace().as_str())
        .bind(request.topic().as_str())
        .bind(request.group().as_str())
        .bind(now)
        .bind(limit)
        .fetch_all(&mut *connection)
        .await
        .map_err(|_| storage_error("select pending deliveries"))?;
        let mut deliveries = Vec::with_capacity(candidates.len());
        for (sequence, prior_attempt) in candidates {
            let prior_attempt = attempt(prior_attempt)?;
            if prior_attempt >= self.config.max_attempts() {
                mark_dead(
                    connection,
                    self.config.namespace().as_str(),
                    request.topic().as_str(),
                    request.group().as_str(),
                    sequence,
                    FailureCode::max_attempts().as_str(),
                    now,
                )
                .await?;
                continue;
            }
            let next_attempt =
                prior_attempt
                    .checked_add(1)
                    .ok_or(MessagingError::InternalState {
                        context: "delivery attempt increment",
                    })?;
            let row: EnvelopeRow = sqlx::query_as(
                "SELECT message_id, event_kind, content_type, headers_json, payload, published_at_ms FROM rullst_messaging_messages WHERE namespace = ? AND topic = ? AND sequence = ?",
            )
            .bind(self.config.namespace().as_str())
            .bind(request.topic().as_str())
            .bind(sequence)
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| storage_error("load pending message"))?;
            let envelope = decode_envelope(
                self.config.namespace(),
                request.topic().as_str(),
                row,
                self.config.max_payload_bytes(),
            )?;
            let token = AckToken::random();
            let updated = sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'in_flight', available_at_ms = NULL, attempt = ?, ack_token = ?, consumer_name = ?, lease_expires_at_ms = ?, failure_code = NULL, dead_lettered_at_ms = NULL WHERE namespace = ? AND topic = ? AND group_name = ? AND sequence = ? AND state = 'pending'")
                .bind(i64::from(next_attempt))
                .bind(token.as_str())
                .bind(request.consumer().as_str())
                .bind(expires_at_ms)
                .bind(self.config.namespace().as_str())
                .bind(request.topic().as_str())
                .bind(request.group().as_str())
                .bind(sequence)
                .execute(&mut *connection)
                .await
                .map_err(|_| storage_error("claim pending delivery"))?;
            if updated.rows_affected() != 1 {
                return Err(MessagingError::CorruptStorage {
                    context: "pending delivery claim",
                });
            }
            deliveries.push(Delivery::new(
                envelope,
                request.group().clone(),
                request.consumer().clone(),
                next_attempt,
                expires_at_ms,
                token,
            ));
        }
        tracing::debug!(
            namespace = %self.config.namespace(),
            topic = %request.topic(),
            group = %request.group(),
            consumer = %request.consumer(),
            count = deliveries.len(),
            "durable messaging receive completed"
        );
        Ok(deliveries)
    }

    pub(super) async fn ack_inner(&self, token: &AckToken) -> Result<()> {
        let now = self.now()?;
        let mut connection = self.begin_write("begin acknowledgement").await?;
        let result = async {
            let lease = load_lease(&mut connection, self.config.namespace().as_str(), token)
                .await?
                .ok_or(MessagingError::LeaseNotFound)?;
            if lease.4 <= now {
                transition_expired(
                    &mut connection,
                    self.config.namespace().as_str(),
                    &lease,
                    now,
                    self.config.max_attempts(),
                )
                .await?;
                return Ok(false);
            }
            set_terminal(
                &mut connection,
                self.config.namespace().as_str(),
                &lease,
                "acked",
                None,
                None,
            )
            .await?;
            Ok(true)
        }
        .await;
        match finish(&mut connection, result, "finish acknowledgement").await? {
            true => Ok(()),
            false => Err(MessagingError::LeaseExpired),
        }
    }

    pub(super) async fn retry_inner(
        &self,
        token: &AckToken,
        delay: Duration,
        failure_code: FailureCode,
    ) -> Result<RetryDisposition> {
        let delay_millis = retry_millis(delay)?;
        let now = self.now()?;
        let available_at_ms = add_millis(now, delay_millis)?;
        let mut connection = self.begin_write("begin retry").await?;
        let result = async {
            let lease = load_lease(&mut connection, self.config.namespace().as_str(), token)
                .await?
                .ok_or(MessagingError::LeaseNotFound)?;
            if lease.4 <= now {
                transition_expired(
                    &mut connection,
                    self.config.namespace().as_str(),
                    &lease,
                    now,
                    self.config.max_attempts(),
                )
                .await?;
                return Ok(None);
            }
            let attempts = attempt(lease.3)?;
            let disposition = if attempts >= self.config.max_attempts() {
                set_terminal(
                    &mut connection,
                    self.config.namespace().as_str(),
                    &lease,
                    "dead",
                    Some(failure_code.as_str()),
                    Some(now),
                )
                .await?;
                RetryDisposition::DeadLettered
            } else {
                let updated = sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'pending', available_at_ms = ?, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = NULL, dead_lettered_at_ms = NULL WHERE namespace = ? AND topic = ? AND group_name = ? AND sequence = ? AND state = 'in_flight'")
                    .bind(available_at_ms)
                    .bind(self.config.namespace().as_str())
                    .bind(&lease.0)
                    .bind(&lease.1)
                    .bind(lease.2)
                    .execute(&mut *connection)
                    .await
                    .map_err(|_| storage_error("schedule retry"))?;
                ensure_one(updated.rows_affected(), "retry transition")?;
                RetryDisposition::Scheduled { available_at_ms }
            };
            Ok(Some(disposition))
        }
        .await;
        finish(&mut connection, result, "finish retry")
            .await?
            .ok_or(MessagingError::LeaseExpired)
    }

    pub(super) async fn dead_letter_inner(
        &self,
        token: &AckToken,
        failure_code: FailureCode,
    ) -> Result<()> {
        let now = self.now()?;
        let mut connection = self.begin_write("begin dead letter").await?;
        let result = async {
            let lease = load_lease(&mut connection, self.config.namespace().as_str(), token)
                .await?
                .ok_or(MessagingError::LeaseNotFound)?;
            if lease.4 <= now {
                transition_expired(
                    &mut connection,
                    self.config.namespace().as_str(),
                    &lease,
                    now,
                    self.config.max_attempts(),
                )
                .await?;
                return Ok(false);
            }
            set_terminal(
                &mut connection,
                self.config.namespace().as_str(),
                &lease,
                "dead",
                Some(failure_code.as_str()),
                Some(now),
            )
            .await?;
            Ok(true)
        }
        .await;
        match finish(&mut connection, result, "finish dead letter").await? {
            true => Ok(()),
            false => Err(MessagingError::LeaseExpired),
        }
    }
}

async fn load_lease(
    connection: &mut SqliteConnection,
    namespace: &str,
    token: &AckToken,
) -> Result<Option<LeaseRow>> {
    sqlx::query_as("SELECT topic, group_name, sequence, attempt, lease_expires_at_ms FROM rullst_messaging_deliveries WHERE namespace = ? AND ack_token = ? AND state = 'in_flight'")
        .bind(namespace)
        .bind(token.as_str())
        .fetch_optional(connection)
        .await
        .map_err(|_| storage_error("lookup acknowledgement lease"))
}

async fn transition_expired(
    connection: &mut SqliteConnection,
    namespace: &str,
    lease: &LeaseRow,
    now: i64,
    max_attempts: u32,
) -> Result<()> {
    if attempt(lease.3)? >= max_attempts {
        set_terminal(
            connection,
            namespace,
            lease,
            "dead",
            Some(FailureCode::max_attempts().as_str()),
            Some(now),
        )
        .await
    } else {
        let updated = sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'pending', available_at_ms = ?, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = NULL, dead_lettered_at_ms = NULL WHERE namespace = ? AND topic = ? AND group_name = ? AND sequence = ? AND state = 'in_flight'")
            .bind(now)
            .bind(namespace)
            .bind(&lease.0)
            .bind(&lease.1)
            .bind(lease.2)
            .execute(connection)
            .await
            .map_err(|_| storage_error("expire acknowledgement lease"))?;
        ensure_one(updated.rows_affected(), "expired lease transition")
    }
}

async fn expire_group_leases(
    connection: &mut SqliteConnection,
    namespace: &str,
    topic: &str,
    group: &str,
    now: i64,
    max_attempts: u32,
) -> Result<()> {
    sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'dead', available_at_ms = NULL, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = ?, dead_lettered_at_ms = ? WHERE namespace = ? AND topic = ? AND group_name = ? AND state = 'in_flight' AND lease_expires_at_ms <= ? AND attempt >= ?")
        .bind(FailureCode::max_attempts().as_str())
        .bind(now)
        .bind(namespace)
        .bind(topic)
        .bind(group)
        .bind(now)
        .bind(i64::from(max_attempts))
        .execute(&mut *connection)
        .await
        .map_err(|_| storage_error("dead-letter expired deliveries"))?;
    sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'pending', available_at_ms = ?, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = NULL, dead_lettered_at_ms = NULL WHERE namespace = ? AND topic = ? AND group_name = ? AND state = 'in_flight' AND lease_expires_at_ms <= ? AND attempt < ?")
        .bind(now)
        .bind(namespace)
        .bind(topic)
        .bind(group)
        .bind(now)
        .bind(i64::from(max_attempts))
        .execute(connection)
        .await
        .map_err(|_| storage_error("requeue expired deliveries"))?;
    Ok(())
}

async fn set_terminal(
    connection: &mut SqliteConnection,
    namespace: &str,
    lease: &LeaseRow,
    state: &'static str,
    failure_code: Option<&str>,
    dead_lettered_at_ms: Option<i64>,
) -> Result<()> {
    let updated = sqlx::query("UPDATE rullst_messaging_deliveries SET state = ?, available_at_ms = NULL, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = ?, dead_lettered_at_ms = ? WHERE namespace = ? AND topic = ? AND group_name = ? AND sequence = ? AND state = 'in_flight'")
        .bind(state)
        .bind(failure_code)
        .bind(dead_lettered_at_ms)
        .bind(namespace)
        .bind(&lease.0)
        .bind(&lease.1)
        .bind(lease.2)
        .execute(connection)
        .await
        .map_err(|_| storage_error("complete delivery"))?;
    ensure_one(updated.rows_affected(), "terminal delivery transition")
}

async fn mark_dead(
    connection: &mut SqliteConnection,
    namespace: &str,
    topic: &str,
    group: &str,
    sequence: i64,
    failure_code: &str,
    now: i64,
) -> Result<()> {
    let updated = sqlx::query("UPDATE rullst_messaging_deliveries SET state = 'dead', available_at_ms = NULL, ack_token = NULL, consumer_name = NULL, lease_expires_at_ms = NULL, failure_code = ?, dead_lettered_at_ms = ? WHERE namespace = ? AND topic = ? AND group_name = ? AND sequence = ? AND state = 'pending'")
        .bind(failure_code)
        .bind(now)
        .bind(namespace)
        .bind(topic)
        .bind(group)
        .bind(sequence)
        .execute(connection)
        .await
        .map_err(|_| storage_error("repair exhausted pending delivery"))?;
    ensure_one(updated.rows_affected(), "exhausted pending transition")
}

fn retry_millis(delay: Duration) -> Result<u64> {
    let millis = u64::try_from(delay.as_millis()).map_err(|_| MessagingError::Invalid {
        field: "retry delay",
        reason: "duration is outside the supported range",
    })?;
    if millis > MAX_RETRY_MILLIS {
        return Err(MessagingError::Invalid {
            field: "retry delay",
            reason: "must not exceed seven days",
        });
    }
    Ok(millis)
}

fn add_millis(timestamp: i64, millis: u64) -> Result<i64> {
    let millis = i64::try_from(millis).map_err(|_| MessagingError::ClockOutOfRange)?;
    timestamp
        .checked_add(millis)
        .ok_or(MessagingError::ClockOutOfRange)
}

fn ensure_one(rows: u64, context: &'static str) -> Result<()> {
    if rows == 1 {
        Ok(())
    } else {
        Err(MessagingError::CorruptStorage { context })
    }
}
