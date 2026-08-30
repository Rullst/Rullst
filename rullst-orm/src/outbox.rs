//! Durable, database-backed transactional outbox.

use serde_json::Value;

use crate::{Error, FromRow, Orm};

const MAX_KEY_LEN: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 1_048_576;
const MAX_ERROR_LEN: usize = 512;

/// Result of an idempotent outbox enqueue operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnqueuedOutboxEvent {
    /// Stable database identifier of the new or pre-existing event.
    pub id: i64,
    /// Whether this call inserted the row rather than finding its idempotency key.
    pub inserted: bool,
}

/// Event exclusively claimed by one worker until its lease expires.
#[derive(Clone, Debug, FromRow)]
pub struct ClaimedOutboxEvent {
    /// Stable database identifier.
    pub id: i64,
    /// Explicit application or tenant stream.
    pub stream: String,
    /// Idempotency key unique within the stream.
    pub event_key: String,
    /// Bounded event kind used for dispatch routing.
    pub event_kind: String,
    /// Serialized JSON payload.
    pub payload_json: String,
    /// Number of successful claims, including this one.
    pub attempts: i32,
    /// Unpredictable token required to acknowledge or fail this claim.
    pub claim_key: String,
    /// Unix timestamp when another worker may reclaim this event.
    pub claim_expires_at_epoch: i64,
}

impl ClaimedOutboxEvent {
    /// Parses the bounded payload as JSON.
    pub fn payload(&self) -> Result<Value, Error> {
        serde_json::from_str(&self.payload_json).map_err(Into::into)
    }
}

/// Stateless facade for the durable outbox table.
pub struct Outbox;

/// Versioned migration for applications that use the built-in migration runner.
pub struct OutboxMigration;

#[async_trait::async_trait]
impl crate::schema::migration::Migration for OutboxMigration {
    fn name(&self) -> &'static str {
        "m20260830_000001_create_rullst_outbox"
    }

    async fn up(&self) -> Result<(), Error> {
        Outbox::install().await
    }

    async fn down(&self) -> Result<(), Error> {
        sqlx::query("DROP TABLE IF EXISTS rullst_outbox")
            .execute(Orm::pool()?)
            .await?;
        Ok(())
    }
}

impl Outbox {
    /// Installs the portable outbox table and its delivery index.
    ///
    /// Release migrations should normally own this DDL. This helper is useful
    /// for explicit application setup and tests; it never runs implicitly.
    pub async fn install() -> Result<(), Error> {
        let driver = Orm::driver()?;
        let table_sql = match driver {
            "postgres" => POSTGRES_TABLE,
            "mysql" => MYSQL_TABLE,
            _ => SQLITE_TABLE,
        };
        sqlx::query(table_sql).execute(Orm::pool()?).await?;
        if driver != "mysql" {
            sqlx::query(
                "CREATE INDEX IF NOT EXISTS rullst_outbox_delivery_idx \
                 ON rullst_outbox (stream, status, available_at_epoch, claim_expires_at_epoch, id)",
            )
            .execute(Orm::pool()?)
            .await?;
        }
        Ok(())
    }

    /// Enqueues an event inside the current [`Orm::transaction`].
    ///
    /// Refusing to open an implicit transaction prevents the domain mutation
    /// and its outbox row from accidentally committing independently.
    pub async fn enqueue(
        stream: impl Into<String>,
        event_key: impl Into<String>,
        event_kind: impl Into<String>,
        payload: &Value,
    ) -> Result<EnqueuedOutboxEvent, Error> {
        let stream = stream.into();
        let event_key = event_key.into();
        let event_kind = event_kind.into();
        let transaction = crate::CURRENT_TX
            .try_with(Clone::clone)
            .map_err(|_| missing_transaction_error())?;
        let mut transaction = transaction.lock().await;
        let transaction = transaction.as_mut().ok_or_else(missing_transaction_error)?;
        Self::enqueue_with_tx_inner(transaction, &stream, &event_key, &event_kind, payload).await
    }

