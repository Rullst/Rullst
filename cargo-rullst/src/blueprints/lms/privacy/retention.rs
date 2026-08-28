pub(super) const PRIVACY_RETENTION_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::UserContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionSweep {
    pub policies_marked: u64,
    pub requests_created: u64,
}

#[derive(Debug)]
pub enum RetentionError {
    Forbidden,
    InvalidPolicy,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("privacy retention access denied"),
            Self::InvalidPolicy => formatter.write_str("invalid privacy retention sweep policy"),
            Self::Database(error) => write!(formatter, "privacy retention database error: {error}"),
        }
    }
}

impl std::error::Error for RetentionError {}

impl From<rullst_orm::Error> for RetentionError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn actor_id(context: &UserContext) -> Result<i32, RetentionError> {
    context.user_id.parse::<i32>().ok().filter(|actor| *actor > 0)
        .ok_or(RetentionError::Forbidden)
}

/// Marks expired policies and schedules durable, idempotent delete requests.
///
/// This deliberately does not erase application rows. A product-owned worker
/// must fulfill each request under its legal/audit policy and record completion.
pub async fn schedule_expired_at(
    context: &UserContext,
    observed_at_epoch: i64,
    batch_limit: i64,
) -> Result<RetentionSweep, RetentionError> {
    if !context.has_role("admin") && !context.has_role("school_owner") {
        return Err(RetentionError::Forbidden);
    }
    if observed_at_epoch <= 0 || !(1..=100).contains(&batch_limit) {
        return Err(RetentionError::InvalidPolicy);
    }
    let actor = actor_id(context)?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => RetentionError::Database(error),
            _ => RetentionError::Forbidden,
        })?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let select_sql = match driver {
        "postgres" => "SELECT id, subject_user_id FROM privacy_subject_policies WHERE school_id = $1 AND status = $2 AND retention_until_epoch <= $3 ORDER BY id ASC LIMIT $4",
        _ => "SELECT id, subject_user_id FROM privacy_subject_policies WHERE school_id = ? AND status = ? AND retention_until_epoch <= ? ORDER BY id ASC LIMIT ?",
    };
    let candidates = rullst::db::sqlx::query_as::<_, (i32, i32)>(select_sql)
        .bind(school_id).bind("active").bind(observed_at_epoch).bind(batch_limit)
        .fetch_all(pool).await
        .map_err(|error| RetentionError::Database(error.into()))?;
    if candidates.iter().any(|row| row.0 <= 0 || row.1 <= 0) {
        return Err(RetentionError::InvalidPolicy);
    }

    let mut transaction = pool.begin().await
        .map_err(|error| RetentionError::Database(error.into()))?;
    let mut policies_marked = 0_u64;
    let mut requests_created = 0_u64;
    for (policy_id, subject_user_id) in candidates {
        let update_sql = match driver {
            "postgres" => "UPDATE privacy_subject_policies SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND school_id = $3 AND status = $4 AND retention_until_epoch <= $5",
            _ => "UPDATE privacy_subject_policies SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND status = ? AND retention_until_epoch <= ?",
        };
        let update = rullst::db::sqlx::query(update_sql).bind("retention_due")
            .bind(policy_id).bind(school_id).bind("active").bind(observed_at_epoch)
            .execute(&mut *transaction).await
            .map_err(|error| RetentionError::Database(error.into()))?;
        if update.rows_affected() != 1 { continue; }
        policies_marked = policies_marked.saturating_add(1);

        let request_key = format!("retention:{school_id}:{policy_id}");
        let insert_sql = match driver {
            "postgres" => "INSERT INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, 0, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
            "mysql" => "INSERT IGNORE INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            _ => "INSERT INTO privacy_requests (request_key, school_id, subject_user_id, requested_by_user_id, request_kind, status, requested_at_epoch, completed_at_epoch, result_digest, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 0, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        };
        let inserted = rullst::db::sqlx::query(insert_sql).bind(request_key).bind(school_id)
            .bind(subject_user_id).bind(actor).bind("delete").bind("pending")
            .bind(observed_at_epoch).execute(&mut *transaction).await
            .map_err(|error| RetentionError::Database(error.into()))?;
        requests_created = requests_created.saturating_add(inserted.rows_affected());
    }
    transaction.commit().await.map_err(|error| RetentionError::Database(error.into()))?;
    Ok(RetentionSweep { policies_marked, requests_created })
}
"##;
