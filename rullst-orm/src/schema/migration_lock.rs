//! Cross-process serialization of the built-in migration runner.

use sqlx::{ConnectOptions, Connection};

use crate::Error;

/// Advisory-lock key of the built-in runner (ASCII `rullstmg`). PostgreSQL
/// scopes advisory locks to the current database.
const POSTGRES_LOCK_KEY: i64 = 0x7275_6c6c_7374_6d67;

/// Blocks until the runner lock is free. `pg_advisory_lock` returns `void`,
/// which the SQLx `Any` driver cannot decode, so the result is cast to text.
const POSTGRES_LOCK_SQL: &str = "SELECT CAST(pg_advisory_lock($1) AS TEXT)";

/// MySQL/MariaDB user-level locks are server-wide, so the name binds the
/// current database; the SHA-1 digest keeps it within the 64-character limit.
/// Each attempt waits up to 10 seconds and returns 0 on timeout (MariaDB
/// rejects the negative "infinite" timeout that MySQL accepts).
const MYSQL_LOCK_SQL: &str = "SELECT CAST(GET_LOCK(CONCAT('rullst_orm_migrations:', SHA1(COALESCE(DATABASE(), ''))), 10) AS SIGNED)";

type LockConnection = <crate::database::RullstDatabase as sqlx::Database>::Connection;

/// Holds the migration runner lock of one database until released or dropped.
///
/// The lock lives on a dedicated connection opened outside the ORM pool, so
/// migrations keep every pooled connection and the lock never returns to the
/// pool: closing (or dropping) the connection ends its session, which
/// releases the lock even when the runner fails or is cancelled. SQLite has no
/// cross-process advisory lock, so SQLite runners are not serialized.
pub(super) struct MigrationLock {
    connection: Option<LockConnection>,
}

impl MigrationLock {
    /// Waits until no other runner of this database holds the lock.
    pub(super) async fn acquire(pool: &crate::RullstPool, driver: &str) -> Result<Self, Error> {
        if !matches!(driver, "postgres" | "mysql") {
            return Ok(Self { connection: None });
        }
        let mut connection = pool.connect_options().connect().await?;
        if driver == "postgres" {
            sqlx::query(POSTGRES_LOCK_SQL)
                .bind(POSTGRES_LOCK_KEY)
                .execute(&mut connection)
                .await?;
        } else {
            // Like `pg_advisory_lock`, wait for as long as another runner
            // holds the lock; only a failed (NULL) attempt is an error.
            loop {
                let acquired: Option<i64> = sqlx::query_scalar(MYSQL_LOCK_SQL)
                    .fetch_one(&mut connection)
                    .await?;
                match acquired {
                    Some(1) => break,
                    Some(0) => continue,
                    _ => {
                        return Err(Error::DatabaseError(
                            "the migration runner lock could not be acquired".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(Self {
            connection: Some(connection),
        })
    }

    /// Ends the lock session. A failed graceful close still drops the
    /// connection, which ends the session and releases the lock.
    pub(super) async fn release(mut self) {
        if let Some(connection) = self.connection.take() {
            let _ = connection.close().await;
        }
    }
}
