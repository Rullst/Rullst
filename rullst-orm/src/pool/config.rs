//! Settings of the application pool beyond its URL: [`OrmConfig`] for
//! [`Orm::init_with_config`] and the SQLite journal mode it can set.

use super::{Orm, dsn};
use crate::{RullstPool, RullstPoolOptions};

/// Journal mode of the application's SQLite database, chosen with
/// [`OrmConfig::with_sqlite_journal_mode`] or, for an application that
/// `Server` starts, `sqlite_journal_mode` in the `[database]` table of
/// `Rullst.toml` (`"wal"` or `"delete"`).
///
/// Without a choice ([`Orm::init`], or a `Rullst.toml` without the key) the
/// pool keeps the mode recorded in the database file: the rollback journal
/// for a new file, or WAL when an earlier connection switched the file to it.
/// The `synchronous` setting is not changed: it stays SQLite's `FULL`, so a
/// committed transaction survives a power loss in either mode. Added in
/// 13.0.0-alpha.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SqliteJournalMode {
    /// The rollback journal (`PRAGMA journal_mode = DELETE`), SQLite's
    /// default. Every statement takes and releases a lock on the database
    /// file, and a reader waits while a transaction commits. The database is
    /// one file, so copying it while no process has it open is a backup.
    /// Keep it for a database on a network filesystem (NFS, SMB, Azure Files),
    /// where WAL's shared memory does not work.
    Delete,
    /// The write-ahead log (`PRAGMA journal_mode = WAL`): readers neither
    /// lock the database file for each statement nor wait for the writer.
    /// SQLite keeps two more files next to the database, `<file>-wal` and
    /// `<file>-shm`, on the same volume; committed data can sit in the `-wal`
    /// file, so the database file alone is not a backup (use `sqlite3
    /// <file> ".backup <copy>"` or `VACUUM INTO`). Every process that opens
    /// the database must run on the same host, on a local filesystem.
    Wal,
}

impl SqliteJournalMode {
    /// The mode as `PRAGMA journal_mode` reports it: `"delete"` or `"wal"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Wal => "wal",
        }
    }

    /// The statement that sets the mode and reports the mode in effect.
    pub(super) const fn pragma(self) -> &'static str {
        match self {
            Self::Delete => "PRAGMA journal_mode = DELETE",
            Self::Wal => "PRAGMA journal_mode = WAL",
        }
    }
}

/// Settings of the application pool, for [`Orm::init_with_config`].
///
/// `OrmConfig::default()` changes nothing, so
/// `Orm::init_with_config(url, OrmConfig::default())` is `Orm::init(url)`.
/// Added in 13.0.0-alpha.6.
///
/// ```no_run
/// use rullst_orm::{Orm, OrmConfig, SqliteJournalMode};
///
/// # async fn start() -> Result<(), rullst_orm::Error> {
/// let config = OrmConfig::default().with_sqlite_journal_mode(SqliteJournalMode::Wal);
/// Orm::init_with_config("sqlite://data/app.db?mode=rwc", config).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrmConfig {
    sqlite_journal_mode: Option<SqliteJournalMode>,
}

impl OrmConfig {
    /// Sets the journal mode of a SQLite database file when the pool
    /// connects. A PostgreSQL or MySQL URL and an in-memory SQLite database
    /// (which has no file to share) ignore it.
    pub const fn with_sqlite_journal_mode(mut self, mode: SqliteJournalMode) -> Self {
        self.sqlite_journal_mode = Some(mode);
        self
    }

    /// The SQLite journal mode that [`Self::with_sqlite_journal_mode`] chose,
    /// if any.
    pub const fn sqlite_journal_mode(&self) -> Option<SqliteJournalMode> {
        self.sqlite_journal_mode
    }
}

impl Orm {
    /// [`Orm::init`] with the settings of `config`.
    ///
    /// When `config` names a SQLite journal mode and `database_url` a SQLite
    /// file, the first connection sets the mode, which SQLite records in the
    /// file, so every later connection (and every other process) uses it.
    /// It fails with `Error::DatabaseError` when SQLite cannot switch: the
    /// connection is read-only, another process is reading the database
    /// while the mode changes, or the filesystem or VFS has no shared memory.
    pub async fn init_with_config(
        database_url: &str,
        config: OrmConfig,
    ) -> Result<(), crate::Error> {
        Self::ensure_uninitialized()?;
        dsn::ensure_configured_dsn(database_url)?;
        Self::validate_dsn(database_url);
        #[cfg(not(any(
            feature = "strict-postgres",
            feature = "strict-mysql",
            feature = "strict-sqlite"
        )))]
        sqlx::any::install_default_drivers();

        let options = Self::pool_options()
            .acquire_timeout(std::time::Duration::from_secs(10))
            .idle_timeout(Some(std::time::Duration::from_secs(300)))
            .max_lifetime(Some(std::time::Duration::from_secs(1800)));
        let pool = Self::connect_primary(database_url, options, &config).await?;
        Self::publish(super::OrmState::new(
            pool,
            dsn::driver_for_url(database_url),
            Vec::new(),
        ))
    }

    /// Connects `options` to `database_url` and applies `config`; the pool
    /// is closed again when a setting fails.
    pub(super) async fn connect_primary(
        database_url: &str,
        options: RullstPoolOptions,
        config: &OrmConfig,
    ) -> Result<RullstPool, crate::Error> {
        let pool = Self::options_for(options, database_url)
            .connect(database_url)
            .await
            .map_err(|error| dsn::connect_error(database_url, error))?;
        if let Some(mode) = journal_mode_for(database_url, config)
            && let Err(error) = set_journal_mode(&pool, mode).await
        {
            pool.close().await;
            return Err(error);
        }
        Ok(pool)
    }
}

/// The journal mode to set on the database `database_url` names: the one
/// `config` chose, for a SQLite file only.
pub(super) fn journal_mode_for(
    database_url: &str,
    config: &OrmConfig,
) -> Option<SqliteJournalMode> {
    config.sqlite_journal_mode.filter(|_| {
        dsn::driver_for_url(database_url) == "sqlite" && !dsn::is_sqlite_memory(database_url)
    })
}

/// Sets `mode` on the database behind `executor` and checks the mode SQLite
/// reports afterwards.
pub(super) async fn set_journal_mode<'c, E, DB>(
    executor: E,
    mode: SqliteJournalMode,
) -> Result<(), crate::Error>
where
    DB: sqlx::Database,
    E: sqlx::Executor<'c, Database = DB>,
    (String,): for<'r> sqlx::FromRow<'r, DB::Row>,
    <DB as sqlx::Database>::Arguments: sqlx::IntoArguments<DB>,
{
    let reported: String = sqlx::query_scalar(mode.pragma())
        .fetch_one(executor)
        .await
        .map_err(|error| {
            crate::Error::DatabaseError(format!(
                "could not set the SQLite journal mode to {}: {error}. When another process \
                 was reading the database, start again once it is idle",
                mode.as_str()
            ))
        })?;
    confirm_journal_mode(mode, &reported)
}

/// Fails unless SQLite reports `requested` as the mode in effect.
pub(super) fn confirm_journal_mode(
    requested: SqliteJournalMode,
    reported: &str,
) -> Result<(), crate::Error> {
    if reported.eq_ignore_ascii_case(requested.as_str()) {
        return Ok(());
    }
    Err(crate::Error::DatabaseError(format!(
        "SQLite kept the {reported} journal instead of {}: the database is read-only or its \
         filesystem or VFS has no shared memory. Use sqlite_journal_mode = \"delete\" for a \
         database on a network filesystem",
        requested.as_str()
    )))
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
