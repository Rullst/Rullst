// Claim/retry/dead-letter service for the Academy transactional outbox.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![("src/services/outbox_service.rs", OUTBOX_SERVICE.to_string())]
}

const OUTBOX_SERVICE: &str = r##"#[derive(Debug, Clone, rullst::db::FromRow)]
pub struct ClaimedOutboxEvent {
    pub id: i32,
    pub school_id: i32,
    pub event_key: String,
    pub event_kind: String,
    pub subject_user_id: i32,
    pub payload_json: String,
    pub attempts: i32,
    pub claim_key: String,
    pub claim_expires_at_epoch: i64,
}

#[derive(Debug)]
pub enum OutboxError {
    InvalidField(&'static str),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for OutboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid outbox field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "outbox database error: {error}"),
        }
    }
}

impl std::error::Error for OutboxError {}

impl From<rullst_orm::Error> for OutboxError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

fn unix_now() -> Result<i64, OutboxError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| OutboxError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| OutboxError::Clock)
}

/// Claims one available event with a bounded lease. Expired processing claims
/// are reclaimed atomically, and the old claim token can no longer ACK them.
pub async fn claim_next(
    worker_id: &str,
    claim_key: &str,
    lease_seconds: i64,
) -> Result<Option<ClaimedOutboxEvent>, OutboxError> {
    claim_next_at(worker_id, claim_key, unix_now()?, lease_seconds).await
}

pub async fn claim_next_at(
    worker_id: &str,
    claim_key: &str,
    now_epoch_seconds: i64,
    lease_seconds: i64,
) -> Result<Option<ClaimedOutboxEvent>, OutboxError> {
    if !valid_key(worker_id, 64)
        || !valid_key(claim_key, 128)
        || now_epoch_seconds <= 0
        || !(1..=3_600).contains(&lease_seconds)
    {
        return Err(OutboxError::InvalidField("claim identity"));
    }
    let claim_expires_at_epoch = now_epoch_seconds
        .checked_add(lease_seconds)
        .ok_or(OutboxError::InvalidField("claim lease"))?;
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| OutboxError::Database(error.into()))?;
    let select_sql = match driver {
        "postgres" => "SELECT id FROM academy_outbox WHERE (status = $1 AND available_at_epoch <= $2) OR (status = $3 AND claim_expires_at_epoch <= $4) ORDER BY id ASC LIMIT 1",
        _ => "SELECT id FROM academy_outbox WHERE (status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?) ORDER BY id ASC LIMIT 1",
    };
    let candidate = rullst::db::sqlx::query_scalar::<_, i32>(select_sql)
        .bind("pending")
        .bind(now_epoch_seconds)
        .bind("processing")
        .bind(now_epoch_seconds)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| OutboxError::Database(error.into()))?;
    let Some(id) = candidate else {
        transaction
            .commit()
            .await
            .map_err(|error| OutboxError::Database(error.into()))?;
        return Ok(None);
    };

    let update_sql = match driver {
        "postgres" => "UPDATE academy_outbox SET status = $1, attempts = attempts + 1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5, updated_at = CURRENT_TIMESTAMP WHERE id = $6 AND ((status = $7 AND available_at_epoch <= $8) OR (status = $9 AND claim_expires_at_epoch <= $10))",
        _ => "UPDATE academy_outbox SET status = ?, attempts = attempts + 1, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND ((status = ? AND available_at_epoch <= ?) OR (status = ? AND claim_expires_at_epoch <= ?))",
    };
    let claimed = rullst::db::sqlx::query(update_sql)
        .bind("processing")
        .bind(worker_id)
        .bind(claim_key)
        .bind(claim_expires_at_epoch)
        .bind("")
        .bind(id)
        .bind("pending")
        .bind(now_epoch_seconds)
        .bind("processing")
        .bind(now_epoch_seconds)
        .execute(&mut *transaction)
        .await
        .map_err(|error| OutboxError::Database(error.into()))?
        .rows_affected()
        == 1;
    if !claimed {
        transaction
            .rollback()
            .await
            .map_err(|error| OutboxError::Database(error.into()))?;
        return Ok(None);
    }

    let fetch_sql = match driver {
        "postgres" => "SELECT id, school_id, event_key, event_kind, subject_user_id, payload_json, attempts, claim_key, claim_expires_at_epoch FROM academy_outbox WHERE id = $1 AND status = $2 AND claim_key = $3",
        _ => "SELECT id, school_id, event_key, event_kind, subject_user_id, payload_json, attempts, claim_key, claim_expires_at_epoch FROM academy_outbox WHERE id = ? AND status = ? AND claim_key = ?",
    };
    let event = rullst::db::sqlx::query_as::<_, ClaimedOutboxEvent>(fetch_sql)
        .bind(id)
        .bind("processing")
        .bind(claim_key)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| OutboxError::Database(error.into()))?;
    transaction
        .commit()
        .await
        .map_err(|error| OutboxError::Database(error.into()))?;
    Ok(Some(event))
}