    /// Enqueues through a caller-owned SQLx transaction.
    pub async fn enqueue_with_tx(
        transaction: &mut crate::db::Transaction<'_>,
        stream: impl Into<String>,
        event_key: impl Into<String>,
        event_kind: impl Into<String>,
        payload: &Value,
    ) -> Result<EnqueuedOutboxEvent, Error> {
        let stream = stream.into();
        let event_key = event_key.into();
        let event_kind = event_kind.into();
        Self::enqueue_with_tx_inner(transaction, &stream, &event_key, &event_kind, payload).await
    }

    async fn enqueue_with_tx_inner(
        transaction: &mut crate::db::Transaction<'_>,
        stream: &str,
        event_key: &str,
        event_kind: &str,
        payload: &Value,
    ) -> Result<EnqueuedOutboxEvent, Error> {
        validate_key("stream", stream)?;
        validate_key("event_key", event_key)?;
        validate_key("event_kind", event_kind)?;
        let payload_json = serde_json::to_string(payload)?;
        if payload_json.len() > MAX_PAYLOAD_BYTES {
            return Err(Error::Validation(format!(
                "outbox payload exceeds {MAX_PAYLOAD_BYTES} bytes"
            )));
        }
        let now = unix_now()?;
        let insert_token = uuid::Uuid::new_v4().simple().to_string();
        let driver = Orm::driver()?;
        let insert_sql = match driver {
            "postgres" => POSTGRES_INSERT,
            "mysql" => MYSQL_INSERT,
            _ => SQLITE_INSERT,
        };
        sqlx::query(insert_sql)
            .bind(stream)
            .bind(event_key)
            .bind(event_kind)
            .bind(&payload_json)
            .bind("pending")
            .bind(0_i32)
            .bind("")
            .bind(0_i64)
            .bind("")
            .bind(now)
            .bind(now)
            .bind(&insert_token)
            .execute(&mut **transaction)
            .await?;
        let select_sql = if driver == "postgres" {
            "SELECT id, event_kind, payload_json, insert_token FROM rullst_outbox WHERE stream = $1 AND event_key = $2"
        } else {
            "SELECT id, event_kind, payload_json, insert_token FROM rullst_outbox WHERE stream = ? AND event_key = ?"
        };
        let (id, stored_kind, stored_payload, stored_insert_token) =
            sqlx::query_as::<_, (i64, String, String, String)>(select_sql)
                .bind(stream)
                .bind(event_key)
                .fetch_one(&mut **transaction)
                .await?;
        if stored_kind != event_kind || stored_payload != payload_json {
            return Err(Error::Validation(format!(
                "outbox idempotency key '{event_key}' already exists in stream '{stream}' with different content"
            )));
        }
        let inserted = stored_insert_token == insert_token;
        Ok(EnqueuedOutboxEvent { id, inserted })
    }

    /// Claims one pending or lease-expired event for a stream.
    pub async fn claim_next(
        stream: impl Into<String>,
        worker_id: impl Into<String>,
        lease_seconds: i64,
        max_attempts: i32,
    ) -> Result<Option<ClaimedOutboxEvent>, Error> {
        let stream = stream.into();
        let worker_id = worker_id.into();
        Self::claim_next_at_inner(
            &stream,
            &worker_id,
            unix_now()?,
            lease_seconds,
            max_attempts,
        )
        .await
    }

    #[doc(hidden)]
    pub async fn claim_next_at(
        stream: impl Into<String>,
        worker_id: impl Into<String>,
        now_epoch_seconds: i64,
        lease_seconds: i64,
        max_attempts: i32,
    ) -> Result<Option<ClaimedOutboxEvent>, Error> {
        let stream = stream.into();
        let worker_id = worker_id.into();
        Self::claim_next_at_inner(
            &stream,
            &worker_id,
            now_epoch_seconds,
            lease_seconds,
            max_attempts,
        )
        .await
    }

