// Owner-bound assignment submission service.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/assignment_submission_service.rs",
        ASSIGNMENT_SUBMISSION_SERVICE.to_string(),
    )]
}

const ASSIGNMENT_SUBMISSION_SERVICE: &str = r##"use crate::services::learning_service::{LearningError, authorize_lesson_at};
use crate::services::school_service;
use rullst_security::UserContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentSubmissionInput {
    pub submission_key: String,
    pub assignment_id: i32,
    pub subject_user_id: i32,
    pub content_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssignmentSubmissionReceipt {
    pub submission_id: i32,
    pub submission_key: String,
    pub assignment_id: i32,
    pub subject_user_id: i32,
    pub attempt_number: i32,
    pub ruleset_version: String,
    pub status: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum AssignmentSubmissionError {
    Access(LearningError),
    Forbidden,
    NotFound,
    NotPublished,
    Deadline,
    AttemptLimit,
    InvalidField(&'static str),
    IdempotencyConflict,
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for AssignmentSubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "assignment access error: {error}"),
            Self::Forbidden => formatter.write_str("assignment submission access denied"),
            Self::NotFound => formatter.write_str("assignment not found"),
            Self::NotPublished => formatter.write_str("assignment is not published"),
            Self::Deadline => formatter.write_str("assignment deadline has passed"),
            Self::AttemptLimit => formatter.write_str("assignment attempt limit reached"),
            Self::InvalidField(field) => write!(formatter, "invalid assignment submission field: {field}"),
            Self::IdempotencyConflict => formatter.write_str("assignment submission idempotency conflict"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "assignment submission database error: {error}"),
        }
    }
}

impl std::error::Error for AssignmentSubmissionError {}

impl From<rullst_orm::Error> for AssignmentSubmissionError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

impl From<LearningError> for AssignmentSubmissionError {
    fn from(error: LearningError) -> Self { Self::Access(error) }
}

pub async fn submit_assignment(
    context: &UserContext,
    input: AssignmentSubmissionInput,
) -> Result<AssignmentSubmissionReceipt, AssignmentSubmissionError> {
    submit_assignment_at(context, input, unix_now()?).await
}

