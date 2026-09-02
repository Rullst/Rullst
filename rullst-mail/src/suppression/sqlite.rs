//! Durable local suppression state shared by processes using one SQLite file.

use super::{
    MutableSuppressionStore, SuppressionError, SuppressionEvent, SuppressionReason,
    SuppressionRecord, SuppressionSnapshot, SuppressionStore, normalize_recipient, unavailable,
    validate_identifier, validate_limits,
};
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const SCHEMA: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS rullst_mail_suppression_meta (id INTEGER PRIMARY KEY CHECK (id = 1), schema_version INTEGER NOT NULL CHECK (schema_version > 0), max_recipients INTEGER NOT NULL CHECK (max_recipients > 0), max_events INTEGER NOT NULL CHECK (max_events > 0))",
    "CREATE TABLE IF NOT EXISTS rullst_mail_suppressed_recipients (recipient TEXT PRIMARY KEY, reason INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 3), provider TEXT NOT NULL, first_seen_at INTEGER NOT NULL CHECK (first_seen_at > 0), last_seen_at INTEGER NOT NULL CHECK (last_seen_at >= first_seen_at), last_event_id TEXT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS rullst_mail_suppression_events (provider TEXT NOT NULL, event_id TEXT NOT NULL, recipient TEXT NOT NULL, reason INTEGER NOT NULL CHECK (reason BETWEEN 1 AND 3), observed_at INTEGER NOT NULL CHECK (observed_at > 0), PRIMARY KEY (provider, event_id))",
    "CREATE INDEX IF NOT EXISTS rullst_mail_suppression_event_time_idx ON rullst_mail_suppression_events(observed_at)",
];

/// File-backed, quota-bounded suppression and replay-evidence store.
///
/// Events must be authenticated and normalized by a provider-specific webhook
/// adapter before they reach this store. SQLite provides shared local state and
/// serialized mutations, not multi-host replication, file encryption, webhook
/// verification, backup or disaster recovery.
#[derive(Clone)]
pub struct SqliteSuppressionStore {
    pool: SqlitePool,
    max_recipients: usize,
    max_events: usize,
}

