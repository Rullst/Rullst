//! Durable local compare-and-swap storage for encrypted token generations.

use super::{
    EncryptedTokenSnapshot, RefreshableTokenState, TokenSnapshotBinding, TokenSnapshotError,
    TokenSnapshotKey,
};
use sqlx::{SqliteConnection, SqlitePool};
use std::fmt;

mod storage;
use storage::*;

const MAX_TOKEN_STORE_ENTRIES: usize = 1_000_000;

/// Typed failures that omit database paths, account identifiers, and tokens.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum TokenStoreError {
    #[error("invalid token store configuration: {0}")]
    InvalidConfiguration(&'static str),
    #[error("token snapshot already exists")]
    AlreadyExists,
    #[error("token snapshot was not found")]
    NotFound,
    #[error("token snapshot generation changed concurrently")]
    GenerationConflict,
    #[error("token snapshot capacity is exhausted")]
    CapacityExceeded,
    #[error("token snapshot cryptographic validation failed: {0}")]
    Snapshot(#[from] TokenSnapshotError),
    #[error("token snapshot storage operation failed: {0}")]
    StorageUnavailable(&'static str),
    #[error("token snapshot storage is corrupt: {0}")]
    CorruptStorage(&'static str),
}

/// Non-secret metadata for one encrypted token generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenStoreMetadata {
    generation: u64,
    key_id: String,
}

impl TokenStoreMetadata {
    /// Monotonic provider-token generation stored for this binding.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Non-secret encryption-key identifier needed to select a rotation key.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

/// Bounded aggregate metadata for one local token database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenStoreSnapshot {
    entries: usize,
    max_entries: usize,
}

impl TokenStoreSnapshot {
    #[must_use]
    pub const fn entries(self) -> usize {
        self.entries
    }

    #[must_use]
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// File-backed encrypted refresh-token state shared by local processes.
///
/// Rows contain only an opaque binding digest, generation, key ID, and the
/// authenticated ciphertext produced by [`EncryptedTokenSnapshot`]. Initial
/// insertion and replacement use `BEGIN IMMEDIATE`; replacement succeeds only
/// when the stored generation is exactly the caller's expected generation.
/// This prevents a stale local writer from overwriting a newer token rotation.
///
/// The store does not serialize calls to a remote provider. Applications still
/// own refresh leases, retry/backoff, authorization, key custody, backup,
/// multi-host replication, and reconciliation after a losing provider call.
#[derive(Clone)]
pub struct SqliteTokenSnapshotStore {
    pool: SqlitePool,
    max_entries: usize,
}

impl SqliteTokenSnapshotStore {
    /// Opens or creates a file-backed store with a persisted entry ceiling.
    pub async fn connect(
        database_url: impl Into<String>,
        max_entries: usize,
    ) -> Result<Self, TokenStoreError> {
        if !(1..=MAX_TOKEN_STORE_ENTRIES).contains(&max_entries) {
            return Err(TokenStoreError::InvalidConfiguration("entry limit"));
        }
        let database_url = database_url.into();
        let pool = connect_pool(&database_url).await?;
        if let Err(error) = prepare_schema(&pool, max_entries).await {
            pool.close().await;
            return Err(error);
        }
        Ok(Self { pool, max_entries })
    }

    /// Inserts generation zero if this trusted provider/account binding is absent.
    pub async fn insert_initial(
        &self,
        binding: &TokenSnapshotBinding,
        state: &RefreshableTokenState,
        key: &TokenSnapshotKey,
    ) -> Result<TokenStoreMetadata, TokenStoreError> {
        if state.generation() != 0 {
            return Err(TokenStoreError::InvalidConfiguration(
                "initial generation must be zero",
            ));
        }
        let envelope = EncryptedTokenSnapshot::seal(state, key, binding)?;
        let subject_key = subject_key(binding);
        let mut connection = begin_write(&self.pool, "begin initial insert").await?;
        let result = self
            .insert_initial_in_transaction(&mut connection, &subject_key, &envelope, key)
            .await;
        finish(connection, result, "finish initial insert").await
    }

    /// Loads, authenticates, and revalidates one encrypted generation.
    pub async fn load(
        &self,
        binding: &TokenSnapshotBinding,
        key: &TokenSnapshotKey,
    ) -> Result<Option<RefreshableTokenState>, TokenStoreError> {
        let row: Option<(i64, String, String)> = sqlx::query_as(
            "SELECT generation, key_id, envelope FROM rullst_connect_token_snapshots WHERE subject_key = ?",
        )
        .bind(subject_key(binding))
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| unavailable("load token snapshot"))?;
        row.map(|row| decode_row(row, binding, key)).transpose()
    }

    /// Returns non-secret generation/key metadata without decrypting token state.
    pub async fn metadata(
        &self,
        binding: &TokenSnapshotBinding,
    ) -> Result<Option<TokenStoreMetadata>, TokenStoreError> {
        let row: Option<(i64, String, String)> = sqlx::query_as(
            "SELECT generation, key_id, envelope FROM rullst_connect_token_snapshots WHERE subject_key = ?",
        )
        .bind(subject_key(binding))
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| unavailable("inspect token snapshot"))?;
        row.map(decode_metadata).transpose()
    }

    /// Atomically writes exactly the successor to `expected_generation`.
    pub async fn compare_and_swap(
        &self,
        binding: &TokenSnapshotBinding,
        expected_generation: u64,
        replacement: &RefreshableTokenState,
        key: &TokenSnapshotKey,
    ) -> Result<TokenStoreMetadata, TokenStoreError> {
        let next_generation = expected_generation
            .checked_add(1)
            .ok_or(TokenStoreError::InvalidConfiguration("token generation"))?;
        if replacement.generation() != next_generation {
            return Err(TokenStoreError::InvalidConfiguration(
                "replacement must be the next generation",
            ));
        }
        let expected = as_i64(expected_generation, "expected generation")?;
        let next = as_i64(next_generation, "replacement generation")?;
        let envelope = EncryptedTokenSnapshot::seal(replacement, key, binding)?;
        let subject_key = subject_key(binding);
        let mut connection = begin_write(&self.pool, "begin generation update").await?;
        let result = async {
            let changed = sqlx::query("UPDATE rullst_connect_token_snapshots SET generation = ?, key_id = ?, envelope = ? WHERE subject_key = ? AND generation = ?")
                .bind(next)
                .bind(key.key_id())
                .bind(envelope.as_str())
                .bind(&subject_key)
                .bind(expected)
                .execute(&mut *connection)
                .await
                .map_err(|_| unavailable("replace token generation"))?;
            if changed.rows_affected() != 1 {
                return classify_conflict(&mut connection, &subject_key).await;
            }
            Ok(TokenStoreMetadata {
                generation: next_generation,
                key_id: key.key_id().to_string(),
            })
        }
        .await;
        finish(connection, result, "finish generation update").await
    }

    /// Deletes a local snapshot only if the caller observed its exact generation.
    pub async fn delete_if_generation(
        &self,
        binding: &TokenSnapshotBinding,
        expected_generation: u64,
    ) -> Result<(), TokenStoreError> {
        let expected = as_i64(expected_generation, "expected generation")?;
        let subject_key = subject_key(binding);
        let mut connection = begin_write(&self.pool, "begin token deletion").await?;
        let result = async {
            let changed = sqlx::query("DELETE FROM rullst_connect_token_snapshots WHERE subject_key = ? AND generation = ?")
                .bind(&subject_key)
                .bind(expected)
                .execute(&mut *connection)
                .await
                .map_err(|_| unavailable("delete token snapshot"))?;
            if changed.rows_affected() != 1 {
                return classify_conflict(&mut connection, &subject_key).await;
            }
            Ok(())
        }
        .await;
        finish(connection, result, "finish token deletion").await
    }

    /// Returns the current row count and persisted local ceiling.
    pub async fn snapshot(&self) -> Result<TokenStoreSnapshot, TokenStoreError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_connect_token_snapshots")
            .fetch_one(&self.pool)
            .await
            .map_err(|_| unavailable("count token snapshots"))?;
        let entries = usize::try_from(row.0).map_err(|_| corrupt("entry count"))?;
        if entries > self.max_entries {
            return Err(corrupt("entry count exceeds configured limit"));
        }
        Ok(TokenStoreSnapshot {
            entries,
            max_entries: self.max_entries,
        })
    }

    /// Gracefully closes pooled handles before backup or file replacement.
    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn insert_initial_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        subject_key: &str,
        envelope: &EncryptedTokenSnapshot,
        key: &TokenSnapshotKey,
    ) -> Result<TokenStoreMetadata, TokenStoreError> {
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM rullst_connect_token_snapshots WHERE subject_key = ?")
                .bind(subject_key)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| unavailable("inspect initial token snapshot"))?;
        if existing.is_some() {
            return Err(TokenStoreError::AlreadyExists);
        }
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_connect_token_snapshots")
            .fetch_one(&mut *connection)
            .await
            .map_err(|_| unavailable("count token snapshots"))?;
        let count = usize::try_from(count.0).map_err(|_| corrupt("entry count"))?;
        if count >= self.max_entries {
            return Err(TokenStoreError::CapacityExceeded);
        }
        sqlx::query("INSERT INTO rullst_connect_token_snapshots (subject_key, generation, key_id, envelope) VALUES (?, 0, ?, ?)")
            .bind(subject_key)
            .bind(key.key_id())
            .bind(envelope.as_str())
            .execute(&mut *connection)
            .await
            .map_err(|_| unavailable("persist initial token snapshot"))?;
        Ok(TokenStoreMetadata {
            generation: 0,
            key_id: key.key_id().to_string(),
        })
    }
}

