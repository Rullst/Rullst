//! Durable shared SQLite revocation state for application-issued JWTs.

use super::quota::must_widen;
use super::{
    ApplicationJwtClaims, AsyncJwtRevocationStore, JwtError, JwtRevocationMode, unix_time,
    valid_identifier, valid_identity,
};
use schema::{
    MAX_REVOCATION_ENTRIES, prepare_schema, reject_existing_unsafe_target, volatile_database_url,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Sqlite, SqliteConnection, SqlitePool, Transaction};
use std::str::FromStr;
use std::time::Duration;

mod schema;

/// Current bounded counts for one durable JWT revocation database.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqliteJwtRevocationSnapshot {
    token_revocations: usize,
    subject_revocations: usize,
    max_entries: usize,
}

impl SqliteJwtRevocationSnapshot {
    #[must_use]
    pub const fn token_revocations(self) -> usize {
        self.token_revocations
    }

    #[must_use]
    pub const fn subject_revocations(self) -> usize {
        self.subject_revocations
    }

    #[must_use]
    pub const fn total_entries(self) -> usize {
        self.token_revocations + self.subject_revocations
    }

    #[must_use]
    pub const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// File-backed, bounded JWT revocation state shared by local processes.
///
/// Mutations use `BEGIN IMMEDIATE`, token expiry is pruned before capacity is
/// assessed, and the configured quota is persisted so another process cannot
/// silently open the same database with a different limit. The host owns file
/// permissions, backup, availability and multi-host replication.
///
/// Token rows may use at most three quarters of `max_entries` and 64 active
/// rows per subject. Beyond either bound, `revoke_token` records a subject
/// cutoff that rejects every token of that subject issued no later than the
/// revoked one, so one principal cannot exhaust the quota for others. Subject
/// rows are never pruned; size `max_entries` for the subjects that may revoke.
/// `connect` adds the nullable `subject` and `revoked_through_iat` columns to
/// an older file; releases without this change ignore subject cutoffs.
#[derive(Clone)]
pub struct SqliteJwtRevocationStore {
    pool: SqlitePool,
    max_entries: usize,
}

impl SqliteJwtRevocationStore {
    /// Opens or creates a file-backed revocation database.
    pub async fn connect(
        database_url: impl Into<String>,
        max_entries: usize,
    ) -> Result<Self, JwtError> {
        if !(1..=MAX_REVOCATION_ENTRIES).contains(&max_entries) {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation max_entries",
            ));
        }
        let database_url = database_url.into();
        if !database_url.starts_with("sqlite:") {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation database URL",
            ));
        }
        let options = SqliteConnectOptions::from_str(&database_url)
            .map_err(|_| backend_error("parse SQLite revocation database URL"))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        if volatile_database_url(&database_url, options.get_filename()) {
            return Err(JwtError::InvalidConfiguration(
                "SQLite revocation database must be file-backed",
            ));
        }
        reject_existing_unsafe_target(options.get_filename())?;
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(|_| backend_error("connect SQLite revocation database"))?;
        if let Err(error) = prepare_schema(&pool, max_entries).await {
            pool.close().await;
            return Err(error);
        }
        Ok(Self { pool, max_entries })
    }

    /// Persists one token identifier until the token expires, or widens to a
    /// subject cutoff when the subject or the token share is at its quota.
    pub async fn revoke_token(&self, claims: &ApplicationJwtClaims) -> Result<(), JwtError> {
        if !valid_identifier(&claims.jti, 64) {
            return Err(JwtError::InvalidConfiguration("jti"));
        }
        if !valid_identity(&claims.sub) {
            return Err(JwtError::InvalidConfiguration("subject"));
        }
        let now = unix_time()?;
        if claims.exp <= now {
            return Ok(());
        }
        let mut connection = self.begin_write("begin token revocation").await?;
        let result = self
            .revoke_token_in_transaction(&mut connection, claims, now)
            .await;
        finish(connection, result, "finish token revocation").await
    }

    /// Rejects subject tokens below a monotonic session version.
    pub async fn revoke_subject_before(
        &self,
        subject: impl Into<String>,
        minimum_session_version: u64,
    ) -> Result<(), JwtError> {
        let subject = subject.into();
        if !valid_identity(&subject) || minimum_session_version == 0 {
            return Err(JwtError::InvalidConfiguration("subject revocation"));
        }
        let minimum_session_version = i64::try_from(minimum_session_version)
            .map_err(|_| JwtError::InvalidConfiguration("subject revocation"))?;
        let now = unix_time()?;
        let mut connection = self.begin_write("begin subject revocation").await?;
        let result = self
            .revoke_subject_in_transaction(&mut connection, &subject, minimum_session_version, now)
            .await;
        finish(connection, result, "finish subject revocation").await
    }

    /// Returns active token and subject counts after pruning expired tokens.
    pub async fn snapshot(&self) -> Result<SqliteJwtRevocationSnapshot, JwtError> {
        let now = unix_time()?;
        let mut connection = self.begin_write("begin revocation snapshot").await?;
        let result = async {
            prune_expired(&mut connection, now).await?;
            let (token_revocations, subject_revocations) = counts(&mut connection).await?;
            Ok(SqliteJwtRevocationSnapshot {
                token_revocations,
                subject_revocations,
                max_entries: self.max_entries,
            })
        }
        .await;
        finish(connection, result, "finish revocation snapshot").await
    }

    /// Gracefully closes all pooled connections, useful before rotating a file.
    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn begin_write(
        &self,
        operation: &'static str,
    ) -> Result<Transaction<'static, Sqlite>, JwtError> {
        self.pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| backend_error(operation))
    }

    async fn revoke_token_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        claims: &ApplicationJwtClaims,
        now: u64,
    ) -> Result<(), JwtError> {
        prune_expired(connection, now).await?;
        let expires_at = i64::try_from(claims.exp)
            .map_err(|_| JwtError::InvalidConfiguration("token expiry"))?;
        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT expires_at FROM rullst_auth_jwt_tokens WHERE jti = ?")
                .bind(&claims.jti)
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| backend_error("lookup token revocation"))?;
        if existing.is_none() {
            let (tokens, subjects) = counts(connection).await?;
            let (subject_tokens,): (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM rullst_auth_jwt_tokens WHERE subject = ?")
                    .bind(&claims.sub)
                    .fetch_one(&mut *connection)
                    .await
                    .map_err(|_| backend_error("count subject token revocations"))?;
            let subject_tokens = usize::try_from(subject_tokens)
                .map_err(|_| backend_error("validate subject token count"))?;
            if must_widen(subject_tokens, tokens, subjects, self.max_entries) {
                let issued_at = i64::try_from(claims.iat)
                    .map_err(|_| JwtError::InvalidConfiguration("token issue time"))?;
                return self
                    .record_subject_revocation(connection, &claims.sub, 1, issued_at)
                    .await;
            }
        }
        sqlx::query("INSERT INTO rullst_auth_jwt_tokens (jti, expires_at, subject) VALUES (?, ?, ?) ON CONFLICT(jti) DO UPDATE SET expires_at = MAX(expires_at, excluded.expires_at), subject = COALESCE(subject, excluded.subject)")
            .bind(&claims.jti)
            .bind(expires_at)
            .bind(&claims.sub)
            .execute(&mut *connection)
            .await
            .map_err(|_| backend_error("persist token revocation"))?;
        Ok(())
    }

    async fn revoke_subject_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        subject: &str,
        minimum_session_version: i64,
        now: u64,
    ) -> Result<(), JwtError> {
        prune_expired(connection, now).await?;
        self.record_subject_revocation(connection, subject, minimum_session_version, 0)
            .await
    }

    /// Raises a subject's minimum session version and issue-time cutoff.
    async fn record_subject_revocation(
        &self,
        connection: &mut SqliteConnection,
        subject: &str,
        minimum_session_version: i64,
        revoked_through_iat: i64,
    ) -> Result<(), JwtError> {
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT minimum_session_version FROM rullst_auth_jwt_subjects WHERE subject = ?",
        )
        .bind(subject)
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| backend_error("lookup subject revocation"))?;
        if existing.is_none() {
            ensure_capacity(connection, self.max_entries).await?;
        }
        sqlx::query("INSERT INTO rullst_auth_jwt_subjects (subject, minimum_session_version, revoked_through_iat) VALUES (?, ?, ?) ON CONFLICT(subject) DO UPDATE SET minimum_session_version = MAX(minimum_session_version, excluded.minimum_session_version), revoked_through_iat = MAX(revoked_through_iat, excluded.revoked_through_iat)")
            .bind(subject)
            .bind(minimum_session_version)
            .bind(revoked_through_iat)
            .execute(&mut *connection)
            .await
            .map_err(|_| backend_error("persist subject revocation"))?;
        Ok(())
    }
}

