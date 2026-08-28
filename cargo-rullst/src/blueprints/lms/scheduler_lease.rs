// Shared database lease for multi-instance Academy schedulers.

#[path = "publication_scheduler.rs"]
mod publication_scheduler;

pub fn get_files() -> Vec<(&'static str, String)> {
    let mut files = vec![
        (
            "src/models/scheduler_lease.rs",
            SCHEDULER_LEASE_MODEL.to_string(),
        ),
        (
            "src/services/scheduler_lease_service.rs",
            SCHEDULER_LEASE_SERVICE.to_string(),
        ),
        (
            "src/migrations/m20260831000000_add_scheduler_leases.rs",
            SCHEDULER_LEASE_MIGRATION.to_string(),
        ),
    ];
    files.extend(publication_scheduler::get_files());
    files
}

const SCHEDULER_LEASE_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "scheduler_leases")]
pub struct SchedulerLease {
    pub id: i32,
    pub lease_key: String,
    pub holder_id: String,
    pub lease_token: String,
    pub heartbeat_at_epoch: i64,
    pub expires_at_epoch: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for SchedulerLease {
    fn nexus_table() -> &'static str { "scheduler_leases" }
    fn nexus_label() -> &'static str { "Scheduler Leases" }
    fn nexus_icon() -> &'static str { "🫀" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "lease_key", label: "Lease", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "holder_id", label: "Holder", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "lease_token", label: "Token", kind: FieldKind::Text, hidden: true, readonly: true },
            FieldMeta { name: "heartbeat_at_epoch", label: "Heartbeat Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "expires_at_epoch", label: "Expires Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
        ]
    }
}
"##;

const SCHEDULER_LEASE_SERVICE: &str = r##"#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseSnapshot {
    pub lease_key: String,
    pub holder_id: String,
    pub heartbeat_at_epoch: i64,
    pub expires_at_epoch: i64,
}

#[derive(Debug)]
pub enum SchedulerLeaseError {
    InvalidField(&'static str),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for SchedulerLeaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid scheduler lease field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "scheduler lease database error: {error}"),
        }
    }
}

impl std::error::Error for SchedulerLeaseError {}

impl From<rullst_orm::Error> for SchedulerLeaseError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn unix_now() -> Result<i64, SchedulerLeaseError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| SchedulerLeaseError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| SchedulerLeaseError::Clock)
}

pub async fn acquire(
    lease_key: &str,
    holder_id: &str,
    lease_token: &str,
    lease_seconds: i64,
) -> Result<bool, SchedulerLeaseError> {
    acquire_at(lease_key, holder_id, lease_token, unix_now()?, lease_seconds).await
}

pub async fn acquire_at(
    lease_key: &str,
    holder_id: &str,
    lease_token: &str,
    now_epoch_seconds: i64,
    lease_seconds: i64,
) -> Result<bool, SchedulerLeaseError> {
    validate(lease_key, holder_id, lease_token, now_epoch_seconds, lease_seconds)?;
    let expires_at_epoch = now_epoch_seconds
        .checked_add(lease_seconds)
        .ok_or(SchedulerLeaseError::InvalidField("lease duration"))?;
    let driver = rullst::db::Orm::driver()?;
    let update_sql = match driver {
        "postgres" => "UPDATE scheduler_leases SET holder_id = $1, lease_token = $2, heartbeat_at_epoch = $3, expires_at_epoch = $4, updated_at = CURRENT_TIMESTAMP WHERE lease_key = $5 AND (expires_at_epoch <= $6 OR (holder_id = $7 AND lease_token = $8))",
        _ => "UPDATE scheduler_leases SET holder_id = ?, lease_token = ?, heartbeat_at_epoch = ?, expires_at_epoch = ?, updated_at = CURRENT_TIMESTAMP WHERE lease_key = ? AND (expires_at_epoch <= ? OR (holder_id = ? AND lease_token = ?))",
    };
    let updated = rullst::db::sqlx::query(update_sql)
        .bind(holder_id)
        .bind(lease_token)
        .bind(now_epoch_seconds)
        .bind(expires_at_epoch)
        .bind(lease_key)
        .bind(now_epoch_seconds)
        .bind(holder_id)
        .bind(lease_token)
        .execute(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| SchedulerLeaseError::Database(error.into()))?
        .rows_affected() == 1;
    if updated { return Ok(true); }
    let insert_sql = match driver {
        "postgres" => "INSERT INTO scheduler_leases (lease_key, holder_id, lease_token, heartbeat_at_epoch, expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO scheduler_leases (lease_key, holder_id, lease_token, heartbeat_at_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO scheduler_leases (lease_key, holder_id, lease_token, heartbeat_at_epoch, expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(insert_sql)
        .bind(lease_key)
        .bind(holder_id)
        .bind(lease_token)
        .bind(now_epoch_seconds)
        .bind(expires_at_epoch)
        .execute(rullst::db::Orm::pool()?)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| SchedulerLeaseError::Database(error.into()))
}

