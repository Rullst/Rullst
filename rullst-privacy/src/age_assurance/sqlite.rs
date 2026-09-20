//! Bounded replay state for processes sharing one trusted local database file.

use super::{AgeError, ReplayDurability, ReplayStore, replay::validate_claim};
use ring::digest::{Context, SHA256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{SqliteConnection, SqlitePool};
use std::{path::Path, time::Duration};

const SCHEMA_VERSION: i64 = 1;
const SCHEMA: &[&str] = &[
    "CREATE TABLE rullst_age_replay_meta (id INTEGER PRIMARY KEY CHECK(id = 1), schema_version INTEGER NOT NULL, capacity INTEGER NOT NULL CHECK(capacity BETWEEN 1 AND 100000), last_now INTEGER NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE rullst_age_replay_claims (nonce_digest BLOB PRIMARY KEY NOT NULL CHECK(length(nonce_digest) = 32), expires_at INTEGER NOT NULL CHECK(expires_at > 0))",
    "CREATE INDEX rullst_age_replay_expiry ON rullst_age_replay_claims(expires_at)",
];

/// Durable atomic claims across processes sharing one local SQLite file.
///
/// Stores nonce digests and expiry only. WAL/full synchronization, serialized
/// writes, a persisted quota and clock high-water mark prevent concurrent or
/// post-restart replay under SQLite's documented durability assumptions.
///
/// The operator owns a trusted directory, permissions, synchronized server time
/// and reliable local storage. Do not use network filesystems. Restoring a stale
/// backup can restore consumed proofs: quiesce verification and change the
/// policy/key epoch to invalidate every outstanding challenge before resuming.
/// Logical deletion does not promise physical erasure from pages or backups.
#[derive(Clone)]
pub struct SqliteReplayStore {
    pool: SqlitePool,
    capacity: i64,
}

impl SqliteReplayStore {
    /// Opens a file in an existing operator-owned directory; does not accept a
    /// database URL, in-memory database or caller-configured pool. Capacity is
    /// 1..=100,000 and must agree across every opener. Opening is startup work.
    pub async fn open(path: impl AsRef<Path>, capacity: usize) -> Result<Self, AgeError> {
        if !(1..=100_000).contains(&capacity) {
            return Err(AgeError::InvalidConfiguration);
        }
        let path = path.as_ref();
        let filename = path
            .file_name()
            .filter(|name| *name != ":memory:")
            .ok_or(AgeError::InvalidConfiguration)?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let path = parent
            .canonicalize()
            .map_err(|_| AgeError::StoreUnavailable)?
            .join(filename);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(AgeError::InvalidConfiguration);
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(AgeError::StoreUnavailable);
            }
            _ => {}
        }
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        let store = Self {
            pool,
            capacity: capacity as i64,
        };
        if let Err(error) = store.prepare().await {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    /// Closes this pool, including its clones. Other independently opened
    /// processes/pools remain usable; subsequent claims here fail closed.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn prepare(&self) -> Result<(), AgeError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name IN ('rullst_age_replay_meta', 'rullst_age_replay_claims')")
            .fetch_one(&mut *tx).await.map_err(|_| AgeError::StoreUnavailable)?;
        match count {
            0 => {
                for statement in SCHEMA {
                    sqlx::query(*statement)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| AgeError::StoreUnavailable)?;
                }
                sqlx::query("INSERT INTO rullst_age_replay_meta (id, schema_version, capacity, last_now) VALUES (1, ?, ?, 0)")
                    .bind(SCHEMA_VERSION).bind(self.capacity).execute(&mut *tx).await
                    .map_err(|_| AgeError::StoreUnavailable)?;
            }
            2 => {}
            // Never recreate half of an existing replay schema.
            _ => return Err(AgeError::StoreConfiguration),
        }
        self.metadata(&mut tx).await?;
        self.count(&mut tx).await?;
        tx.commit().await.map_err(|_| AgeError::StoreUnavailable)
    }

    async fn metadata(&self, connection: &mut SqliteConnection) -> Result<i64, AgeError> {
        let row: Option<(i64, i64, i64)> = sqlx::query_as(
            "SELECT schema_version, capacity, last_now FROM rullst_age_replay_meta WHERE id = 1",
        )
        .fetch_optional(connection)
        .await
        .map_err(|_| AgeError::StoreUnavailable)?;
        match row {
            Some((SCHEMA_VERSION, capacity, last_now))
                if capacity == self.capacity && last_now >= 0 =>
            {
                Ok(last_now)
            }
            _ => Err(AgeError::StoreConfiguration),
        }
    }

    async fn count(&self, connection: &mut SqliteConnection) -> Result<i64, AgeError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_age_replay_claims")
            .fetch_one(connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        if !(0..=self.capacity).contains(&count) {
            return Err(AgeError::StoreConfiguration);
        }
        Ok(count)
    }

    async fn claim_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        nonce: [u8; 32],
        expires_at: i64,
        now: i64,
    ) -> Result<bool, AgeError> {
        if now < self.metadata(connection).await? {
            return Err(AgeError::ClockRollback);
        }
        sqlx::query("UPDATE rullst_age_replay_meta SET last_now = ? WHERE id = 1")
            .bind(now)
            .execute(&mut *connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        sqlx::query("DELETE FROM rullst_age_replay_claims WHERE expires_at <= ?")
            .bind(now)
            .execute(&mut *connection)
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        let mut hash = Context::new(&SHA256);
        hash.update(b"rullst.age-replay.v1\0");
        hash.update(&nonce);
        let digest = hash.finish();
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM rullst_age_replay_claims WHERE nonce_digest = ?)",
        )
        .bind(digest.as_ref())
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| AgeError::StoreUnavailable)?;
        if exists {
            return Ok(false);
        }
        if self.count(connection).await? >= self.capacity {
            return Err(AgeError::StoreCapacity);
        }
        sqlx::query(
            "INSERT INTO rullst_age_replay_claims (nonce_digest, expires_at) VALUES (?, ?)",
        )
        .bind(digest.as_ref())
        .bind(expires_at)
        .execute(connection)
        .await
        .map_err(|_| AgeError::StoreUnavailable)?;
        Ok(true)
    }
}

impl ReplayStore for SqliteReplayStore {
    fn durability(&self) -> ReplayDurability {
        ReplayDurability::SharedDurable
    }

    async fn claim(&self, nonce: [u8; 32], expires_at: i64, now: i64) -> Result<bool, AgeError> {
        validate_claim(expires_at, now)?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| AgeError::StoreUnavailable)?;
        match self
            .claim_in_transaction(&mut tx, nonce, expires_at, now)
            .await
        {
            Ok(claimed) => {
                tx.commit().await.map_err(|_| AgeError::StoreUnavailable)?;
                Ok(claimed)
            }
            Err(error) => {
                tx.rollback()
                    .await
                    .map_err(|_| AgeError::StoreUnavailable)?;
                Err(error)
            }
        }
    }
}