pub async fn submit_assignment_at(
    context: &UserContext,
    input: AssignmentSubmissionInput,
    now_epoch: i64,
) -> Result<AssignmentSubmissionReceipt, AssignmentSubmissionError> {
    let actor_user_id = context.user_id.parse::<i32>().ok().filter(|id| *id > 0)
        .ok_or(AssignmentSubmissionError::Forbidden)?;
    let content_text = input.content_text.trim();
    if actor_user_id != input.subject_user_id { return Err(AssignmentSubmissionError::Forbidden); }
    if input.assignment_id <= 0 || now_epoch <= 0
        || !valid_key(&input.submission_key, 96) || content_text.is_empty()
        || content_text.len() > 32_768 || content_text.contains('\0')
    {
        return Err(AssignmentSubmissionError::InvalidField("request"));
    }
    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let assignment_sql = match driver {
        "postgres" => "SELECT lesson_id, ruleset_version, max_attempts, due_at_epoch, status FROM assignments WHERE id = $1",
        _ => "SELECT lesson_id, ruleset_version, max_attempts, due_at_epoch, status FROM assignments WHERE id = ?",
    };
    let assignment = rullst::db::sqlx::query_as::<_, (i32, String, i32, i64, String)>(assignment_sql)
        .bind(input.assignment_id).fetch_optional(pool).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?
        .ok_or(AssignmentSubmissionError::NotFound)?;
    if assignment.4 != "published" { return Err(AssignmentSubmissionError::NotPublished); }
    if !valid_key(&assignment.1, 96) || !(1..=100).contains(&assignment.2) || assignment.3 < 0 {
        return Err(AssignmentSubmissionError::InvalidField("assignment policy"));
    }
    authorize_lesson_at(input.subject_user_id, context, assignment.0, now_epoch).await?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssignmentSubmissionError::Database(error),
            _ => AssignmentSubmissionError::Forbidden,
        })?;

    let mut transaction = pool.begin().await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT id, assignment_id, subject_user_id, attempt_number, content_text, ruleset_version, status FROM assignment_submissions WHERE submission_key = $1",
        _ => "SELECT id, assignment_id, subject_user_id, attempt_number, content_text, ruleset_version, status FROM assignment_submissions WHERE submission_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, String, String, String)>(replay_sql)
        .bind(&input.submission_key).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?
    {
        if existing.1 != input.assignment_id || existing.2 != input.subject_user_id
            || existing.4 != content_text || existing.5 != assignment.1
        {
            return Err(AssignmentSubmissionError::IdempotencyConflict);
        }
        transaction.commit().await
            .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
        return Ok(receipt(existing, input.submission_key, false));
    }
    if assignment.3 > 0 && now_epoch > assignment.3 {
        return Err(AssignmentSubmissionError::Deadline);
    }
    let count_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM assignment_submissions WHERE assignment_id = $1 AND subject_user_id = $2",
        _ => "SELECT COUNT(*) FROM assignment_submissions WHERE assignment_id = ? AND subject_user_id = ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i64>(count_sql)
        .bind(input.assignment_id).bind(input.subject_user_id).fetch_one(&mut *transaction).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    let attempt_number = i32::try_from(attempts).ok().and_then(|value| value.checked_add(1))
        .ok_or(AssignmentSubmissionError::AttemptLimit)?;
    if attempt_number > assignment.2 { return Err(AssignmentSubmissionError::AttemptLimit); }

    let insert_sql = match driver {
        "postgres" => "INSERT INTO assignment_submissions (submission_key, assignment_id, actor_user_id, subject_user_id, attempt_number, content_text, ruleset_version, status, submitted_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO assignment_submissions (submission_key, assignment_id, actor_user_id, subject_user_id, attempt_number, content_text, ruleset_version, status, submitted_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(&input.submission_key).bind(input.assignment_id)
        .bind(actor_user_id).bind(input.subject_user_id).bind(attempt_number).bind(content_text)
        .bind(&assignment.1).bind("submitted").bind(now_epoch).execute(&mut *transaction).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    let id_sql = match driver {
        "postgres" => "SELECT id FROM assignment_submissions WHERE submission_key = $1",
        _ => "SELECT id FROM assignment_submissions WHERE submission_key = ?",
    };
    let submission_id = rullst::db::sqlx::query_scalar::<_, i32>(id_sql)
        .bind(&input.submission_key).fetch_one(&mut *transaction).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    let payload = serde_json::json!({
        "schema_version":1,"actor_user_id":actor_user_id,"subject_user_id":input.subject_user_id,
        "assignment_id":input.assignment_id,"submission_id":submission_id,
        "submission_key":input.submission_key,"ruleset_version":assignment.1,
    }).to_string();
    insert_outbox(&mut transaction, driver, &format!("assignment-submitted:{}", input.submission_key),
        school_id, "assignment_submitted", input.subject_user_id, &payload).await?;
    transaction.commit().await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    Ok(AssignmentSubmissionReceipt {
        submission_id, submission_key: input.submission_key, assignment_id: input.assignment_id,
        subject_user_id: input.subject_user_id, attempt_number, ruleset_version: assignment.1,
        status: "submitted".to_string(), applied: true,
    })
}

async fn insert_outbox(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst_orm::RullstDatabase>,
    driver: &str, event_key: &str, school_id: i32, event_kind: &str, subject_user_id: i32, payload: &str,
) -> Result<(), AssignmentSubmissionError> {
    let sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(sql).bind(school_id).bind(event_key).bind(event_kind).bind(subject_user_id)
        .bind(payload).bind("pending").bind("").bind("").bind("")
        .execute(&mut **transaction).await
        .map_err(|error| AssignmentSubmissionError::Database(error.into()))?;
    Ok(())
}

fn receipt(
    row: (i32, i32, i32, i32, String, String, String), submission_key: String, applied: bool,
) -> AssignmentSubmissionReceipt {
    AssignmentSubmissionReceipt { submission_id: row.0, submission_key, assignment_id: row.1,
        subject_user_id: row.2, attempt_number: row.3, ruleset_version: row.5,
        status: row.6, applied }
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn unix_now() -> Result<i64, AssignmentSubmissionError> {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AssignmentSubmissionError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AssignmentSubmissionError::Clock)
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSIGNMENT_SUBMISSION_SERVICE;

    #[test]
    fn submission_is_owner_bound_timed_and_transactional() {
        assert!(ASSIGNMENT_SUBMISSION_SERVICE.contains("authorize_lesson_at"));
        assert!(ASSIGNMENT_SUBMISSION_SERVICE.contains("AssignmentSubmissionError::Forbidden"));
        assert!(ASSIGNMENT_SUBMISSION_SERVICE.contains("AttemptLimit"));
        assert!(ASSIGNMENT_SUBMISSION_SERVICE.contains("assignment_submitted"));
        assert!(!ASSIGNMENT_SUBMISSION_SERVICE.contains("format!(\"SELECT"));
    }
}
