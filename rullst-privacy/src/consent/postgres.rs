//! Shared consent state on one authoritative writable PostgreSQL database.

use super::*;
use sqlx::{PgConnection, PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::time::Duration;

const SCHEMA: &[&str] = &[
    "CREATE SCHEMA rullst_consent",
    "CREATE TABLE rullst_consent.metadata (id INTEGER PRIMARY KEY CHECK(id = 1), schema_version BIGINT NOT NULL CHECK(schema_version = 1), capacity BIGINT NOT NULL CHECK(capacity BETWEEN 1 AND 100000), last_now BIGINT NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE rullst_consent.choices (scope_digest BYTEA PRIMARY KEY CHECK(octet_length(scope_digest) = 32), revision BIGINT NOT NULL CHECK(revision > 0), policy_version TEXT NOT NULL CHECK(octet_length(policy_version) BETWEEN 1 AND 128), choice BIGINT NOT NULL CHECK(choice IN (1,2,3)), changed_at BIGINT NOT NULL CHECK(changed_at >= 0), valid_until BIGINT NOT NULL CHECK(valid_until >= 0))",
];

/// Durable optional-processing choices shared by independent application hosts.
///
/// All hosts must use the same authoritative database, capacity and synchronized
/// clocks. Reads and writes lock the metadata row, persist clock observations and
/// serialize quota/revision checks. Withdrawals and expired choices are never
/// evicted to make room. Only scope digests and bounded choice metadata are stored;
/// these digests are pseudonymous data, not anonymization.
///
/// Operators own role permissions, storage durability, replication fencing and
/// restoration. A stale backup can revive old grants: quiesce optional processing,
/// reconcile withdrawals and rotate purpose versions before resuming. A fresh
/// gate check is required immediately before each processing action.
#[derive(Clone)]
pub struct PostgresConsentStore {
    pool: PgPool,
    capacity: i64,
}

impl PostgresConsentStore {
    /// Explicitly initializes an absent schema or validates existing state.
    /// Concurrent initializers serialize. Existing state is never reset/repaired;
    /// an incompatible capacity or schema fails. Use a deployment role for DDL.
    pub async fn initialize(url: impl Into<String>, capacity: usize) -> Result<Self, ConsentError> {
        Self::open(url.into(), capacity, true).await
    }

    /// Opens existing state without DDL. The runtime role needs schema USAGE and
    /// SELECT/UPDATE on metadata, SELECT/INSERT/UPDATE on choices; no DELETE.
    /// Remote TCP uses certificate/hostname-verified TLS. Errors omit credentials.
    pub async fn connect(url: impl Into<String>, capacity: usize) -> Result<Self, ConsentError> {
        Self::open(url.into(), capacity, false).await
    }

    /// Closes this pool and its clones; independent pools are unaffected.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn open(url: String, capacity: usize, initialize: bool) -> Result<Self, ConsentError> {
        validate_capacity(capacity)?;
        let options = crate::postgres_connection::connection_options(&url)
            .map_err(|_| ConsentError::InvalidConfiguration)?
            .application_name("rullst-consent-v1");
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|connection, _| {
                Box::pin(async move {
                    for statement in [
                        "SET search_path = pg_catalog",
                        "SET synchronous_commit = on",
                        "SET statement_timeout = '5s'",
                        "SET lock_timeout = '5s'",
                        "SET idle_in_transaction_session_timeout = '5s'",
                    ] {
                        sqlx::query(statement).execute(&mut *connection).await?;
                    }
                    Ok(())
                })
            })
            .connect_with(options)
            .await
            .map_err(unavailable)?;
        let store = Self {
            pool,
            capacity: capacity as i64,
        };
        if let Err(error) = store.prepare(initialize).await {
            store.close().await;
            return Err(error);
        }
        Ok(store)
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, ConsentError> {
        self.pool
            .begin_with("BEGIN ISOLATION LEVEL READ COMMITTED READ WRITE")
            .await
            .map_err(unavailable)
    }

    async fn prepare(&self, initialize: bool) -> Result<(), ConsentError> {
        let mut tx = self.begin().await?;
        if initialize {
            sqlx::query("SELECT pg_catalog.pg_advisory_xact_lock(7311456720483292)")
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname = 'rullst_consent')")
                .fetch_one(&mut *tx).await.map_err(unavailable)?;
            if !exists {
                for statement in SCHEMA {
                    sqlx::query(*statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(unavailable)?;
                }
                sqlx::query("INSERT INTO rullst_consent.metadata (id,schema_version,capacity,last_now) VALUES (1,1,$1,0)")
                    .bind(self.capacity).execute(&mut *tx).await.map_err(unavailable)?;
            }
        }
        self.storage_configuration(&mut tx).await?;
        self.metadata(&mut tx).await?;
        self.count(&mut tx).await?;
        tx.commit().await.map_err(unavailable)
    }

    async fn storage_configuration(
        &self,
        connection: &mut PgConnection,
    ) -> Result<(), ConsentError> {
        let durable: bool = sqlx::query_scalar("SELECT pg_catalog.current_setting('fsync') = 'on' AND pg_catalog.current_setting('full_page_writes') = 'on' AND pg_catalog.current_setting('synchronous_commit') IN ('on', 'remote_apply') AND NOT pg_catalog.pg_is_in_recovery()")
            .fetch_one(&mut *connection).await.map_err(unavailable)?;
        let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'rullst_consent' AND c.relname IN ('metadata','choices') AND c.relkind = 'r' AND c.relpersistence = 'p'")
            .fetch_one(connection).await.map_err(unavailable)?;
        if !durable || tables != 2 {
            return Err(ConsentError::StoreConfiguration);
        }
        Ok(())
    }

    async fn metadata(&self, connection: &mut PgConnection) -> Result<i64, ConsentError> {
        let rows: Vec<(i32, i64, i64, i64)> = sqlx::query_as("SELECT id,schema_version,capacity,last_now FROM rullst_consent.metadata LIMIT 2 FOR UPDATE")
            .fetch_all(connection).await.map_err(unavailable)?;
        match rows.as_slice() {
            [(1, 1, capacity, last_now)] if *capacity == self.capacity && *last_now >= 0 => {
                Ok(*last_now)
            }
            _ => Err(ConsentError::StoreConfiguration),
        }
    }

    async fn observe(&self, connection: &mut PgConnection, now: i64) -> Result<(), ConsentError> {
        self.storage_configuration(connection).await?;
        if now < self.metadata(connection).await? {
            return Err(ConsentError::ClockRollback);
        }
        sqlx::query("UPDATE rullst_consent.metadata SET last_now = $1 WHERE id = 1")
            .bind(now)
            .execute(connection)
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn count(&self, connection: &mut PgConnection) -> Result<i64, ConsentError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_consent.choices")
            .fetch_one(connection)
            .await
            .map_err(unavailable)?;
        if !(0..=self.capacity).contains(&count) {
            return Err(ConsentError::StoreConfiguration);
        }
        Ok(count)
    }

    async fn load(
        connection: &mut PgConnection,
        subject: &ConsentSubject,
        purpose_id: &str,
    ) -> Result<ConsentRecord, ConsentError> {
        let absent = ConsentRecord::absent(subject.clone(), purpose_id)?;
        let digest = scope_digest(subject, purpose_id);
        let row: Option<(i64, String, i64, i64, i64)> = sqlx::query_as("SELECT revision,policy_version,choice,changed_at,valid_until FROM rullst_consent.choices WHERE scope_digest = $1")
            .bind(digest.as_ref()).fetch_optional(connection).await.map_err(unavailable)?;
        let Some((revision, version, choice, changed, expiry)) = row else {
            return Ok(absent);
        };
        let choice = match choice {
            1 => ConsentChoice::Granted,
            2 => ConsentChoice::Declined,
            3 => ConsentChoice::Withdrawn,
            _ => return Err(ConsentError::StoreConfiguration),
        };
        let purpose = ConsentPurpose::new(purpose_id, version)
            .map_err(|_| ConsentError::StoreConfiguration)?;
        ConsentRecord::from_stored(
            subject.clone(),
            purpose,
            u64::try_from(revision).map_err(|_| ConsentError::StoreConfiguration)?,
            choice,
            changed,
            expiry,
        )
    }
}