impl SqliteSuppressionStore {
    /// Opens or creates a file-backed store with persisted immutable quotas.
    pub async fn connect(
        database_url: impl Into<String>,
        max_recipients: usize,
        max_events: usize,
    ) -> Result<Self, SuppressionError> {
        validate_limits(max_recipients, max_events)?;
        let database_url = database_url.into();
        if !database_url.starts_with("sqlite:") {
            return Err(SuppressionError::InvalidConfiguration("database URL"));
        }
        let options = SqliteConnectOptions::from_str(&database_url)
            .map_err(|_| unavailable("parse database URL"))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(30));
        if volatile_database_url(&database_url, options.get_filename()) {
            return Err(SuppressionError::InvalidConfiguration(
                "database must be file-backed",
            ));
        }
        reject_existing_unsafe_target(options.get_filename())?;
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(|_| unavailable("connect database"))?;
        prepare_schema(&pool, max_recipients, max_events).await?;
        Ok(Self {
            pool,
            max_recipients,
            max_events,
        })
    }

    /// Returns current recipient and retained-event counts.
    pub async fn snapshot(&self) -> Result<SuppressionSnapshot, SuppressionError> {
        let (recipients, events) = counts(&self.pool).await?;
        Ok(SuppressionSnapshot::new(
            recipients,
            events,
            self.max_recipients,
            self.max_events,
        ))
    }

    /// Removes replay identifiers older than `cutoff`, retaining suppressions.
    ///
    /// The host must choose a cutoff no shorter than every provider's possible
    /// redelivery window; otherwise an old event can be accepted again.
    pub async fn prune_events_before(&self, cutoff: u64) -> Result<usize, SuppressionError> {
        if cutoff == 0 {
            return Err(SuppressionError::InvalidConfiguration("event cutoff"));
        }
        let cutoff = i64::try_from(cutoff)
            .map_err(|_| SuppressionError::InvalidConfiguration("event cutoff"))?;
        let mut connection = self.begin_write("begin event pruning").await?;
        let result = async {
            let deleted =
                sqlx::query("DELETE FROM rullst_mail_suppression_events WHERE observed_at < ?")
                    .bind(cutoff)
                    .execute(&mut *connection)
                    .await
                    .map_err(|_| unavailable("prune event identifiers"))?;
            usize::try_from(deleted.rows_affected())
                .map_err(|_| SuppressionError::CorruptStorage("deleted event count"))
        }
        .await;
        finish(&mut connection, result, "finish event pruning").await
    }

    /// Gracefully closes all pooled connections, useful before rotating a file.
    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn record_event(
        &self,
        event: SuppressionEvent,
    ) -> Result<SuppressionRecord, SuppressionError> {
        let mut connection = self.begin_write("begin suppression event").await?;
        let result = self.record_in_transaction(&mut connection, &event).await;
        finish(&mut connection, result, "finish suppression event").await
    }

    async fn record_in_transaction(
        &self,
        connection: &mut SqliteConnection,
        event: &SuppressionEvent,
    ) -> Result<SuppressionRecord, SuppressionError> {
        let existing: Option<(String, i64, i64)> = sqlx::query_as(
            "SELECT recipient, reason, observed_at FROM rullst_mail_suppression_events WHERE provider = ? AND event_id = ?",
        )
        .bind(event.provider())
        .bind(event.event_id())
        .fetch_optional(&mut *connection)
        .await
        .map_err(|_| unavailable("lookup provider event"))?;
        if let Some((recipient, reason, observed_at)) = existing {
            let observed_at = u64::try_from(observed_at)
                .map_err(|_| SuppressionError::CorruptStorage("event timestamp"))?;
            if recipient != event.recipient()
                || SuppressionReason::from_rank(reason)? != event.reason()
                || observed_at != event.observed_at()
            {
                return Err(SuppressionError::EventConflict);
            }
            return fetch_record(connection, event.recipient()).await;
        }

        let (recipients, events) = counts(&mut *connection).await?;
        let recipient_exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM rullst_mail_suppressed_recipients WHERE recipient = ?")
                .bind(event.recipient())
                .fetch_optional(&mut *connection)
                .await
                .map_err(|_| unavailable("lookup suppressed recipient"))?;
        if events >= self.max_events
            || (recipient_exists.is_none() && recipients >= self.max_recipients)
        {
            return Err(SuppressionError::CapacityExceeded);
        }

        let observed_at = i64::try_from(event.observed_at())
            .map_err(|_| SuppressionError::InvalidEvent("observation time"))?;
        sqlx::query("INSERT INTO rullst_mail_suppression_events (provider, event_id, recipient, reason, observed_at) VALUES (?, ?, ?, ?, ?)")
            .bind(event.provider())
            .bind(event.event_id())
            .bind(event.recipient())
            .bind(event.reason().rank())
            .bind(observed_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| unavailable("persist provider event"))?;
        sqlx::query(
            "INSERT INTO rullst_mail_suppressed_recipients (recipient, reason, provider, first_seen_at, last_seen_at, last_event_id) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(recipient) DO UPDATE SET reason = MAX(reason, excluded.reason), provider = CASE WHEN excluded.reason > reason OR (excluded.reason = reason AND excluded.last_seen_at >= last_seen_at) THEN excluded.provider ELSE provider END, first_seen_at = MIN(first_seen_at, excluded.first_seen_at), last_seen_at = MAX(last_seen_at, excluded.last_seen_at), last_event_id = CASE WHEN excluded.reason > reason OR (excluded.reason = reason AND excluded.last_seen_at >= last_seen_at) THEN excluded.last_event_id ELSE last_event_id END",
        )
        .bind(event.recipient())
        .bind(event.reason().rank())
        .bind(event.provider())
        .bind(observed_at)
        .bind(observed_at)
        .bind(event.event_id())
        .execute(&mut *connection)
        .await
        .map_err(|_| unavailable("persist suppressed recipient"))?;
        fetch_record(connection, event.recipient()).await
    }

    async fn begin_write(
        &self,
        operation: &'static str,
    ) -> Result<PoolConnection<Sqlite>, SuppressionError> {
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(|_| unavailable(operation))?;
        connection
            .execute("BEGIN IMMEDIATE")
            .await
            .map_err(|_| unavailable(operation))?;
        Ok(connection)
    }
}

impl SuppressionStore for SqliteSuppressionStore {
    async fn lookup(&self, recipient: &str) -> Result<Option<SuppressionRecord>, SuppressionError> {
        let recipient = normalize_recipient(recipient)?;
        let row = sqlx::query_as::<_, RecipientRow>(
            "SELECT recipient, reason, provider, first_seen_at, last_seen_at FROM rullst_mail_suppressed_recipients WHERE recipient = ?",
        )
        .bind(recipient)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| unavailable("lookup suppressed recipient"))?;
        row.map(decode_record).transpose()
    }
}

impl MutableSuppressionStore for SqliteSuppressionStore {
    async fn record(&self, event: SuppressionEvent) -> Result<SuppressionRecord, SuppressionError> {
        self.record_event(event).await
    }
}

type RecipientRow = (String, i64, String, i64, i64);

async fn fetch_record(
    connection: &mut SqliteConnection,
    recipient: &str,
) -> Result<SuppressionRecord, SuppressionError> {
    let row: RecipientRow = sqlx::query_as("SELECT recipient, reason, provider, first_seen_at, last_seen_at FROM rullst_mail_suppressed_recipients WHERE recipient = ?")
        .bind(recipient)
        .fetch_one(&mut *connection)
        .await
        .map_err(|_| unavailable("read suppressed recipient"))?;
    decode_record(row)
}