impl AsyncJwtRevocationStore for SqliteJwtRevocationStore {
    fn mode(&self) -> JwtRevocationMode {
        JwtRevocationMode::Shared
    }

    async fn is_revoked(&self, claims: &ApplicationJwtClaims, now: u64) -> Result<bool, JwtError> {
        let now = i64::try_from(now).map_err(|_| JwtError::InvalidSystemTime)?;
        let token: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM rullst_auth_jwt_tokens WHERE jti = ? AND expires_at > ?")
                .bind(&claims.jti)
                .bind(now)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| backend_error("read token revocation"))?;
        if token.is_some() {
            return Ok(true);
        }
        let subject: Option<(i64, i64)> = sqlx::query_as(
            "SELECT minimum_session_version, revoked_through_iat FROM rullst_auth_jwt_subjects WHERE subject = ?",
        )
        .bind(&claims.sub)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| backend_error("read subject revocation"))?;
        match subject {
            Some((minimum, cutoff)) => {
                let minimum = u64::try_from(minimum)
                    .map_err(|_| backend_error("validate subject revocation"))?;
                let cutoff = u64::try_from(cutoff)
                    .map_err(|_| backend_error("validate subject revocation"))?;
                Ok(claims.session_version < minimum || (cutoff > 0 && claims.iat <= cutoff))
            }
            None => Ok(false),
        }
    }
}