    async fn claim_next_at_inner(
        stream: &str,
        worker_id: &str,
        now_epoch_seconds: i64,
        lease_seconds: i64,
        max_attempts: i32,
    ) -> Result<Option<ClaimedOutboxEvent>, Error> {
        validate_key("stream", stream)?;
        validate_key("worker_id", worker_id)?;
        if now_epoch_seconds <= 0
            || !(1..=3_600).contains(&lease_seconds)
            || !(1..=100).contains(&max_attempts)
        {
            return Err(Error::Validation(
                "outbox claim timestamp, lease or attempt limit is outside its bound".to_string(),
            ));
        }
        let claim_expires_at_epoch = now_epoch_seconds
            .checked_add(lease_seconds)
            .ok_or_else(|| Error::Validation("outbox claim lease overflowed".to_string()))?;
        let claim_key = uuid::Uuid::new_v4().simple().to_string();
        let driver = Orm::driver()?;
        let mut transaction = Orm::begin_transaction().await?;
        let exhaust_sql = if driver == "postgres" {
            POSTGRES_EXHAUST
        } else {
            PORTABLE_EXHAUST
        };
        sqlx::query(exhaust_sql)
            .bind("dead_letter")
            .bind("")
            .bind("")
            .bind(0_i64)
            .bind("claim attempt limit reached")
            .bind(stream)
            .bind(max_attempts)
            .bind("pending")
            .bind("processing")
            .bind(now_epoch_seconds)
            .execute(&mut *transaction)
            .await?;

        let select_sql = match driver {
            "postgres" => POSTGRES_CLAIM_SELECT,
            "mysql" => MYSQL_CLAIM_SELECT,
            _ => PORTABLE_CLAIM_SELECT,
        };
        let candidate = sqlx::query_scalar::<_, i64>(select_sql)
            .bind(stream)
            .bind("pending")
            .bind(now_epoch_seconds)
            .bind("processing")
            .bind(now_epoch_seconds)
            .bind(max_attempts)
            .fetch_optional(&mut *transaction)
            .await?;
        let Some(id) = candidate else {
            transaction.commit().await?;
            return Ok(None);
        };

        let update_sql = if driver == "postgres" {
            POSTGRES_CLAIM_UPDATE
        } else {
            PORTABLE_CLAIM_UPDATE
        };
        let claimed = sqlx::query(update_sql)
            .bind("processing")
            .bind(worker_id)
            .bind(&claim_key)
            .bind(claim_expires_at_epoch)
            .bind("")
            .bind(id)
            .bind(stream)
            .bind("pending")
            .bind(now_epoch_seconds)
            .bind("processing")
            .bind(now_epoch_seconds)
            .bind(max_attempts)
            .execute(&mut *transaction)
            .await?
            .rows_affected()
            == 1;
        if !claimed {
            transaction.rollback().await?;
            return Ok(None);
        }

        let fetch_sql = if driver == "postgres" {
            POSTGRES_CLAIM_FETCH
        } else {
            PORTABLE_CLAIM_FETCH
        };
        let event = sqlx::query_as::<_, ClaimedOutboxEvent>(fetch_sql)
            .bind(id)
            .bind(stream)
            .bind("processing")
            .bind(&claim_key)
            .fetch_one(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(Some(event))
    }

    /// Marks an event delivered only when the exact claim token still owns it.
    pub async fn acknowledge(id: i64, claim_key: impl Into<String>) -> Result<bool, Error> {
        let claim_key = claim_key.into();
        transition(id, &claim_key, Transition::Delivered).await
    }

    /// Releases an event for retry or dead-letters it at the attempt limit.
    pub async fn fail(
        id: i64,
        claim_key: impl Into<String>,
        error: impl Into<String>,
        max_attempts: i32,
        retry_delay_seconds: i64,
    ) -> Result<bool, Error> {
        let claim_key = claim_key.into();
        let error = error.into();
        let retry_at_epoch = unix_now()?
            .checked_add(retry_delay_seconds)
            .ok_or_else(|| Error::Validation("outbox retry timestamp overflowed".to_string()))?;
        transition(
            id,
            &claim_key,
            Transition::Failed {
                error: &error,
                max_attempts,
                retry_at_epoch,
                retry_delay_seconds,
            },
        )
        .await
    }
}

enum Transition<'a> {
    Delivered,
    Failed {
        error: &'a str,
        max_attempts: i32,
        retry_at_epoch: i64,
        retry_delay_seconds: i64,
    },
}

