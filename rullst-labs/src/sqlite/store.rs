use super::crypto::ContentKey;
use crate::{
    Clock, ExecutionProfile, LabError as Error, Reference, SystemClock, authorization::checked_time,
};
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
        "labs_meta",
        "CREATE TABLE labs_meta (id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL CHECK(version=1), configuration TEXT NOT NULL, last_now INTEGER NOT NULL CHECK(last_now>0))",
    ),
    (
        "labs_exercises",
        "CREATE TABLE labs_exercises (tenant TEXT NOT NULL, course TEXT NOT NULL, id TEXT NOT NULL, revision TEXT NOT NULL, digest TEXT NOT NULL, enabled INTEGER NOT NULL CHECK(enabled IN (0,1)), content BLOB NOT NULL CHECK(length(content)<=262172), PRIMARY KEY(tenant,course,id,revision))",
    ),
    (
        "labs_jobs",
        "CREATE TABLE labs_jobs (tenant TEXT NOT NULL, course TEXT NOT NULL, id TEXT NOT NULL, learner TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), expires_at INTEGER NOT NULL, lease_until INTEGER NOT NULL, body BLOB NOT NULL CHECK(length(body)<=32796), content BLOB CHECK(length(content)<=262172), PRIMARY KEY(tenant,course,id))",
    ),
    (
        "labs_ready",
        "CREATE INDEX labs_ready ON labs_jobs(state,expires_at,tenant,course,id)",
    ),
];

#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub(super) namespace: Reference,
    pub(super) max_jobs: u32,
    pub(super) max_exercises: u32,
    pub(super) profile: ExecutionProfile,
}
impl StoreConfig {
    pub fn new(
        namespace: Reference,
        max_jobs: u32,
        max_exercises: u32,
        profile: ExecutionProfile,
    ) -> Result<Self, Error> {
        if !(1..=1000).contains(&max_jobs) || !(1..=1000).contains(&max_exercises) {
            return Err(Error::Configuration);
        }
        Ok(Self {
            namespace,
            max_jobs,
            max_exercises,
            profile,
        })
    }
    pub(super) fn binding(&self, key: &ContentKey) -> Result<String, Error> {
        serde_json::to_string(&(
            crate::PROTOCOL_VERSION,
            &self.namespace,
            self.max_jobs,
            self.max_exercises,
            &self.profile,
            key.binding(),
        ))
        .map_err(|_| Error::Configuration)
    }
}

/// Shared-local trusted SQLite. The encrypted job plane and its key must be
/// separate from identity/session databases and absent from untrusted workers.
#[derive(Clone)]
pub struct SqliteLabs<C = SystemClock> {
    pub(super) pool: SqlitePool,
    pub(super) config: StoreConfig,
    pub(super) key: ContentKey,
    pub(super) clock: C,
}
impl<C: Clock> SqliteLabs<C> {
    pub async fn initialize(
        path: impl AsRef<Path>,
        config: StoreConfig,
        key: ContentKey,
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
        let store = Self::connect(&path, config, key, clock).await?;
        let mut tx = store
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(storage)?;
        for (_, sql) in SCHEMA {
            sqlx::query(*sql).execute(&mut *tx).await.map_err(storage)?;
        }
        sqlx::query("INSERT INTO labs_meta VALUES (1,1,?,?)")
            .bind(store.config.binding(&store.key)?)
            .bind(checked_time(store.clock.now()?)?)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(|_| Error::Uncertain)?;
        store.validate().await?;
        super::transaction::Operation::begin(&store, None)
            .await?
            .commit()
            .await?;
        Ok(store)
    }
    pub async fn open(
        path: impl AsRef<Path>,
        config: StoreConfig,
        key: ContentKey,
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
        let store = Self::connect(&path, config, key, clock).await?;
        store.validate().await?;
        super::transaction::Operation::begin(&store, None)
            .await?
            .commit()
            .await?;
        Ok(store)
    }
    async fn connect(
        path: &Path,
        config: StoreConfig,
        key: ContentKey,
        clock: C,
    ) -> Result<Self, Error> {
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
            key,
            clock,
        })
    }
    async fn validate(&self) -> Result<(), Error> {
        let rows:Vec<(String,String)>=sqlx::query_as("SELECT substr(name,1,65),substr(sql,1,4097) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' LIMIT 6").fetch_all(&self.pool).await.map_err(storage)?;
        if rows.len() != SCHEMA.len()
            || SCHEMA
                .iter()
                .any(|(name, sql)| !rows.iter().any(|(n, s)| name == n && sql == s))
        {
            return Err(Error::Configuration);
        }
        let config: Vec<(i64, i64, String, i64)> = sqlx::query_as(
            "SELECT id,version,substr(configuration,1,4097),last_now FROM labs_meta LIMIT 2",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?;
        match config.as_slice() {
            [(1, 1, binding, last)] if binding == &self.config.binding(&self.key)? && *last > 0 => {
                Ok(())
            }
            _ => Err(Error::Configuration),
        }
    }
    pub fn profile(&self) -> &ExecutionProfile {
        &self.config.profile
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
