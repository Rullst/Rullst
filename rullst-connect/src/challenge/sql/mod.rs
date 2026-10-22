//! Login challenges in the application's own SQL database.

use super::{ChallengeError, ChallengeId, ChallengeStore, LoginChallenge, ReturnPath};
use sqlx::{Database, Pool, Transaction};
use std::fmt;
use std::future::Future;

#[cfg(feature = "mysql")]
mod mysql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;

/// Largest entry limit a [`SqlChallengeStore`] accepts.
const MAX_SQL_ENTRIES: u32 = 1_000_000;

/// A [`SqlChallengeStore`] over a `SqlitePool`.
#[cfg(feature = "sqlite")]
pub type SqliteChallengeStore = SqlChallengeStore<sqlx::Sqlite>;
/// A [`SqlChallengeStore`] over a `PgPool`.
#[cfg(feature = "postgres")]
pub type PostgresChallengeStore = SqlChallengeStore<sqlx::Postgres>;
/// A [`SqlChallengeStore`] over a `MySqlPool`.
#[cfg(feature = "mysql")]
pub type MySqlChallengeStore = SqlChallengeStore<sqlx::MySql>;

mod sealed {
    pub trait Sealed {}
}

/// An SQLx database this crate stores challenges in: SQLite (`sqlite`
/// feature), PostgreSQL (`postgres`) or MySQL/MariaDB (`mysql`).
///
/// Sealed. Its methods are the per-database SQL behind [`SqlChallengeStore`]
/// and are not meant to be called directly.
pub trait ChallengeDatabase: Database + sealed::Sealed {
    #[doc(hidden)]
    fn prepare_challenges(
        pool: &Pool<Self>,
    ) -> impl Future<Output = Result<(), ChallengeError>> + Send;

    #[doc(hidden)]
    fn insert_challenge(
        pool: &Pool<Self>,
        key: &str,
        challenge: &LoginChallenge,
        max_entries: u32,
    ) -> impl Future<Output = Result<(), ChallengeError>> + Send;

    #[doc(hidden)]
    fn remove_challenge(
        pool: &Pool<Self>,
        key: &str,
    ) -> impl Future<Output = Result<Option<LoginChallenge>, ChallengeError>> + Send;

    #[doc(hidden)]
    fn purge_challenges(
        pool: &Pool<Self>,
        now: u64,
        limit: u32,
    ) -> impl Future<Output = Result<u64, ChallengeError>> + Send;

    #[doc(hidden)]
    fn count_challenges(
        pool: &Pool<Self>,
    ) -> impl Future<Output = Result<u64, ChallengeError>> + Send;
}

/// Challenges in the table `rullst_connect_login_challenges` of the
/// application's database, reached through the application's own pool (for
/// a Rullst application, `Orm::pool()`).
///
/// [`Self::open`] creates the table and its expiry index when they are
/// missing, the way the SQLite token store prepares its schema. Rows are
/// keyed by [`ChallengeId::storage_key`]; the state, nonce and verifier are
/// stored as issued and deleted when the callback consumes them.
/// Consumption is one atomic statement or transaction on every backend:
/// `DELETE … RETURNING` on SQLite and PostgreSQL, `SELECT … FOR UPDATE` then
/// `DELETE` in one transaction on MySQL. When a save finds the store at its
/// entry limit, it first deletes expired rows. On SQLite the limit is exact;
/// on PostgreSQL and MySQL concurrent saves can pass it by at most their
/// number.
pub struct SqlChallengeStore<DB: ChallengeDatabase> {
    pool: Pool<DB>,
    max_entries: u32,
}

impl<DB: ChallengeDatabase> SqlChallengeStore<DB> {
    /// Prepares the table on `pool` and returns a store holding at most
    /// `max_entries` (1 to 1,000,000) live challenges.
    pub async fn open(pool: Pool<DB>, max_entries: u32) -> Result<Self, ChallengeError> {
        if !(1..=MAX_SQL_ENTRIES).contains(&max_entries) {
            return Err(ChallengeError::Invalid("entry limit"));
        }
        DB::prepare_challenges(&pool).await?;
        Ok(Self { pool, max_entries })
    }

    /// The number of stored challenges, expired ones included.
    pub async fn count(&self) -> Result<u64, ChallengeError> {
        DB::count_challenges(&self.pool).await
    }
}

impl<DB: ChallengeDatabase> ChallengeStore for SqlChallengeStore<DB> {
    async fn save(&self, challenge: &LoginChallenge) -> Result<ChallengeId, ChallengeError> {
        let id = ChallengeId::generate();
        DB::insert_challenge(&self.pool, &id.storage_key(), challenge, self.max_entries).await?;
        Ok(id)
    }

    async fn remove(&self, id: &ChallengeId) -> Result<Option<LoginChallenge>, ChallengeError> {
        DB::remove_challenge(&self.pool, &id.storage_key()).await
    }

    async fn purge_expired(&self, now: u64, limit: u32) -> Result<u64, ChallengeError> {
        DB::purge_challenges(&self.pool, now, limit).await
    }
}

impl<DB: ChallengeDatabase> Clone for SqlChallengeStore<DB> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            max_entries: self.max_entries,
        }
    }
}

impl<DB: ChallengeDatabase> fmt::Debug for SqlChallengeStore<DB> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqlChallengeStore")
            .field("database", &"[REDACTED]")
            .field("max_entries", &self.max_entries)
            .finish()
    }
}

/// One stored row: provider, state, nonce, verifier, return path, created
/// and expiry seconds.
type ChallengeRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
    i64,
);

fn corrupt<E>(_: E) -> ChallengeError {
    ChallengeError::CorruptStorage("stored challenge")
}

fn decode(row: ChallengeRow) -> Result<LoginChallenge, ChallengeError> {
    let (provider, state, nonce, verifier, return_to, created_at, expires_at) = row;
    let created_at = u64::try_from(created_at).map_err(corrupt)?;
    let expires_at = u64::try_from(expires_at).map_err(corrupt)?;
    let return_to = return_to
        .map(ReturnPath::parse)
        .transpose()
        .map_err(corrupt)?;
    LoginChallenge::new(
        provider, state, nonce, verifier, return_to, created_at, expires_at,
    )
    .map_err(corrupt)
}

/// A Unix second as the signed column value; [`LoginChallenge`] keeps its
/// own times in range, and a later `now` purges everything.
fn seconds(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn count_from(value: i64) -> Result<u64, ChallengeError> {
    u64::try_from(value).map_err(|_| ChallengeError::CorruptStorage("entry count"))
}

async fn finish<DB: Database, T>(
    transaction: Transaction<'static, DB>,
    result: Result<T, ChallengeError>,
    operation: &'static str,
) -> Result<T, ChallengeError> {
    match result {
        Ok(value) => {
            transaction
                .commit()
                .await
                .map_err(|_| unavailable(operation))?;
            Ok(value)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(|_| unavailable(operation))?;
            Err(error)
        }
    }
}

const fn unavailable(operation: &'static str) -> ChallengeError {
    ChallengeError::StorageUnavailable(operation)
}

#[cfg(all(test, feature = "sqlite"))]
#[path = "tests.rs"]
mod tests;
