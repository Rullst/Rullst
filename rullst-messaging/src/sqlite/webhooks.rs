//! Private composition boundaries for the outgoing webhook outbox.
use super::{
    SqliteBroker,
    transaction::{finish, storage_error},
};
use crate::{Clock, Delivery, MessagingError, Result};

impl<C: Clock> SqliteBroker<C> {
    pub(crate) async fn validate_webhook_delivery(
        &self,
        delivery: &Delivery,
        topic: &str,
        group: &str,
        window: i64,
    ) -> Result<i64> {
        let mut tx = self.begin_write("begin webhook dispatch check").await?;
        let now = self.now()?;
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT d.lease_expires_at_ms,m.published_at_ms
            FROM rullst_messaging_deliveries d JOIN rullst_messaging_messages m
            ON m.namespace=d.namespace AND m.topic=d.topic AND m.sequence=d.sequence
            WHERE d.namespace=? AND d.topic=? AND d.group_name=? AND d.state='in_flight'
            AND d.ack_token=? AND m.message_id=?",
        )
        .bind(self.config.namespace().as_str())
        .bind(topic)
        .bind(group)
        .bind(delivery.ack_token().as_str())
        .bind(delivery.envelope().id().as_str())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| storage_error("check webhook lease"))?;
        let (expires, created) = row.ok_or(MessagingError::LeaseNotFound)?;
        let end = created
            .checked_add(window)
            .ok_or(MessagingError::ClockOutOfRange)?
            .min(expires);
        if now < created || now >= end {
            return Err(MessagingError::LeaseExpired);
        }
        let after = self.now()?;
        if after < now || after >= end {
            return Err(MessagingError::LeaseExpired);
        }
        tx.commit()
            .await
            .map_err(|_| storage_error("finish webhook dispatch check"))?;
        let after = self.now()?;
        if after < now || after >= end {
            return Err(MessagingError::LeaseExpired);
        }
        Ok(end)
    }
    pub(crate) async fn retry_webhook_failure(
        &self,
        id: &str,
        topic: &str,
        group: &str,
        window: i64,
    ) -> Result<()> {
        let mut tx = self.begin_write("begin webhook operator retry").await?;
        let now = self.now()?;
        let result=async {
            let row:Option<(i64,i64)>=sqlx::query_as("SELECT m.sequence,m.published_at_ms FROM rullst_messaging_messages m
                JOIN rullst_messaging_deliveries d ON d.namespace=m.namespace AND d.topic=m.topic AND d.sequence=m.sequence
                WHERE m.namespace=? AND m.topic=? AND d.group_name=? AND m.message_id=? AND d.state='dead'
                AND d.failure_code!='webhook.cancelled'")
                .bind(self.config.namespace().as_str()).bind(topic).bind(group).bind(id)
                .fetch_optional(&mut *tx).await.map_err(|_|storage_error("find webhook failure"))?;
            let (sequence,created)=row.ok_or(MessagingError::LeaseNotFound)?;
            let end=created.checked_add(window).ok_or(MessagingError::ClockOutOfRange)?;
            if now<created || now>=end {return Err(MessagingError::LeaseExpired);}
            sqlx::query("UPDATE rullst_messaging_deliveries SET state='pending',available_at_ms=?,attempt=0,
                ack_token=NULL,consumer_name=NULL,lease_expires_at_ms=NULL,failure_code=NULL,dead_lettered_at_ms=NULL
                WHERE namespace=? AND topic=? AND group_name=? AND sequence=?")
                .bind(now).bind(self.config.namespace().as_str()).bind(topic).bind(group).bind(sequence)
                .execute(&mut *tx).await.map_err(|_|storage_error("retry webhook failure"))?;
            let after=self.now()?;if after<now || after>=end {return Err(MessagingError::LeaseExpired);} Ok(())
        }.await;
        finish(tx, result, "finish webhook operator retry").await
    }
    pub(crate) async fn cancel_webhook(&self, id: &str, topic: &str, group: &str) -> Result<()> {
        let mut tx = self.begin_write("begin webhook cancellation").await?;
        let now = self.now()?;
        let result=async {
            let updated=sqlx::query("UPDATE rullst_messaging_deliveries SET state='dead',available_at_ms=NULL,
                ack_token=NULL,consumer_name=NULL,lease_expires_at_ms=NULL,failure_code='webhook.cancelled',dead_lettered_at_ms=?
                WHERE namespace=? AND topic=? AND group_name=? AND state!='acked' AND sequence IN
                (SELECT sequence FROM rullst_messaging_messages WHERE namespace=? AND topic=? AND message_id=?)")
                .bind(now).bind(self.config.namespace().as_str()).bind(topic).bind(group)
                .bind(self.config.namespace().as_str()).bind(topic).bind(id).execute(&mut *tx).await.map_err(|_|storage_error("cancel webhook"))?;
            if updated.rows_affected()!=1 {return Err(MessagingError::LeaseNotFound);} Ok(())
        }.await;
        finish(tx, result, "finish webhook cancellation").await
    }
    pub(crate) async fn purge_webhooks_before(
        &self,
        topic: &str,
        group: &str,
        cutoff: i64,
        window: i64,
        limit: i64,
    ) -> Result<u64> {
        let mut tx = self.begin_write("begin webhook retention").await?;
        let now = self.now()?;
        let result=async {
            if cutoff<0 || cutoff>now.saturating_sub(window) {return Err(MessagingError::ClockOutOfRange);}
            let deleted=sqlx::query("DELETE FROM rullst_messaging_messages WHERE namespace=? AND topic=? AND sequence IN
                (SELECT m.sequence FROM rullst_messaging_messages m JOIN rullst_messaging_deliveries d
                 ON d.namespace=m.namespace AND d.topic=m.topic AND d.sequence=m.sequence
                 WHERE m.namespace=? AND m.topic=? AND d.group_name=? AND d.state IN ('acked','dead')
                 AND m.published_at_ms < ? ORDER BY m.sequence LIMIT ?)")
                 .bind(self.config.namespace().as_str()).bind(topic).bind(self.config.namespace().as_str()).bind(topic)
                 .bind(group).bind(cutoff).bind(limit).execute(&mut *tx).await.map_err(|_|storage_error("purge webhook terminal state"))?;
            Ok(deleted.rows_affected())
        }.await;
        finish(tx, result, "finish webhook retention").await
    }
}
