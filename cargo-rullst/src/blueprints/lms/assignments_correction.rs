// Append-only administrative assignment-grade correction service.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/assignment_grade_correction_service.rs",
        ASSIGNMENT_GRADE_CORRECTION_SERVICE.to_string(),
    )]
}

const ASSIGNMENT_GRADE_CORRECTION_SERVICE: &str = r##"use crate::services::assignment_grading_service::RubricScoreInput;
use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentGradeCorrectionInput {
    pub correction_key: String,
    pub assignment_grade_id: i32,
    pub reason: String,
    pub scores: Vec<RubricScoreInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssignmentGradeCorrectionReceipt {
    pub correction_id: i32,
    pub correction_key: String,
    pub assignment_grade_id: i32,
    pub previous_points: i32,
    pub corrected_points: i32,
    pub max_points: i32,
    pub applied: bool,
}

#[derive(Debug)]
pub enum AssignmentGradeCorrectionError {
    Forbidden,
    NotFound,
    InvalidField(&'static str),
    InvalidRubric,
    InvalidScore,
    IdempotencyConflict,
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for AssignmentGradeCorrectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("assignment grade correction access denied"),
            Self::NotFound => formatter.write_str("assignment grade not found"),
            Self::InvalidField(field) => write!(formatter, "invalid assignment grade correction field: {field}"),
            Self::InvalidRubric => formatter.write_str("assignment grade correction rubric is inconsistent"),
            Self::InvalidScore => formatter.write_str("assignment grade correction score is invalid"),
            Self::IdempotencyConflict => formatter.write_str("assignment grade correction idempotency conflict"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "assignment grade correction database error: {error}"),
        }
    }
}

impl std::error::Error for AssignmentGradeCorrectionError {}

impl From<rullst_orm::Error> for AssignmentGradeCorrectionError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

pub async fn correct_assignment_grade(
    context: &UserContext,
    input: AssignmentGradeCorrectionInput,
) -> Result<AssignmentGradeCorrectionReceipt, AssignmentGradeCorrectionError> {
    correct_assignment_grade_at(context, input, unix_now()?).await
}

