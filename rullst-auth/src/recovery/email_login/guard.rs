use super::*;
use sqlx::{Any, Row, Transaction};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Failed redemptions allowed per browser binding in one window.
const FAILURE_LIMIT: u32 = 60;
/// Failure window length in seconds.
const FAILURE_WINDOW: i64 = 60;
/// Browser bindings tracked at once; the oldest window is evicted beyond it.
const TRACKED_BINDINGS: usize = 10_000;

/// Process-local failed-redemption budgets keyed by the HMAC digest of the
/// browser binding and shared by clones of one service. Only failed token
/// lookups are charged, and memory stays bounded by `TRACKED_BINDINGS`.
#[derive(Clone, Default)]
pub(super) struct RedemptionFailures(Arc<Mutex<HashMap<String, (i64, u32)>>>);

impl RedemptionFailures {
    /// Whether this binding already spent its budget in the current window.
    pub(super) fn exhausted(&self, binding: &str, now: i64) -> Result<bool, RecoveryError> {
        let budgets = self.0.lock().map_err(|_| RecoveryError::Storage)?;
        Ok(budgets.get(binding).is_some_and(|(start, failures)| {
            (*start..start.saturating_add(FAILURE_WINDOW)).contains(&now)
                && *failures >= FAILURE_LIMIT
        }))
    }

    /// Records one failed lookup for this binding.
    pub(super) fn charge(&self, binding: String, now: i64) -> Result<(), RecoveryError> {
        let mut budgets = self.0.lock().map_err(|_| RecoveryError::Storage)?;
        if !budgets.contains_key(&binding) && budgets.len() >= TRACKED_BINDINGS {
            budgets.retain(|_, (start, _)| {
                (*start..start.saturating_add(FAILURE_WINDOW)).contains(&now)
            });
            if budgets.len() >= TRACKED_BINDINGS
                && let Some(oldest) = budgets
                    .iter()
                    .min_by_key(|(_, (start, _))| *start)
                    .map(|(key, _)| key.clone())
            {
                budgets.remove(&oldest);
            }
        }
        let (start, failures) = budgets.entry(binding).or_insert((now, 0));
        if !(*start..start.saturating_add(FAILURE_WINDOW)).contains(&now) {
            (*start, *failures) = (now, 0);
        }
        *failures = failures.saturating_add(1);
        Ok(())
    }
}

impl EmailLoginService {
    pub(super) async fn begin(
        &self,
        clock: &impl EmailLoginClock,
    ) -> Result<(Transaction<'_, Any>, i64), RecoveryError> {
        let started = now(clock)?;
        let mut tx = self.store.pool.begin().await?;
        self.store.lock_writes(&mut tx).await?;
        let current = advance_clock(now(clock)?, started)?;
        let current = self.observe(&mut tx, current).await?;
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

    /// Namespace-wide request budget. Redemption uses the per-browser failure
    /// budget below instead, so one client cannot block every account.
    pub(super) async fn rate(
        &self,
        tx: &mut Transaction<'_, Any>,
        current: i64,
    ) -> Result<bool, RecoveryError> {
        let row = sqlx::query(
            "SELECT request_window,requests FROM rullst_email_login_control WHERE namespace = $1",
        )
        .bind(&self.config.namespace)
        .fetch_one(&mut **tx)
        .await?;
        let window: i64 = row.try_get("request_window")?;
        let count: i64 = row.try_get("requests")?;
        if window < 0 || count < 0 || current < window {
            return Err(RecoveryError::Configuration);
        }
        let (window, count) = if current - window >= 60 {
            (current, 0)
        } else {
            (window, count)
        };
        if count >= 120 {
            return Ok(false);
        }
        sqlx::query("UPDATE rullst_email_login_control SET request_window = $1, requests = $2 WHERE namespace = $3")
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
        let current = advance_clock(now(clock)?, started)?;
        if current >= expiry {
            return Err(RecoveryError::InvalidAction);
        }
        let current = self.observe(tx, current).await?;
        if current >= expiry {
            return Err(RecoveryError::InvalidAction);
        }
        Ok(current)
    }

    pub(super) fn digest(&self, purpose: &str, value: &str) -> String {
        self.store
            .keys
            .digest(purpose, &format!("{}:{value}", self.config.namespace))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn failure_budget_is_per_binding_windowed_and_bounded() {
        let failures = RedemptionFailures::default();
        for _ in 0..FAILURE_LIMIT {
            assert!(!failures.exhausted("attacker", 1_000).unwrap());
            failures.charge("attacker".to_owned(), 1_000).unwrap();
        }
        assert!(failures.exhausted("attacker", 1_000).unwrap());
        assert!(
            failures
                .exhausted("attacker", 1_000 + FAILURE_WINDOW - 1)
                .unwrap()
        );
        assert!(!failures.exhausted("owner", 1_000).unwrap());
        // The window ends, and a clock regression never extends a block.
        assert!(
            !failures
                .exhausted("attacker", 1_000 + FAILURE_WINDOW)
                .unwrap()
        );
        assert!(!failures.exhausted("attacker", 999).unwrap());
        failures
            .charge("attacker".to_owned(), 1_000 + FAILURE_WINDOW)
            .unwrap();
        assert!(
            !failures
                .exhausted("attacker", 1_000 + FAILURE_WINDOW)
                .unwrap()
        );

        for index in 0..=TRACKED_BINDINGS {
            failures.charge(format!("rotated-{index}"), 2_000).unwrap();
        }
        let budgets = failures.0.lock().unwrap();
        assert_eq!(budgets.len(), TRACKED_BINDINGS);
        assert!(budgets.contains_key(&format!("rotated-{TRACKED_BINDINGS}")));
    }
}