/// Acknowledges only the event held by the exact claim token.
pub async fn acknowledge(id: i32, claim_key: &str) -> Result<bool, OutboxError> {
    transition(id, claim_key, "delivered", "", 1, 0).await
}

/// Releases for retry or dead-letters after `max_attempts` claims.
pub async fn fail(
    id: i32,
    claim_key: &str,
    error: &str,
    max_attempts: i32,
    retry_delay_seconds: i64,
) -> Result<bool, OutboxError> {
    fail_at(
        id,
        claim_key,
        error,
        max_attempts,
        unix_now()?,
        retry_delay_seconds,
    )
    .await
}

pub async fn fail_at(
    id: i32,
    claim_key: &str,
    error: &str,
    max_attempts: i32,
    now_epoch_seconds: i64,
    retry_delay_seconds: i64,
) -> Result<bool, OutboxError> {
    if !(1..=100).contains(&max_attempts)
        || now_epoch_seconds <= 0
        || !(0..=86_400).contains(&retry_delay_seconds)
        || error.is_empty()
        || error.len() > 512
        || error.chars().any(char::is_control)
    {
        return Err(OutboxError::InvalidField("failure policy"));
    }
    let retry_at_epoch = now_epoch_seconds
        .checked_add(retry_delay_seconds)
        .ok_or(OutboxError::InvalidField("retry schedule"))?;
    transition(id, claim_key, "pending", error, max_attempts, retry_at_epoch).await
}

async fn transition(
    id: i32,
    claim_key: &str,
    success_status: &str,
    error: &str,
    max_attempts: i32,
    retry_at_epoch: i64,
) -> Result<bool, OutboxError> {
    if id <= 0 || !valid_key(claim_key, 128) {
        return Err(OutboxError::InvalidField("claim"));
    }
    let driver = rullst::db::Orm::driver()?;
    let sql = if success_status == "delivered" {
        match driver {
            "postgres" => "UPDATE academy_outbox SET status = $1, claimed_by = $2, claim_key = $3, claim_expires_at_epoch = $4, last_error = $5, updated_at = CURRENT_TIMESTAMP WHERE id = $6 AND status = $7 AND claim_key = $8",
            _ => "UPDATE academy_outbox SET status = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = ? AND claim_key = ?",
        }
    } else {
        match driver {
            "postgres" => "UPDATE academy_outbox SET status = CASE WHEN attempts >= $1 THEN $2 ELSE $3 END, available_at = CURRENT_TIMESTAMP, available_at_epoch = $4, claimed_by = $5, claim_key = $6, claim_expires_at_epoch = $7, last_error = $8, updated_at = CURRENT_TIMESTAMP WHERE id = $9 AND status = $10 AND claim_key = $11",
            _ => "UPDATE academy_outbox SET status = CASE WHEN attempts >= ? THEN ? ELSE ? END, available_at = CURRENT_TIMESTAMP, available_at_epoch = ?, claimed_by = ?, claim_key = ?, claim_expires_at_epoch = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = ? AND claim_key = ?",
        }
    };
    let mut query = rullst::db::sqlx::query(sql);
    if success_status == "delivered" {
        query = query
            .bind(success_status)
            .bind("")
            .bind("")
            .bind(0_i64)
            .bind(error)
            .bind(id)
            .bind("processing")
            .bind(claim_key);
    } else {
        query = query
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
            .bind(claim_key);
    }
    query
        .execute(rullst::db::Orm::pool()?)
        .await
        .map(|result| result.rows_affected() == 1)
        .map_err(|error| OutboxError::Database(error.into()))
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}
"##;

#[cfg(test)]
mod tests {
    use super::OUTBOX_SERVICE;

    #[test]
    fn outbox_template_uses_claim_compare_and_dead_letter_state() {
        assert!(OUTBOX_SERVICE.contains("claim_expires_at_epoch <= $10"));
        assert!(OUTBOX_SERVICE.contains("AND claim_key = $8"));
        assert!(OUTBOX_SERVICE.contains("available_at_epoch = $4"));
        assert!(OUTBOX_SERVICE.contains("dead_letter"));
        assert!(OUTBOX_SERVICE.contains("rows_affected() == 1"));
        assert!(!OUTBOX_SERVICE.contains("format!(\"UPDATE"));
    }
}