impl ConsentStore for PostgresConsentStore {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::SharedDurable
    }

    async fn read(
        &self,
        subject: &ConsentSubject,
        purpose_id: &str,
        now: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        let mut tx = self.begin().await?;
        self.observe(&mut tx, now).await?;
        let record = Self::load(&mut tx, subject, purpose_id).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(record)
    }

    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        let mut tx = self.begin().await?;
        self.observe(&mut tx, update.now()).await?;
        let current = Self::load(&mut tx, update.subject(), update.purpose().id()).await?;
        let next = update.apply_to(&current)?;
        if current.revision() == 0 && self.count(&mut tx).await? >= self.capacity {
            return Err(ConsentError::StoreCapacity);
        }
        let choice = match next.choice() {
            ConsentChoice::Granted => 1_i64,
            ConsentChoice::Declined => 2,
            ConsentChoice::Withdrawn => 3,
            ConsentChoice::Unset => return Err(ConsentError::StoreConfiguration),
        };
        let digest = scope_digest(update.subject(), update.purpose().id());
        sqlx::query("INSERT INTO rullst_consent.choices (scope_digest,revision,policy_version,choice,changed_at,valid_until) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT(scope_digest) DO UPDATE SET revision=excluded.revision,policy_version=excluded.policy_version,choice=excluded.choice,changed_at=excluded.changed_at,valid_until=excluded.valid_until")
            .bind(digest.as_ref()).bind(next.revision() as i64).bind(next.version()).bind(choice).bind(next.changed_at()).bind(next.valid_until())
            .execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(next)
    }
}

fn unavailable(_: sqlx::Error) -> ConsentError {
    ConsentError::StoreUnavailable
}
