pub(super) const PRIVACY_REQUEST_WORKER_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::UserContext;

const HARD_MAX_ATTEMPTS: i32 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyRequestClaim {
    pub id: i32,
    pub request_key: String,
    pub school_id: i32,
    pub subject_user_id: i32,
    pub request_kind: String,
    pub claim_key: String,
    pub attempts: i32,
}

#[derive(Debug)]
pub enum PrivacyWorkerError {
    Forbidden,
    InvalidField(&'static str),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for PrivacyWorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("privacy worker access denied"),
            Self::InvalidField(field) => write!(formatter, "invalid privacy worker field: {field}"),
            Self::Database(error) => write!(formatter, "privacy worker database error: {error}"),
        }
    }
}

impl std::error::Error for PrivacyWorkerError {}

impl From<rullst_orm::Error> for PrivacyWorkerError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn actor_id(context: &UserContext) -> Result<i32, PrivacyWorkerError> {
    if !context.has_role("admin") && !context.has_role("school_owner") {
        return Err(PrivacyWorkerError::Forbidden);
    }
    context.user_id.parse::<i32>().ok().filter(|actor| *actor > 0)
        .ok_or(PrivacyWorkerError::Forbidden)
}

async fn school_id(context: &UserContext) -> Result<i32, PrivacyWorkerError> {
    school_service::context_school_id(context).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => PrivacyWorkerError::Database(error),
        _ => PrivacyWorkerError::Forbidden,
    })
}

/// Claims one due request under a bounded renewable database lease.
///
/// Expired processing claims are recoverable. A concurrent loser returns
/// `None` and may poll again; no request is executed inside this function.
pub async fn claim_next_at(
    context: &UserContext,
    claim_key: &str,
    observed_at_epoch: i64,
    lease_seconds: i64,
) -> Result<Option<PrivacyRequestClaim>, PrivacyWorkerError> {
    let actor = actor_id(context)?;
    if !valid_key(claim_key, 128) || observed_at_epoch <= 0 || !(1..=3_600).contains(&lease_seconds) {
        return Err(PrivacyWorkerError::InvalidField("claim"));
    }
    let school_id = school_id(context).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let select_sql = match driver {
        "postgres" => "SELECT id, request_key, subject_user_id, request_kind, attempts FROM privacy_requests WHERE school_id = $1 AND ((status IN ($2, $3) AND available_at_epoch <= $4) OR (status = $5 AND claim_expires_at_epoch < $6)) ORDER BY requested_at_epoch ASC, id ASC LIMIT 1",
        _ => "SELECT id, request_key, subject_user_id, request_kind, attempts FROM privacy_requests WHERE school_id = ? AND ((status IN (?, ?) AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch < ?)) ORDER BY requested_at_epoch ASC, id ASC LIMIT 1",
    };
    let candidate = rullst::db::sqlx::query_as::<_, (i32, String, i32, String, i32)>(select_sql)
        .bind(school_id).bind("pending").bind("retry").bind(observed_at_epoch)
        .bind("processing").bind(observed_at_epoch).fetch_optional(pool).await
        .map_err(|error| PrivacyWorkerError::Database(error.into()))?;
    let Some(candidate) = candidate else { return Ok(None); };
    if candidate.0 <= 0 || candidate.2 <= 0 || candidate.4 < 0
        || !valid_key(&candidate.1, 128) || !matches!(candidate.3.as_str(), "export" | "delete")
    {
        return Err(PrivacyWorkerError::InvalidField("stored request"));
    }
    if candidate.4 >= HARD_MAX_ATTEMPTS {
        let dead_letter_sql = match driver {
            "postgres" => "UPDATE privacy_requests SET status = $1, processed_by_user_id = $2, claim_key = $3, claim_expires_at_epoch = 0, last_error_code = $4, updated_at = CURRENT_TIMESTAMP WHERE id = $5 AND school_id = $6 AND attempts = $7 AND ((status IN ($8, $9) AND available_at_epoch <= $10) OR (status = $11 AND claim_expires_at_epoch < $12))",
            _ => "UPDATE privacy_requests SET status = ?, processed_by_user_id = ?, claim_key = ?, claim_expires_at_epoch = 0, last_error_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND attempts = ? AND ((status IN (?, ?) AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch < ?))",
        };
        rullst::db::sqlx::query(dead_letter_sql)
            .bind("dead_letter").bind(actor).bind("").bind("claim-expired-at-limit")
            .bind(candidate.0).bind(school_id).bind(candidate.4)
            .bind("pending").bind("retry").bind(observed_at_epoch)
            .bind("processing").bind(observed_at_epoch).execute(pool).await
            .map_err(|error| PrivacyWorkerError::Database(error.into()))?;
        return Ok(None);
    }
    let claim_expires_at_epoch = observed_at_epoch.saturating_add(lease_seconds);
    let update_sql = match driver {
        "postgres" => "UPDATE privacy_requests SET status = $1, claim_key = $2, claim_expires_at_epoch = $3, attempts = attempts + 1, updated_at = CURRENT_TIMESTAMP WHERE id = $4 AND school_id = $5 AND ((status IN ($6, $7) AND available_at_epoch <= $8) OR (status = $9 AND claim_expires_at_epoch < $10))",
        _ => "UPDATE privacy_requests SET status = ?, claim_key = ?, claim_expires_at_epoch = ?, attempts = attempts + 1, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND ((status IN (?, ?) AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch < ?))",
    };
    let claimed = rullst::db::sqlx::query(update_sql).bind("processing").bind(claim_key)
        .bind(claim_expires_at_epoch).bind(candidate.0).bind(school_id)
        .bind("pending").bind("retry").bind(observed_at_epoch)
        .bind("processing").bind(observed_at_epoch).execute(pool).await
        .map_err(|error| PrivacyWorkerError::Database(error.into()))?;
    if claimed.rows_affected() != 1 { return Ok(None); }
    Ok(Some(PrivacyRequestClaim {
        id: candidate.0,
        request_key: candidate.1,
        school_id,
        subject_user_id: candidate.2,
        request_kind: candidate.3,
        claim_key: claim_key.to_string(),
        attempts: candidate.4.saturating_add(1),
    }))
}

