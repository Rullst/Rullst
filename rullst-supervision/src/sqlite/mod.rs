//! Bounded shared-local SQLite. No replication or network-filesystem support.
//! Restore requires independently re-established authority and deployment epoch.

#[cfg(feature = "analysis")]
mod analysis;
mod authority;
mod events;
mod maintenance;
mod observations;
mod parental;
mod schema;
mod sessions;
mod transaction;

use crate::{Clock, StoreConfig, SupervisionError as Error, SystemClock};
pub use maintenance::PurgeReceipt;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use transaction::Operation;

/// One private initialized file and exact immutable configuration. Clones share
/// a pool; separate opens coordinate through SQLite. The host owns identity,
/// trusted directories, encryption, keys, retention and reliable local storage.
#[derive(Clone)]
pub struct SqliteSupervision<C = SystemClock> {
    pool: SqlitePool,
    config: StoreConfig,
    clock: C,
}

impl<C: Clock> SqliteSupervision<C> {
    /// Exclusive bootstrap; refuses existing files. Failed/cancelled bootstrap
    /// may leave partial state requiring operator inspection, never silent repair.
    pub async fn initialize(
        path: impl AsRef<Path>,
        config: StoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        let path = resolved_path(path.as_ref())?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Configuration
            } else {
                Error::Storage
            }
        })?;
        file.sync_all().map_err(|_| Error::Storage)?;
        drop(file);
        let store = Self::connect(&path, config, clock).await?;
        let prepared = async {
            let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await.map_err(storage)?;
            for (_, statement) in schema::SCHEMA { sqlx::query(*statement).execute(&mut *tx).await.map_err(storage)?; }
            let now = crate::clock::checked_time(store.clock.now()?)?;
            sqlx::query("INSERT INTO rullst_supervision_meta (id,version,config,last_now,revision) VALUES (1,2,?,?,0)")
                .bind(store.config_key()).bind(now).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(|_| Error::UncertainCommit)?;
            store.validate_schema().await
        }.await;
        if let Err(error) = prepared {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    /// Opens existing, complete state only; never creates or repairs state.
    pub async fn open(
        path: impl AsRef<Path>,
        config: StoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        let path = resolved_path(path.as_ref())?;
        if !std::fs::symlink_metadata(&path)
            .map_err(|_| Error::Storage)?
            .file_type()
            .is_file()
        {
            return Err(Error::Configuration);
        }
        let store = Self::connect(&path, config, clock).await?;
        let checked = async {
            store.validate_schema().await?;
            store.begin().await?.finish().await
        }
        .await;
        if let Err(error) = checked {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    /// Closes this pool and its clones. Independent openers remain usable.
    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn connect(path: &Path, config: StoreConfig, clock: C) -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await
            .map_err(storage)?;
        Ok(Self {
            pool,
            config,
            clock,
        })
    }

    fn config_key(&self) -> String {
        let c = &self.config;
        format!(
            "v2|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            c.epoch.as_str(),
            c.limits.grants,
            c.limits.sessions,
            c.limits.events,
            c.limits.managed,
            c.limits.events_per_session,
            c.limits.event_interval,
            c.event_retention,
            c.session_lifetime
        )
    }

    async fn validate_schema(&self) -> Result<(), Error> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        let records: Vec<(String, String)> =
            sqlx::query_as("SELECT substr(name,1,129),substr(sql,1,2049) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' LIMIT 10")
                .fetch_all(&mut *tx)
                .await
                .map_err(storage)?;
        if records.len() != schema::SCHEMA.len()
            || schema::SCHEMA.iter().any(|(name, sql)| {
                !records
                    .iter()
                    .any(|(found, definition)| name == found && sql == definition)
            })
        {
            return Err(Error::Configuration);
        }
        if sqlx::query("PRAGMA foreign_key_check")
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .is_some()
        {
            return Err(Error::Configuration);
        }
        tx.commit().await.map_err(|_| Error::UncertainCommit)
    }

    async fn begin(&self) -> Result<Operation<'_, C>, Error> {
        Operation::begin(self).await
    }
}

fn storage(_: sqlx::Error) -> Error {
    Error::Storage
}

fn resolved_path(path: &Path) -> Result<PathBuf, Error> {
    let filename = path
        .file_name()
        .filter(|name| *name != ":memory:")
        .ok_or(Error::Configuration)?;
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let resolved = parent
        .canonicalize()
        .map_err(|_| Error::Storage)?
        .join(filename);
    match std::fs::symlink_metadata(&resolved) {
        Ok(metadata) if !metadata.file_type().is_file() => return Err(Error::Configuration),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(Error::Storage),
        _ => {}
    }
    Ok(resolved)
}
