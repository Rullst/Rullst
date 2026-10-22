//! Authoritative shared-local optional-consent state; remote filesystems and
//! multi-host replication are not supported. A restore must invalidate existing
//! policy versions before processing resumes, or old grants could reappear.

use super::*;
use sqlx::{
    SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const SCHEMA: [&str; 2] = [
    "CREATE TABLE rullst_consent_meta (id INTEGER PRIMARY KEY CHECK(id = 1), schema_version INTEGER NOT NULL CHECK(schema_version = 1), capacity INTEGER NOT NULL CHECK(capacity BETWEEN 1 AND 100000), last_now INTEGER NOT NULL CHECK(last_now >= 0))",
    "CREATE TABLE rullst_consent_choices (scope_digest BLOB PRIMARY KEY NOT NULL CHECK(length(scope_digest) = 32), revision INTEGER NOT NULL CHECK(revision > 0), policy_version TEXT NOT NULL, choice INTEGER NOT NULL CHECK(choice IN (1,2,3)), changed_at INTEGER NOT NULL CHECK(changed_at >= 0), valid_until INTEGER NOT NULL CHECK(valid_until >= 0))",
];

/// Bounded SQLite state shared by processes using the same trusted local file.
/// Stores scope digests, latest choice/version/revision and timestamps, with no
/// raw subject IDs. Digests remain pseudonymous data, not anonymization.
/// All reads and writes use WAL/full synchronization and `BEGIN IMMEDIATE` to
/// persist the global clock high-water mark and serialize revision changes.
/// Tombstones are never evicted automatically. The operator owns file custody,
/// capacity planning, synchronized time, backup policy and durable storage.
#[derive(Clone)]
pub struct SqliteConsentStore {
    pool: SqlitePool,
    capacity: i64,
}

impl SqliteConsentStore {
    /// Explicit deployment bootstrap. Creates a new file exclusively; refuses
    /// every existing file. A failed/cancelled initialization may leave an empty
    /// or partial file for the operator to inspect; it is never silently repaired.
    pub async fn initialize(path: impl AsRef<Path>, capacity: usize) -> Result<Self, ConsentError> {
        validate_capacity(capacity)?;
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
                ConsentError::StoreConfiguration
            } else {
                ConsentError::StoreUnavailable
            }
        })?;
        file.sync_all()
            .map_err(|_| ConsentError::StoreUnavailable)?;
        drop(file);
        let store = Self::connect(&path, capacity).await?;
        let prepared = async {
            let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await.map_err(unavailable)?;
            for statement in SCHEMA { sqlx::query(statement).execute(&mut *tx).await.map_err(unavailable)?; }
            sqlx::query("INSERT INTO rullst_consent_meta (id,schema_version,capacity,last_now) VALUES (1,1,?,0)")
                .bind(store.capacity).execute(&mut *tx).await.map_err(unavailable)?;
            tx.commit().await.map_err(unavailable)
        }.await;
        if let Err(error) = prepared {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    /// Opens only initialized existing state; never creates missing files/tables.
    pub async fn open(path: impl AsRef<Path>, capacity: usize) -> Result<Self, ConsentError> {
        validate_capacity(capacity)?;
        let path = resolved_path(path.as_ref())?;
        if !std::fs::symlink_metadata(&path)
            .map_err(|_| ConsentError::StoreUnavailable)?
            .file_type()
            .is_file()
        {
            return Err(ConsentError::StoreConfiguration);
        }
        let store = Self::connect(&path, capacity).await?;
        let checked = async {
            let mut tx = store
                .pool
                .begin_with("BEGIN IMMEDIATE")
                .await
                .map_err(unavailable)?;
            store.metadata(&mut tx).await?;
            store.count(&mut tx).await?;
            tx.commit().await.map_err(unavailable)
        }
        .await;
        if let Err(error) = checked {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    async fn connect(path: &Path, capacity: usize) -> Result<Self, ConsentError> {
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
            .map_err(unavailable)?;
        Ok(Self {
            pool,
            capacity: capacity as i64,
        })
    }

    async fn metadata(&self, connection: &mut SqliteConnection) -> Result<i64, ConsentError> {
        let rows: Vec<(i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT id,schema_version,capacity,last_now FROM rullst_consent_meta LIMIT 2",
        )
        .fetch_all(connection)
        .await
        .map_err(unavailable)?;
        match rows.as_slice() {
            [(1, 1, capacity, last_now)] if *capacity == self.capacity && *last_now >= 0 => {
                Ok(*last_now)
            }
            _ => Err(ConsentError::StoreConfiguration),
        }
    }

    async fn count(&self, connection: &mut SqliteConnection) -> Result<i64, ConsentError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_consent_choices")
            .fetch_one(connection)
            .await
            .map_err(unavailable)?;
        if !(0..=self.capacity).contains(&count) {
            return Err(ConsentError::StoreConfiguration);
        }
        Ok(count)
    }

    async fn observe(
        &self,
        connection: &mut SqliteConnection,
        now: i64,
    ) -> Result<(), ConsentError> {
        if now < self.metadata(connection).await? {
            return Err(ConsentError::ClockRollback);
        }
        sqlx::query("UPDATE rullst_consent_meta SET last_now = ? WHERE id = 1")
            .bind(now)
            .execute(connection)
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn load(
        connection: &mut SqliteConnection,
        subject: &ConsentSubject,
        purpose_id: &str,
    ) -> Result<ConsentRecord, ConsentError> {
        let absent = ConsentRecord::absent(subject.clone(), purpose_id)?;
        let digest = scope_digest(subject, purpose_id);
        let row: Option<(i64, String, i64, i64, i64)> = sqlx::query_as("SELECT revision,policy_version,choice,changed_at,valid_until FROM rullst_consent_choices WHERE scope_digest = ?")
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

impl ConsentStore for SqliteConsentStore {
    fn durability(&self) -> ConsentDurability {
        ConsentDurability::SharedDurable
    }

    async fn read(
        &self,
        subject: &ConsentSubject,
        purpose_id: &str,
        now: i64,
    ) -> Result<ConsentRecord, ConsentError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
        self.observe(&mut tx, now).await?;
        let record = Self::load(&mut tx, subject, purpose_id).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(record)
    }

    async fn update(&self, update: &ConsentUpdate) -> Result<ConsentRecord, ConsentError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
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
        sqlx::query("INSERT INTO rullst_consent_choices (scope_digest,revision,policy_version,choice,changed_at,valid_until) VALUES (?,?,?,?,?,?) ON CONFLICT(scope_digest) DO UPDATE SET revision=excluded.revision,policy_version=excluded.policy_version,choice=excluded.choice,changed_at=excluded.changed_at,valid_until=excluded.valid_until")
            .bind(digest.as_ref()).bind(next.revision() as i64).bind(next.version()).bind(choice).bind(next.changed_at()).bind(next.valid_until())
            .execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(next)
    }
}

fn resolved_path(path: &Path) -> Result<PathBuf, ConsentError> {
    let name = path
        .file_name()
        .filter(|name| *name != ":memory:")
        .ok_or(ConsentError::InvalidConfiguration)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    Ok(parent
        .canonicalize()
        .map_err(|_| ConsentError::StoreUnavailable)?
        .join(name))
}

fn unavailable(_: sqlx::Error) -> ConsentError {
    ConsentError::StoreUnavailable
}
