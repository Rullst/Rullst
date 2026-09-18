//! Atomic event completion and caller-owned SQL mutations, without preclaims.

mod schema;
mod scope;
pub use scope::StripeInboxScope;

use crate::{SqlWebhookBackend, StripeSubscriptionEvent};
use rullst_orm::{RullstPool, db::Transaction, sqlx};
use std::{future::Future, pin::Pin};

/// Redacted inbox failures. A conflict never grants permission to apply again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StripeInboxError {
    #[error("invalid Stripe inbox configuration")]
    InvalidConfiguration,
    #[error("Stripe inbox configuration is missing or changed")]
    ConfigurationMismatch,
    #[error("mock Stripe events are not accepted by a durable inbox")]
    MockEvent,
    #[error("Stripe event does not match the configured account scope or mode")]
    ScopeMismatch,
    #[error("Stripe event identity was reused with conflicting mutation content")]
    EventConflict,
    #[error("Stripe inbox capacity is exhausted; reconciliation and retention are required")]
    CapacityExhausted,
    #[error("Stripe inbox storage is unavailable or inconsistent")]
    StorageUnavailable,
    #[error("Stripe event domain mutation was rejected")]
    MutationRejected,
    #[error("Stripe inbox commit outcome is uncertain; retry the same verified event")]
    CommitUncertain,
}

/// Durable result selected after authorization and domain processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StripeInboxOutcome {
    /// Domain SQL was applied in the inbox transaction.
    Applied,
    /// The application deliberately recorded a terminal no-op.
    Ignored,
}

impl StripeInboxOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Ignored => "ignored",
        }
    }

    fn parse(value: &str) -> Result<Self, StripeInboxError> {
        match value {
            "applied" => Ok(Self::Applied),
            "ignored" => Ok(Self::Ignored),
            _ => Err(StripeInboxError::StorageUnavailable),
        }
    }
}

/// A committed result, either newly processed or recovered from an exact retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StripeInboxResult {
    outcome: StripeInboxOutcome,
    duplicate: bool,
}

impl StripeInboxResult {
    pub fn outcome(self) -> StripeInboxOutcome {
        self.outcome
    }
    /// True means the callback was not invoked and the prior outcome was retained.
    pub fn is_duplicate(self) -> bool {
        self.duplicate
    }
}

/// Per-scope durable inbox using the ORM's selected relational pool type.
///
/// Admission and the callback share one transaction. The callback must validate
/// persisted customer/owner bindings and ordering, and use this transaction for
/// every domain write. External effects require a transactional outbox. Entries
/// never expire automatically: capacity exhaustion requires a reviewed retention
/// and provider reconciliation policy, not dropping rows merely to admit traffic.
#[derive(Clone)]
pub struct SqlStripeEventInbox {
    pool: RullstPool,
    backend: SqlWebhookBackend,
    scope: StripeInboxScope,
    capacity: i64,
}

impl std::fmt::Debug for SqlStripeEventInbox {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlStripeEventInbox")
            .field("backend", &self.backend)
            .field("scope", &self.scope)
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl SqlStripeEventInbox {
    /// Wraps a caller-owned pool; dialect must match that pool. No schema is installed.
    pub fn new(
        pool: RullstPool,
        backend: SqlWebhookBackend,
        scope: StripeInboxScope,
        capacity: usize,
    ) -> Result<Self, StripeInboxError> {
        if !(1..=1_000_000).contains(&capacity) {
            return Err(StripeInboxError::InvalidConfiguration);
        }
        Ok(Self {
            pool,
            backend,
            scope,
            capacity: capacity as i64,
        })
    }

    /// Explicit setup/migration convenience; never called by the request path.
    /// Existing per-scope capacity must match. Partial DDL can be safely retried.
    pub async fn prepare_schema(&self) -> Result<(), StripeInboxError> {
        for ddl in [
            schema::config_schema(self.backend),
            schema::event_schema(self.backend),
        ] {
            sqlx::query(ddl)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let result = async {
            sqlx::query(schema::insert_config(self.backend))
                .bind(&self.scope.hash)
                .bind(self.capacity)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            self.lock_and_validate(&mut tx).await
        }
        .await;
        finish(tx, result).await
    }

    /// Atomically commits a verified event and its SQL mutation.
    ///
    /// Exact retries return the committed outcome without calling `mutate`.
    /// Conflicting content, capacity exhaustion and scope/mock failures do not
    /// invoke it. Do not put an earlier replay-claim middleware in front of this
    /// path. A database failure or cancellation rolls back uncommitted work;
    /// an uncertain commit is resolved by retrying the same stable event.
    pub async fn process<F>(
        &self,
        event: &StripeSubscriptionEvent,
        mutate: F,
    ) -> Result<StripeInboxResult, StripeInboxError>
    where
        F: for<'a> FnOnce(
            &'a mut Transaction<'_>,
            &'a StripeSubscriptionEvent,
        ) -> Pin<
            Box<dyn Future<Output = Result<StripeInboxOutcome, StripeInboxError>> + Send + 'a>,
        >,
    {
        self.scope.verify(event)?;
        let event_hash = hex::encode(ring::digest::digest(
            &ring::digest::SHA256,
            event.event_id().as_bytes(),
        ));
        let mutation_hash = hex::encode(event.mutation_digest());
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let result = async {
            self.lock_and_validate(&mut tx).await?;
            let existing = sqlx::query_as::<_, (String, String)>(schema::existing(self.backend))
                .bind(&self.scope.hash)
                .bind(&event_hash)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?;
            if let Some((digest, outcome)) = existing {
                if digest != mutation_hash {
                    return Err(StripeInboxError::EventConflict);
                }
                return Ok(StripeInboxResult {
                    outcome: StripeInboxOutcome::parse(&outcome)?,
                    duplicate: true,
                });
            }
            let count = sqlx::query_scalar::<_, i64>(schema::count(self.backend))
                .bind(&self.scope.hash)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
            if count < 0 {
                return Err(StripeInboxError::StorageUnavailable);
            }
            if count >= self.capacity {
                return Err(StripeInboxError::CapacityExhausted);
            }
            let outcome = mutate(&mut tx, event).await?;
            sqlx::query(schema::insert_event(self.backend))
                .bind(&self.scope.hash)
                .bind(&event_hash)
                .bind(&mutation_hash)
                .bind(outcome.as_str())
                .bind(event.created_at())
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            Ok(StripeInboxResult {
                outcome,
                duplicate: false,
            })
        }
        .await;
        finish(tx, result).await
    }

    async fn lock_and_validate(&self, tx: &mut Transaction<'_>) -> Result<(), StripeInboxError> {
        sqlx::query(schema::lock_scope(self.backend))
            .bind(&self.scope.hash)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
        let capacity = sqlx::query_scalar::<_, i64>(schema::capacity(self.backend))
            .bind(&self.scope.hash)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage)?;
        if capacity != Some(self.capacity) {
            return Err(StripeInboxError::ConfigurationMismatch);
        }
        Ok(())
    }
}

fn storage(_: sqlx::Error) -> StripeInboxError {
    StripeInboxError::StorageUnavailable
}

async fn finish<T>(
    tx: Transaction<'_>,
    result: Result<T, StripeInboxError>,
) -> Result<T, StripeInboxError> {
    match result {
        Ok(value) => {
            tx.commit()
                .await
                .map_err(|_| StripeInboxError::CommitUncertain)?;
            Ok(value)
        }
        Err(error) => {
            tx.rollback().await.map_err(storage)?;
            Err(error)
        }
    }
}