async fn transition(id: i64, claim_key: &str, transition: Transition<'_>) -> Result<bool, Error> {
    if id <= 0 {
        return Err(Error::Validation("outbox id must be positive".to_string()));
    }
    validate_key("claim_key", claim_key)?;
    let driver = Orm::driver()?;
    let now_epoch_seconds = unix_now()?;
    let result = match transition {
        Transition::Delivered => {
            let sql = if driver == "postgres" {
                POSTGRES_ACK
            } else {
                PORTABLE_ACK
            };
            sqlx::query(sql)
                .bind("delivered")
                .bind("")
                .bind("")
                .bind(0_i64)
                .bind(now_epoch_seconds)
                .bind(id)
                .bind("processing")
                .bind(claim_key)
                .bind(now_epoch_seconds)
                .execute(Orm::pool()?)
                .await?
        }
        Transition::Failed {
            error,
            max_attempts,
            retry_at_epoch,
            retry_delay_seconds,
        } => {
            if !(1..=100).contains(&max_attempts)
                || !(0..=86_400).contains(&retry_delay_seconds)
                || error.is_empty()
                || error.len() > MAX_ERROR_LEN
                || error.chars().any(char::is_control)
            {
                return Err(Error::Validation(
                    "outbox failure policy is outside its bound".to_string(),
                ));
            }
            let sql = if driver == "postgres" {
                POSTGRES_FAIL
            } else {
                PORTABLE_FAIL
            };
            sqlx::query(sql)
                .bind(max_attempts)
                .bind("dead_letter")
                .bind("pending")
                .bind(retry_at_epoch)
                .bind("")
                .bind("")
                .bind(0_i64)
                .bind(error)
                .bind(id)
                .bind("processing")
                .bind(claim_key)
                .bind(now_epoch_seconds)
                .execute(Orm::pool()?)
                .await?
        }
    };
    Ok(result.rows_affected() == 1)
}

fn validate_key(field: &str, value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > MAX_KEY_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(Error::Validation(format!(
            "outbox {field} must contain 1-{MAX_KEY_LEN} ASCII letters, digits, '.', '-', '_' or ':'"
        )));
    }
    Ok(())
}

fn unix_now() -> Result<i64, Error> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::Internal("system clock is before the Unix epoch".to_string()))?;
    i64::try_from(elapsed.as_secs())
        .map_err(|_| Error::Internal("Unix timestamp exceeds i64".to_string()))
}

fn missing_transaction_error() -> Error {
    Error::Validation(
        "Outbox::enqueue must run inside Orm::transaction; use enqueue_with_tx for a caller-owned transaction"
            .to_string(),
    )
}

const POSTGRES_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id BIGSERIAL PRIMARY KEY, stream VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, event_kind VARCHAR(128) NOT NULL, payload_json TEXT NOT NULL, status VARCHAR(16) NOT NULL, attempts INTEGER NOT NULL, claimed_by VARCHAR(128) NOT NULL, claim_key VARCHAR(128) NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error VARCHAR(512) NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT, insert_token VARCHAR(32) NOT NULL, UNIQUE (stream, event_key))";
const MYSQL_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id BIGINT AUTO_INCREMENT PRIMARY KEY, stream VARCHAR(128) NOT NULL, event_key VARCHAR(128) NOT NULL, event_kind VARCHAR(128) NOT NULL, payload_json LONGTEXT NOT NULL, status VARCHAR(16) NOT NULL, attempts INT NOT NULL, claimed_by VARCHAR(128) NOT NULL, claim_key VARCHAR(128) NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error VARCHAR(512) NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT NULL, insert_token VARCHAR(32) NOT NULL, UNIQUE KEY rullst_outbox_stream_event_unique (stream, event_key), INDEX rullst_outbox_delivery_idx (stream, status, available_at_epoch, claim_expires_at_epoch, id))";
const SQLITE_TABLE: &str = "CREATE TABLE IF NOT EXISTS rullst_outbox (id INTEGER PRIMARY KEY AUTOINCREMENT, stream TEXT NOT NULL, event_key TEXT NOT NULL, event_kind TEXT NOT NULL, payload_json TEXT NOT NULL, status TEXT NOT NULL, attempts INTEGER NOT NULL, claimed_by TEXT NOT NULL, claim_key TEXT NOT NULL, claim_expires_at_epoch BIGINT NOT NULL, last_error TEXT NOT NULL, available_at_epoch BIGINT NOT NULL, created_at_epoch BIGINT NOT NULL, delivered_at_epoch BIGINT, insert_token TEXT NOT NULL, UNIQUE (stream, event_key))";

