use super::*;
use sqlx::{Any, Row, Transaction};

impl EmailLoginService {
    pub(super) async fn begin(
        &self,
        clock: &impl EmailLoginClock,
    ) -> Result<(Transaction<'_, Any>, i64), RecoveryError> {
        let started = now(clock)?;
        let mut tx = self.store.pool.begin().await?;
        self.store.lock_writes(&mut tx).await?;
        let current = now(clock)?;
        if current < started {
            return Err(RecoveryError::InvalidAction);
        }
        self.observe(&mut tx, current).await?;
        Ok((tx, current))
    }

    pub(super) async fn durable(&self, tx: &mut Transaction<'_, Any>) -> Result<(), RecoveryError> {
        if self.postgres {
            // Explicit integer casts are portable through SQLx Any.
            let durable: i64 = sqlx::query_scalar("SELECT CASE WHEN pg_catalog.current_setting('fsync') = 'on' AND pg_catalog.current_setting('full_page_writes') = 'on' AND pg_catalog.current_setting('synchronous_commit') IN ('on','remote_apply') AND NOT pg_catalog.pg_is_in_recovery() THEN 1::bigint ELSE 0::bigint END")
                .fetch_one(&mut **tx).await?;
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND c.relname IN ('rullst_recovery_control','rullst_recovery_accounts','rullst_recovery_sessions','rullst_recovery_session_details','rullst_email_login_control','rullst_email_login_accounts','rullst_email_login_tokens','rullst_email_login_outbox') AND c.relkind = 'r' AND c.relpersistence = 'p'")
                .fetch_one(&mut **tx).await?;
            if durable != 1 || count != 8 {
                return Err(RecoveryError::Configuration);
            }
        } else {
            let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
                .fetch_one(&mut **tx)
                .await?;
            let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
                .fetch_one(&mut **tx)
                .await?;
            if !mode.eq_ignore_ascii_case("wal") || synchronous != 2 {
                return Err(RecoveryError::Configuration);
            }
        }
        Ok(())
    }

    pub(super) async fn rate(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: i64,
        consume: bool,
    ) -> Result<bool, RecoveryError> {
        let row = sqlx::query("SELECT request_window,requests,consume_window,consumes FROM rullst_email_login_control WHERE namespace = $1")
            .bind(&self.config.namespace).fetch_one(&mut **tx).await?;
        let (window, count): (i64, i64) = if consume {
            (row.try_get("consume_window")?, row.try_get("consumes")?)
        } else {
            (row.try_get("request_window")?, row.try_get("requests")?)
        };
        if window < 0 || count < 0 || current < window {
            return Err(RecoveryError::Configuration);
        }
        let (window, count) = if current - window >= 60 {
            (current, 0)
        } else {
            (window, count)
        };
        let limit = if consume { 60 } else { 120 };
        if count >= limit {
            return Ok(false);
        }
        let statement = if consume {
            "UPDATE rullst_email_login_control SET consume_window = $1, consumes = $2 WHERE namespace = $3"
        } else {
            "UPDATE rullst_email_login_control SET request_window = $1, requests = $2 WHERE namespace = $3"
        };
        sqlx::query(statement)
            .bind(window)
            .bind(count + 1)
            .bind(&self.config.namespace)
            .execute(&mut **tx)
            .await?;
        Ok(true)
    }

    pub(super) async fn finish_time(
        &self,
        tx: &mut Transaction<'_, Any>,
        clock: &impl EmailLoginClock,
        started: i64,
        expiry: i64,
    ) -> Result<i64, RecoveryError> {
        let current = now(clock)?;
        if current < started || current >= expiry {
            return Err(RecoveryError::InvalidAction);
        }
        self.observe(tx, current).await?;
        Ok(current)
    }

    pub(super) fn digest(&self, purpose: &str, value: &str) -> String {
        self.store
            .keys
            .digest(purpose, &format!("{}:{value}", self.config.namespace))
    }
}
