use crate::{Clock, MediaError as Error, ProviderBinding, ProviderMode, SystemClock};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(super) const SCHEMA: &[(&str, &str)] = &[
    (
        "media_meta",
        "CREATE TABLE media_meta (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL CHECK(version=1), configuration TEXT NOT NULL, last_now INTEGER NOT NULL CHECK(last_now>0))",
    ),
    (
        "media_assets",
        "CREATE TABLE media_assets (id TEXT NOT NULL, tenant TEXT NOT NULL, course TEXT NOT NULL, video TEXT UNIQUE, revision INTEGER NOT NULL CHECK(revision>0), body TEXT NOT NULL CHECK(length(body)<=32768), PRIMARY KEY(tenant,course,id))",
    ),
    (
        "media_scope",
        "CREATE INDEX media_scope ON media_assets(tenant,course,id)",
    ),
];

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub(super) binding: ProviderBinding,
    pub(super) max_assets: u32,
    pub(super) testing: bool,
}
impl StoreConfig {
    pub fn production(binding: ProviderBinding, max_assets: u32) -> Result<Self, Error> {
        if binding.mode != ProviderMode::RemoteUnvalidated {
            return Err(Error::Configuration);
        }
        Self::new(binding, max_assets, false)
    }
    /// Explicit offline/protocol acceptance only; never promotes fixtures to live.
    pub fn testing(binding: ProviderBinding, max_assets: u32) -> Result<Self, Error> {
        Self::new(binding, max_assets, true)
    }
    fn new(binding: ProviderBinding, max_assets: u32, testing: bool) -> Result<Self, Error> {
        if max_assets == 0 || max_assets > 10_000 {
            return Err(Error::Configuration);
        }
        Ok(Self {
            binding,
            max_assets,
            testing,
        })
    }
    pub(super) fn key(&self) -> Result<String, Error> {
        serde_json::to_string(&(&self.binding, self.max_assets, self.testing))
            .map_err(|_| Error::Configuration)
    }
}

/// Authoritative shared-local SQLite. The host owns a trusted private directory,
/// encryption/backup/restore and exclusive schema administration. No network FS.
#[derive(Clone)]
pub struct SqliteMedia<C = SystemClock> {
    pub(super) pool: SqlitePool,
    pub(super) config: StoreConfig,
    pub(super) clock: C,
}
impl<C: Clock> SqliteMedia<C> {
    pub async fn initialize(
        path: impl AsRef<Path>,
        config: StoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        let path = resolved(path.as_ref())?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path).map_err(|_| Error::Configuration)?;
        file.sync_all().map_err(|_| Error::Storage)?;
        drop(file);
        let store = Self::connect(&path, config, clock).await?;
        let mut tx = store
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        for (_, sql) in SCHEMA {
            sqlx::query(*sql).execute(&mut *tx).await.map_err(storage)?;
        }
        let now = crate::contracts::checked_time(store.clock.now()?)?;
        sqlx::query("INSERT INTO media_meta VALUES (1,1,?,?)")
            .bind(store.config.key()?)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(|_| Error::Uncertain)?;
        store.validate().await?;
        Ok(store)
    }
    pub async fn open(
        path: impl AsRef<Path>,
        config: StoreConfig,
        clock: C,
    ) -> Result<Self, Error> {
        let path = resolved(path.as_ref())?;
        if !std::fs::symlink_metadata(&path)
            .map_err(|_| Error::Storage)?
            .file_type()
            .is_file()
        {
            return Err(Error::Configuration);
        }
        let store = Self::connect(&path, config, clock).await?;
        store.validate().await?;
        super::transaction::Operation::begin(&store)
            .await?
            .commit()
            .await?;
        Ok(store)
    }
    async fn connect(path: &Path, config: StoreConfig, clock: C) -> Result<Self, Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(3));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(3))
            .connect_with(options)
            .await
            .map_err(storage)?;
        Ok(Self {
            pool,
            config,
            clock,
        })
    }
    async fn validate(&self) -> Result<(), Error> {
        let rows: Vec<(String,String)> = sqlx::query_as("SELECT substr(name,1,65),substr(sql,1,2049) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' LIMIT 5")
            .fetch_all(&self.pool).await.map_err(storage)?;
        if rows.len() != SCHEMA.len()
            || SCHEMA
                .iter()
                .any(|(name, sql)| !rows.iter().any(|(n, s)| name == n && sql == s))
        {
            return Err(Error::Configuration);
        }
        Ok(())
    }
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
pub(super) fn storage(_: sqlx::Error) -> Error {
    Error::Storage
}
fn resolved(path: &Path) -> Result<PathBuf, Error> {
    let name = path
        .file_name()
        .filter(|n| *n != ":memory:")
        .ok_or(Error::Configuration)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok(parent
        .canonicalize()
        .map_err(|_| Error::Storage)?
        .join(name))
}