fn decode_record(row: RecipientRow) -> Result<SuppressionRecord, SuppressionError> {
    let recipient = normalize_recipient(&row.0)?;
    if recipient != row.0 {
        return Err(SuppressionError::CorruptStorage("recipient normalization"));
    }
    validate_identifier(&row.2, 64, "provider")
        .map_err(|_| SuppressionError::CorruptStorage("provider"))?;
    let first_seen_at =
        u64::try_from(row.3).map_err(|_| SuppressionError::CorruptStorage("first timestamp"))?;
    let last_seen_at =
        u64::try_from(row.4).map_err(|_| SuppressionError::CorruptStorage("last timestamp"))?;
    if first_seen_at == 0 || last_seen_at < first_seen_at {
        return Err(SuppressionError::CorruptStorage("recipient timestamps"));
    }
    Ok(SuppressionRecord {
        recipient,
        reason: SuppressionReason::from_rank(row.1)?,
        provider: row.2,
        first_seen_at,
        last_seen_at,
    })
}

async fn prepare_schema(
    pool: &SqlitePool,
    max_recipients: usize,
    max_events: usize,
) -> Result<(), SuppressionError> {
    for statement in SCHEMA {
        pool.execute(*statement)
            .await
            .map_err(|_| unavailable("prepare schema"))?;
    }
    let max_recipients = as_i64(max_recipients, "recipient limit")?;
    let max_events = as_i64(max_events, "event limit")?;
    sqlx::query("INSERT OR IGNORE INTO rullst_mail_suppression_meta (id, schema_version, max_recipients, max_events) VALUES (1, ?, ?, ?)")
        .bind(SCHEMA_VERSION)
        .bind(max_recipients)
        .bind(max_events)
        .execute(pool)
        .await
        .map_err(|_| unavailable("register configuration"))?;
    let stored: (i64, i64, i64) = sqlx::query_as("SELECT schema_version, max_recipients, max_events FROM rullst_mail_suppression_meta WHERE id = 1")
        .fetch_one(pool)
        .await
        .map_err(|_| unavailable("read configuration"))?;
    if stored.0 != SCHEMA_VERSION || stored.1 <= 0 || stored.2 <= 0 {
        return Err(SuppressionError::CorruptStorage("schema configuration"));
    }
    if stored.1 != max_recipients || stored.2 != max_events {
        return Err(SuppressionError::InvalidConfiguration(
            "limits conflict with stored configuration",
        ));
    }
    Ok(())
}

async fn counts<'e, E>(executor: E) -> Result<(usize, usize), SuppressionError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM rullst_mail_suppressed_recipients), (SELECT COUNT(*) FROM rullst_mail_suppression_events)",
    )
    .fetch_one(executor)
    .await
    .map_err(|_| unavailable("count suppression state"))?;
    let recipients =
        usize::try_from(row.0).map_err(|_| SuppressionError::CorruptStorage("recipient count"))?;
    let events =
        usize::try_from(row.1).map_err(|_| SuppressionError::CorruptStorage("event count"))?;
    Ok((recipients, events))
}

async fn finish<T>(
    connection: &mut PoolConnection<Sqlite>,
    result: Result<T, SuppressionError>,
    operation: &'static str,
) -> Result<T, SuppressionError> {
    let statement = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    if connection.execute(statement).await.is_err() {
        connection.close_on_drop();
        return Err(unavailable(operation));
    }
    result
}

fn as_i64(value: usize, field: &'static str) -> Result<i64, SuppressionError> {
    i64::try_from(value).map_err(|_| SuppressionError::InvalidConfiguration(field))
}

fn volatile_database_url(database_url: &str, filename: &Path) -> bool {
    let filename = filename.as_os_str().to_string_lossy();
    let memory_mode = database_url
        .split_once('?')
        .map(|(_, query)| {
            url::form_urlencoded::parse(query.as_bytes()).any(|(key, value)| {
                key.eq_ignore_ascii_case("mode") && value.eq_ignore_ascii_case("memory")
            })
        })
        .unwrap_or(false);
    database_url.eq_ignore_ascii_case("sqlite::memory:")
        || database_url.eq_ignore_ascii_case("sqlite://:memory:")
        || filename.is_empty()
        || filename.eq_ignore_ascii_case(":memory:")
        || filename.eq_ignore_ascii_case("file::memory:")
        || memory_mode
}

fn reject_existing_unsafe_target(path: &Path) -> Result<(), SuppressionError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            SuppressionError::InvalidConfiguration("target must be a regular file"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(unavailable("inspect database target")),
    }
}

#[cfg(test)]
mod tests;