async fn prune_expired(connection: &mut SqliteConnection, now: u64) -> Result<(), JwtError> {
    let now = i64::try_from(now).map_err(|_| JwtError::InvalidSystemTime)?;
    sqlx::query("DELETE FROM rullst_auth_jwt_tokens WHERE expires_at <= ?")
        .bind(now)
        .execute(&mut *connection)
        .await
        .map_err(|_| backend_error("prune expired token revocations"))?;
    Ok(())
}

async fn ensure_capacity(
    connection: &mut SqliteConnection,
    max_entries: usize,
) -> Result<(), JwtError> {
    let (tokens, subjects) = counts(connection).await?;
    let total = tokens
        .checked_add(subjects)
        .ok_or(JwtError::RevocationStoreCapacity)?;
    if total >= max_entries {
        return Err(JwtError::RevocationStoreCapacity);
    }
    Ok(())
}

async fn counts(connection: &mut SqliteConnection) -> Result<(usize, usize), JwtError> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM rullst_auth_jwt_tokens), (SELECT COUNT(*) FROM rullst_auth_jwt_subjects)",
    )
    .fetch_one(&mut *connection)
    .await
    .map_err(|_| backend_error("count SQLite revocations"))?;
    let tokens = usize::try_from(row.0).map_err(|_| backend_error("validate token count"))?;
    let subjects = usize::try_from(row.1).map_err(|_| backend_error("validate subject count"))?;
    Ok((tokens, subjects))
}

async fn finish<T>(
    transaction: Transaction<'static, Sqlite>,
    result: Result<T, JwtError>,
    operation: &'static str,
) -> Result<T, JwtError> {
    // SQLx owns rollback on task cancellation, including cancellation while
    // beginning or finishing the transaction. A raw pooled BEGIN does not.
    if result.is_ok() {
        transaction.commit().await
    } else {
        transaction.rollback().await
    }
    .map_err(|_| backend_error(operation))?;
    result
}

fn backend_error(operation: &'static str) -> JwtError {
    JwtError::RevocationBackend(operation.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_revocation_rolls_back_before_pool_reuse() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("isolated test database");
        prepare_schema(&pool, 16).await.expect("revocation schema");
        let store = SqliteJwtRevocationStore {
            pool,
            max_entries: 16,
        };
        let writer = store.clone();
        let (ready, started) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut transaction = writer.begin_write("cancelled write").await.unwrap();
            sqlx::query("INSERT INTO rullst_auth_jwt_subjects (subject, minimum_session_version) VALUES ('cancelled-subject', 3)")
                .execute(&mut *transaction)
                .await
                .unwrap();
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
            drop(transaction);
        });
        started.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM rullst_auth_jwt_subjects")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(
            count.0, 0,
            "cancelled transaction must not leak pooled state"
        );
        store
            .revoke_subject_before("completed-subject", 2)
            .await
            .expect("next transaction must not inherit an open transaction");
        store.close().await;
    }
}