impl fmt::Debug for SqliteTokenSnapshotStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqliteTokenSnapshotStore")
            .field("database", &"[REDACTED]")
            .field("max_entries", &self.max_entries)
            .finish()
    }
}

fn decode_row(
    row: (i64, String, String),
    binding: &TokenSnapshotBinding,
    key: &TokenSnapshotKey,
) -> Result<RefreshableTokenState, TokenStoreError> {
    let metadata = decode_metadata((row.0, row.1, row.2.clone()))?;
    if metadata.key_id != key.key_id() {
        return Err(TokenSnapshotError::KeyIdMismatch.into());
    }
    let envelope = EncryptedTokenSnapshot::try_from_envelope(row.2)?;
    let state = envelope.open(key, binding)?;
    if state.generation() != metadata.generation {
        return Err(corrupt("row and encrypted generations disagree"));
    }
    Ok(state)
}

fn decode_metadata(row: (i64, String, String)) -> Result<TokenStoreMetadata, TokenStoreError> {
    let generation = u64::try_from(row.0).map_err(|_| corrupt("token generation"))?;
    let envelope = EncryptedTokenSnapshot::try_from_envelope(row.2)?;
    if envelope.key_id()? != row.1 {
        return Err(corrupt("row and encrypted key identifiers disagree"));
    }
    Ok(TokenStoreMetadata {
        generation,
        key_id: row.1,
    })
}

async fn classify_conflict<T>(
    connection: &mut SqliteConnection,
    subject_key: &str,
) -> Result<T, TokenStoreError> {
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM rullst_connect_token_snapshots WHERE subject_key = ?")
            .bind(subject_key)
            .fetch_optional(&mut *connection)
            .await
            .map_err(|_| unavailable("classify token generation conflict"))?;
    match exists {
        Some(_) => Err(TokenStoreError::GenerationConflict),
        None => Err(TokenStoreError::NotFound),
    }
}

#[cfg(test)]
#[path = "sqlite/tests.rs"]
mod tests;