const POSTGRES_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, '') ON CONFLICT (stream, event_key) DO NOTHING";
const MYSQL_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '') ON DUPLICATE KEY UPDATE id = LAST_INSERT_ID(id)";
const SQLITE_INSERT: &str = "INSERT INTO rullst_outbox (stream, event_key, event_kind, payload_json, status, attempts, claimed_by, claim_expires_at_epoch, last_error, available_at_epoch, created_at_epoch, insert_token, claim_key) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '') ON CONFLICT (stream, event_key) DO NOTHING";

const POSTGRES_EXHAUST: &str = "UPDATE rullst_outbox SET status = $1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5 WHERE stream = $6 AND attempts >= $7 AND (status = $8 OR (status = $9 AND claim_expires_at_epoch <= $10))";
const PORTABLE_EXHAUST: &str = "UPDATE rullst_outbox SET status = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE stream = ? AND attempts >= ? AND (status = ? OR (status = ? AND claim_expires_at_epoch <= ?))";
const POSTGRES_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = $1 AND ((status = $2 AND available_at_epoch <= $3) OR (status = $4 AND claim_expires_at_epoch <= $5)) AND attempts < $6 ORDER BY id ASC LIMIT 1 FOR UPDATE SKIP LOCKED";
const MYSQL_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ? ORDER BY id ASC LIMIT 1 FOR UPDATE SKIP LOCKED";
const PORTABLE_CLAIM_SELECT: &str = "SELECT id FROM rullst_outbox WHERE stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ? ORDER BY id ASC LIMIT 1";
const POSTGRES_CLAIM_UPDATE: &str = "UPDATE rullst_outbox SET status = $1, attempts = attempts + 1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5 WHERE id = $6 AND stream = $7 AND ((status = $8 AND available_at_epoch <= $9) OR (status = $10 AND claim_expires_at_epoch <= $11)) AND attempts < $12";
const PORTABLE_CLAIM_UPDATE: &str = "UPDATE rullst_outbox SET status = ?, attempts = attempts + 1, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE id = ? AND stream = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?)) AND attempts < ?";
const POSTGRES_CLAIM_FETCH: &str = "SELECT id, stream, event_key, event_kind, payload_json, attempts, claim_key, claim_expires_at_epoch FROM rullst_outbox WHERE id = $1 AND stream = $2 AND status = $3 AND claim_key = $4";
const PORTABLE_CLAIM_FETCH: &str = "SELECT id, stream, event_key, event_kind, payload_json, attempts, claim_key, claim_expires_at_epoch FROM rullst_outbox WHERE id = ? AND stream = ? AND status = ? AND claim_key = ?";
const POSTGRES_ACK: &str = "UPDATE rullst_outbox SET status = $1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, delivered_at_epoch = $5 WHERE id = $6 AND status = $7 AND claim_key = $8 AND claim_expires_at_epoch > $9";
const PORTABLE_ACK: &str = "UPDATE rullst_outbox SET status = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, delivered_at_epoch = ? WHERE id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch > ?";
const POSTGRES_FAIL: &str = "UPDATE rullst_outbox SET status = CASE WHEN attempts >= $1 THEN $2 ELSE $3 END, available_at_epoch = $4, claimed_by = $5, claim_key = $6, claim_expires_at_epoch = $7, last_error = $8 WHERE id = $9 AND status = $10 AND claim_key = $11 AND claim_expires_at_epoch > $12";
const PORTABLE_FAIL: &str = "UPDATE rullst_outbox SET status = CASE WHEN attempts >= ? THEN ? ELSE ? END, available_at_epoch = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ? WHERE id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch > ?";