pub async fn renew_at(
    lease_key: &str,
    holder_id: &str,
    lease_token: &str,
    now_epoch_seconds: i64,
    lease_seconds: i64,
) -> Result<bool, SchedulerLeaseError> {
    validate(lease_key, holder_id, lease_token, now_epoch_seconds, lease_seconds)?;
    let expires_at_epoch = now_epoch_seconds
        .checked_add(lease_seconds)
        .ok_or(SchedulerLeaseError::InvalidField("lease duration"))?;
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "UPDATE scheduler_leases SET heartbeat_at_epoch = $1, expires_at_epoch = $2, updated_at = CURRENT_TIMESTAMP WHERE lease_key = $3 AND holder_id = $4 AND lease_token = $5 AND expires_at_epoch > $6",
        _ => "UPDATE scheduler_leases SET heartbeat_at_epoch = ?, expires_at_epoch = ?, updated_at = CURRENT_TIMESTAMP WHERE lease_key = ? AND holder_id = ? AND lease_token = ? AND expires_at_epoch > ?",
    };
    rullst::db::sqlx::query(sql)
        .bind(now_epoch_seconds)
        .bind(expires_at_epoch)
        .bind(lease_key)
        .bind(holder_id)
        .bind(lease_token)
        .bind(now_epoch_seconds)
        .execute(rullst::db::Orm::pool()?)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| SchedulerLeaseError::Database(error.into()))
}

pub async fn release(
    lease_key: &str,
    holder_id: &str,
    lease_token: &str,
) -> Result<bool, SchedulerLeaseError> {
    if !valid_key(lease_key, 96) || !valid_key(holder_id, 64) || !valid_key(lease_token, 128) {
        return Err(SchedulerLeaseError::InvalidField("release identity"));
    }
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "UPDATE scheduler_leases SET holder_id = $1, lease_token = $2, heartbeat_at_epoch = 0, expires_at_epoch = 0, updated_at = CURRENT_TIMESTAMP WHERE lease_key = $3 AND holder_id = $4 AND lease_token = $5",
        _ => "UPDATE scheduler_leases SET holder_id = ?, lease_token = ?, heartbeat_at_epoch = 0, expires_at_epoch = 0, updated_at = CURRENT_TIMESTAMP WHERE lease_key = ? AND holder_id = ? AND lease_token = ?",
    };
    rullst::db::sqlx::query(sql)
        .bind("")
        .bind("")
        .bind(lease_key)
        .bind(holder_id)
        .bind(lease_token)
        .execute(rullst::db::Orm::pool()?)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| SchedulerLeaseError::Database(error.into()))
}

pub async fn snapshot(lease_key: &str) -> Result<Option<LeaseSnapshot>, SchedulerLeaseError> {
    if !valid_key(lease_key, 96) { return Err(SchedulerLeaseError::InvalidField("lease key")); }
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT lease_key, holder_id, heartbeat_at_epoch, expires_at_epoch FROM scheduler_leases WHERE lease_key = $1",
        _ => "SELECT lease_key, holder_id, heartbeat_at_epoch, expires_at_epoch FROM scheduler_leases WHERE lease_key = ?",
    };
    rullst::db::sqlx::query_as::<_, (String, String, i64, i64)>(sql)
        .bind(lease_key)
        .fetch_optional(rullst::db::Orm::pool()?)
        .await
        .map(|value| value.map(|row| LeaseSnapshot {
            lease_key: row.0,
            holder_id: row.1,
            heartbeat_at_epoch: row.2,
            expires_at_epoch: row.3,
        }))
        .map_err(|error| SchedulerLeaseError::Database(error.into()))
}

fn validate(
    lease_key: &str,
    holder_id: &str,
    lease_token: &str,
    now_epoch_seconds: i64,
    lease_seconds: i64,
) -> Result<(), SchedulerLeaseError> {
    if !valid_key(lease_key, 96)
        || !valid_key(holder_id, 64)
        || !valid_key(lease_token, 128)
        || now_epoch_seconds <= 0
        || !(1..=3_600).contains(&lease_seconds)
    {
        return Err(SchedulerLeaseError::InvalidField("acquire policy"));
    }
    Ok(())
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
        })
}
"##;

const SCHEDULER_LEASE_MIGRATION: &str = r##"use rullst::db::{Orm, sqlx};
use rullst::db::async_trait;
use rullst::db::schema::{Migration, Schema};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str { "m20260831000000_add_scheduler_leases" }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("scheduler_leases", |table| {
            table.id();
            table.string("lease_key").not_null();
            table.string("holder_id").not_null();
            table.string("lease_token").not_null();
            table.big_integer("heartbeat_at_epoch").not_null();
            table.big_integer("expires_at_epoch").not_null();
            table.timestamps();
        }).await?;
        let pool = Orm::pool()?;
        for statement in [
            "CREATE UNIQUE INDEX scheduler_leases_key_unique ON scheduler_leases(lease_key)",
            "CREATE INDEX scheduler_leases_expiry_idx ON scheduler_leases(expires_at_epoch)",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(statement)).execute(pool).await?;
        }
        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("scheduler_leases").await
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::SCHEDULER_LEASE_SERVICE;

    #[test]
    fn lease_uses_expiry_and_exact_holder_token_comparisons() {
        assert!(SCHEDULER_LEASE_SERVICE.contains("expires_at_epoch <="));
        assert!(SCHEDULER_LEASE_SERVICE.contains("holder_id = $4 AND lease_token = $5"));
        assert!(SCHEDULER_LEASE_SERVICE.contains("INSERT IGNORE INTO scheduler_leases"));
        assert!(!SCHEDULER_LEASE_SERVICE.contains("format!(\"UPDATE"));
    }
}