pub async fn correct_assignment_grade_at(
    context: &UserContext,
    input: AssignmentGradeCorrectionInput,
    now_epoch: i64,
) -> Result<AssignmentGradeCorrectionReceipt, AssignmentGradeCorrectionError> {
    let actor_user_id = authorize_admin(context)?;
    let reason = normalize_reason(&input.reason)?;
    if input.assignment_grade_id <= 0 || now_epoch <= 0 || !valid_key(&input.correction_key, 96)
        || input.scores.is_empty() || input.scores.len() > 100
    { return Err(AssignmentGradeCorrectionError::InvalidField("request")); }
    authorize_grade_scope(context, input.assignment_grade_id).await?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssignmentGradeCorrectionError::Database(error),
            _ => AssignmentGradeCorrectionError::Forbidden,
        })?;
    let mut scores = input.scores;
    for score in &mut scores {
        if score.criterion_id <= 0 || score.points_awarded < 0 {
            return Err(AssignmentGradeCorrectionError::InvalidScore);
        }
        score.feedback = normalize_feedback(&score.feedback)?.to_string();
    }
    scores.sort_by_key(|score| score.criterion_id);
    let ids = scores.iter().map(|score| score.criterion_id).collect::<BTreeSet<_>>();
    if ids.len() != scores.len() { return Err(AssignmentGradeCorrectionError::InvalidRubric); }
    let request_json = serde_json::to_string(&serde_json::json!({
        "reason": reason, "scores": &scores,
    })).map_err(|_| AssignmentGradeCorrectionError::InvalidField("request"))?;
    let scores_json = serde_json::to_string(&scores)
        .map_err(|_| AssignmentGradeCorrectionError::InvalidField("scores"))?;

    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    let grade_sql = match driver {
        "postgres" => "SELECT assignment_id, subject_user_id, points_awarded, max_points, ruleset_version FROM assignment_grades WHERE id = $1 FOR UPDATE",
        "mysql" => "SELECT assignment_id, subject_user_id, points_awarded, max_points, ruleset_version FROM assignment_grades WHERE id = ? FOR UPDATE",
        _ => "SELECT assignment_id, subject_user_id, points_awarded, max_points, ruleset_version FROM assignment_grades WHERE id = ?",
    };
    let grade = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, String)>(grade_sql)
        .bind(input.assignment_grade_id).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?
        .ok_or(AssignmentGradeCorrectionError::NotFound)?;

    let replay_sql = match driver {
        "postgres" => "SELECT id, assignment_grade_id, actor_user_id, previous_points, corrected_points, max_points, request_json FROM assignment_grade_corrections WHERE correction_key = $1",
        _ => "SELECT id, assignment_grade_id, actor_user_id, previous_points, corrected_points, max_points, request_json FROM assignment_grade_corrections WHERE correction_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, i32, i32, String)>(replay_sql)
        .bind(&input.correction_key).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?
    {
        if existing.1 != input.assignment_grade_id || existing.2 != actor_user_id
            || existing.6 != request_json
        { return Err(AssignmentGradeCorrectionError::IdempotencyConflict); }
        transaction.commit().await
            .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
        return Ok(receipt(existing, input.correction_key, false));
    }

    let criteria_sql = match driver {
        "postgres" => "SELECT id, max_points FROM rubric_criteria WHERE assignment_id = $1 ORDER BY id ASC",
        _ => "SELECT id, max_points FROM rubric_criteria WHERE assignment_id = ? ORDER BY id ASC",
    };
    let criteria = rullst::db::sqlx::query_as::<_, (i32, i32)>(criteria_sql)
        .bind(grade.0).fetch_all(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    if criteria.is_empty() || criteria.len() != scores.len() || criteria.len() > 100 {
        return Err(AssignmentGradeCorrectionError::InvalidRubric);
    }
    let mut corrected_points = 0_i32;
    let mut max_points = 0_i32;
    for (criterion, score) in criteria.iter().zip(&scores) {
        if criterion.0 != score.criterion_id || criterion.1 <= 0
            || score.points_awarded > criterion.1
        { return Err(AssignmentGradeCorrectionError::InvalidScore); }
        corrected_points = corrected_points.checked_add(score.points_awarded)
            .ok_or(AssignmentGradeCorrectionError::InvalidScore)?;
        max_points = max_points.checked_add(criterion.1)
            .ok_or(AssignmentGradeCorrectionError::InvalidScore)?;
    }
    if max_points != grade.3 { return Err(AssignmentGradeCorrectionError::InvalidRubric); }
    let previous_sql = match driver {
        "postgres" => "SELECT corrected_points FROM assignment_grade_corrections WHERE assignment_grade_id = $1 ORDER BY id DESC LIMIT 1",
        _ => "SELECT corrected_points FROM assignment_grade_corrections WHERE assignment_grade_id = ? ORDER BY id DESC LIMIT 1",
    };
    let previous_points = rullst::db::sqlx::query_scalar::<_, i32>(previous_sql)
        .bind(input.assignment_grade_id).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?
        .unwrap_or(grade.2);
    if corrected_points == previous_points { return Err(AssignmentGradeCorrectionError::InvalidScore); }

    let insert_sql = match driver {
        "postgres" => "INSERT INTO assignment_grade_corrections (correction_key, assignment_grade_id, actor_user_id, previous_points, corrected_points, max_points, reason, scores_json, request_json, corrected_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO assignment_grade_corrections (correction_key, assignment_grade_id, actor_user_id, previous_points, corrected_points, max_points, reason, scores_json, request_json, corrected_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(insert_sql).bind(&input.correction_key).bind(input.assignment_grade_id)
        .bind(actor_user_id).bind(previous_points).bind(corrected_points).bind(max_points)
        .bind(reason).bind(&scores_json).bind(&request_json).bind(now_epoch)
        .execute(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    let id_sql = match driver {
        "postgres" => "SELECT id FROM assignment_grade_corrections WHERE correction_key = $1",
        _ => "SELECT id FROM assignment_grade_corrections WHERE correction_key = ?",
    };
    let correction_id = rullst::db::sqlx::query_scalar::<_, i32>(id_sql)
        .bind(&input.correction_key).fetch_one(&mut *transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    let payload = serde_json::json!({
        "schema_version":1,"actor_user_id":actor_user_id,"subject_user_id":grade.1,
        "assignment_id":grade.0,"assignment_grade_id":input.assignment_grade_id,
        "correction_id":correction_id,"correction_key":input.correction_key,
        "ruleset_version":grade.4,"previous_points":previous_points,
        "corrected_points":corrected_points,"max_points":max_points,
    }).to_string();
    insert_outbox(&mut transaction, driver,
        &format!("assignment-grade-corrected:{}", input.correction_key), school_id, grade.1, &payload).await?;
    transaction.commit().await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    Ok(AssignmentGradeCorrectionReceipt { correction_id, correction_key: input.correction_key,
        assignment_grade_id: input.assignment_grade_id, previous_points, corrected_points,
        max_points, applied: true })
}

pub async fn effective_grade(assignment_grade_id: i32) -> Result<(i32, i32), AssignmentGradeCorrectionError> {
    if assignment_grade_id <= 0 { return Err(AssignmentGradeCorrectionError::InvalidField("grade")); }
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT points_awarded, max_points, (SELECT corrected_points FROM assignment_grade_corrections WHERE assignment_grade_id = $1 ORDER BY id DESC LIMIT 1) FROM assignment_grades WHERE id = $2",
        _ => "SELECT points_awarded, max_points, (SELECT corrected_points FROM assignment_grade_corrections WHERE assignment_grade_id = ? ORDER BY id DESC LIMIT 1) FROM assignment_grades WHERE id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, i32, Option<i32>)>(sql)
        .bind(assignment_grade_id).bind(assignment_grade_id)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?
        .ok_or(AssignmentGradeCorrectionError::NotFound)?;
    Ok((row.2.unwrap_or(row.0), row.1))
}

async fn insert_outbox(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str, event_key: &str, school_id: i32, subject_user_id: i32, payload: &str,
) -> Result<(), AssignmentGradeCorrectionError> {
    let sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(sql).bind(school_id).bind(event_key).bind("assignment_grade_corrected")
        .bind(subject_user_id).bind(payload).bind("pending").bind("").bind("").bind("")
        .execute(&mut **transaction).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?;
    Ok(())
}

fn receipt(
    row: (i32, i32, i32, i32, i32, i32, String), correction_key: String, applied: bool,
) -> AssignmentGradeCorrectionReceipt {
    AssignmentGradeCorrectionReceipt { correction_id: row.0, correction_key,
        assignment_grade_id: row.1, previous_points: row.3, corrected_points: row.4,
        max_points: row.5, applied }
}

fn authorize_admin(context: &UserContext) -> Result<i32, AssignmentGradeCorrectionError> {
    RbacGuard::authorize(context, "admin").map_err(|_| AssignmentGradeCorrectionError::Forbidden)?;
    context.user_id.parse::<i32>().ok().filter(|id| *id > 0)
        .ok_or(AssignmentGradeCorrectionError::Forbidden)
}

async fn authorize_grade_scope(
    context: &UserContext,
    assignment_grade_id: i32,
) -> Result<(), AssignmentGradeCorrectionError> {
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT a.lesson_id FROM assignment_grades g INNER JOIN assignments a ON a.id = g.assignment_id WHERE g.id = $1",
        _ => "SELECT a.lesson_id FROM assignment_grades g INNER JOIN assignments a ON a.id = g.assignment_id WHERE g.id = ?",
    };
    let lesson_id = rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(assignment_grade_id)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| AssignmentGradeCorrectionError::Database(error.into()))?
        .ok_or(AssignmentGradeCorrectionError::NotFound)?;
    school_service::authorize_lesson(context, lesson_id).await
        .map(|_| ())
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssignmentGradeCorrectionError::Database(error),
            _ => AssignmentGradeCorrectionError::Forbidden,
        })
}