/// Records completion only for the exact live claim after the host application
/// has fulfilled the export/deletion policy and computed its canonical digest.
pub async fn complete_at(
    context: &UserContext,
    request_id: i32,
    claim_key: &str,
    completed_at_epoch: i64,
    result_digest: &str,
) -> Result<bool, PrivacyWorkerError> {
    let actor = actor_id(context)?;
    if request_id <= 0 || completed_at_epoch <= 0 || !valid_key(claim_key, 128)
        || !valid_digest(result_digest)
    {
        return Err(PrivacyWorkerError::InvalidField("completion"));
    }
    let school_id = school_id(context).await?;
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "UPDATE privacy_requests SET status = $1, processed_by_user_id = $2, completed_at_epoch = $3, result_digest = $4, claim_key = $5, claim_expires_at_epoch = 0, last_error_code = $6, updated_at = CURRENT_TIMESTAMP WHERE id = $7 AND school_id = $8 AND status = $9 AND claim_key = $10 AND claim_expires_at_epoch >= $11",
        _ => "UPDATE privacy_requests SET status = ?, processed_by_user_id = ?, completed_at_epoch = ?, result_digest = ?, claim_key = ?, claim_expires_at_epoch = 0, last_error_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch >= ?",
    };
    rullst::db::sqlx::query(sql).bind("completed").bind(actor).bind(completed_at_epoch)
        .bind(result_digest).bind("").bind("").bind(request_id).bind(school_id)
        .bind("processing").bind(claim_key).bind(completed_at_epoch)
        .execute(rullst::db::Orm::pool()?).await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| PrivacyWorkerError::Database(error.into()))
}

/// Schedules bounded retry or dead-letter for the exact live claim.
pub async fn fail_at(
    context: &UserContext,
    request_id: i32,
    claim_key: &str,
    failure_code: &str,
    observed_at_epoch: i64,
    retry_delay_seconds: i64,
    max_attempts: i32,
) -> Result<bool, PrivacyWorkerError> {
    let actor = actor_id(context)?;
    if request_id <= 0 || observed_at_epoch <= 0 || !valid_key(claim_key, 128)
        || !valid_key(failure_code, 64) || !(0..=86_400).contains(&retry_delay_seconds)
        || !(1..=10).contains(&max_attempts)
    {
        return Err(PrivacyWorkerError::InvalidField("failure"));
    }
    let school_id = school_id(context).await?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let select_sql = match driver {
        "postgres" => "SELECT attempts FROM privacy_requests WHERE id = $1 AND school_id = $2 AND status = $3 AND claim_key = $4 AND claim_expires_at_epoch >= $5",
        _ => "SELECT attempts FROM privacy_requests WHERE id = ? AND school_id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch >= ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i32>(select_sql).bind(request_id)
        .bind(school_id).bind("processing").bind(claim_key).bind(observed_at_epoch)
        .fetch_optional(pool).await
        .map_err(|error| PrivacyWorkerError::Database(error.into()))?;
    let Some(attempts) = attempts else { return Ok(false); };
    let next_status = if attempts >= max_attempts { "dead_letter" } else { "retry" };
    let available_at_epoch = observed_at_epoch.saturating_add(retry_delay_seconds);
    let update_sql = match driver {
        "postgres" => "UPDATE privacy_requests SET status = $1, processed_by_user_id = $2, available_at_epoch = $3, claim_key = $4, claim_expires_at_epoch = 0, last_error_code = $5, updated_at = CURRENT_TIMESTAMP WHERE id = $6 AND school_id = $7 AND status = $8 AND claim_key = $9 AND claim_expires_at_epoch >= $10 AND attempts = $11",
        _ => "UPDATE privacy_requests SET status = ?, processed_by_user_id = ?, available_at_epoch = ?, claim_key = ?, claim_expires_at_epoch = 0, last_error_code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND school_id = ? AND status = ? AND claim_key = ? AND claim_expires_at_epoch >= ? AND attempts = ?",
    };
    rullst::db::sqlx::query(update_sql).bind(next_status).bind(actor).bind(available_at_epoch)
        .bind("").bind(failure_code).bind(request_id).bind(school_id).bind("processing")
        .bind(claim_key).bind(observed_at_epoch).bind(attempts).execute(pool).await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| PrivacyWorkerError::Database(error.into()))
}
"##;

#[cfg(test)]
mod tests {
    use super::PRIVACY_REQUEST_WORKER_SERVICE;

    #[test]
    fn worker_is_school_scoped_exactly_claimed_and_result_bound() {
        for required in [
            "pub async fn claim_next_at",
            "pub async fn complete_at",
            "pub async fn fail_at",
            "claim_expires_at_epoch",
            "result_digest",
            "dead_letter",
            "HARD_MAX_ATTEMPTS",
            "claim-expired-at-limit",
            "school_id =",
        ] {
            assert!(PRIVACY_REQUEST_WORKER_SERVICE.contains(required));
        }
    }
}