fn normalize_reason(value: &str) -> Result<&str, AssignmentGradeCorrectionError> {
    let value = value.trim();
    if !(8..=512).contains(&value.len()) || value.chars().any(char::is_control) {
        return Err(AssignmentGradeCorrectionError::InvalidField("reason"));
    }
    Ok(value)
}

fn normalize_feedback(value: &str) -> Result<&str, AssignmentGradeCorrectionError> {
    let value = value.trim();
    if value.len() > 2_000 || value.contains('\0') {
        return Err(AssignmentGradeCorrectionError::InvalidField("feedback"));
    }
    Ok(value)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn unix_now() -> Result<i64, AssignmentGradeCorrectionError> {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AssignmentGradeCorrectionError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AssignmentGradeCorrectionError::Clock)
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSIGNMENT_GRADE_CORRECTION_SERVICE;

    #[test]
    fn grade_correction_is_admin_only_append_only_and_effective() {
        assert!(ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("authorize(context, \"admin\")"));
        assert!(ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("previous_points"));
        assert!(ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("assignment_grade_corrected"));
        assert!(ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("effective_grade"));
        assert!(ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("FOR UPDATE"));
        assert!(!ASSIGNMENT_GRADE_CORRECTION_SERVICE.contains("format!(\"SELECT"));
    }
}
