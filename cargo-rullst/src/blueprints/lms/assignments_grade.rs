// Server-bounded human rubric grading service.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![(
        "src/services/assignment_grading_service.rs",
        ASSIGNMENT_GRADING_SERVICE.to_string(),
    )]
}

const ASSIGNMENT_GRADING_SERVICE: &str = r##"use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RubricScoreInput {
    pub criterion_id: i32,
    pub points_awarded: i32,
    pub feedback: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentGradeInput {
    pub grading_key: String,
    pub submission_id: i32,
    pub feedback: String,
    pub scores: Vec<RubricScoreInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AssignmentGradeReceipt {
    pub grade_id: i32,
    pub grading_key: String,
    pub assignment_id: i32,
    pub submission_id: i32,
    pub subject_user_id: i32,
    pub points_awarded: i32,
    pub max_points: i32,
    pub ruleset_version: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum AssignmentGradeError {
    Forbidden,
    NotFound,
    InvalidState,
    InvalidField(&'static str),
    InvalidRubric,
    InvalidScore,
    IdempotencyConflict,
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for AssignmentGradeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("assignment grading access denied"),
            Self::NotFound => formatter.write_str("assignment submission not found"),
            Self::InvalidState => formatter.write_str("assignment submission cannot be graded"),
            Self::InvalidField(field) => write!(formatter, "invalid assignment grade field: {field}"),
            Self::InvalidRubric => formatter.write_str("assignment rubric is incomplete or inconsistent"),
            Self::InvalidScore => formatter.write_str("assignment rubric score exceeds the server policy"),
            Self::IdempotencyConflict => formatter.write_str("assignment grade idempotency conflict"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "assignment grade database error: {error}"),
        }
    }
}

impl std::error::Error for AssignmentGradeError {}

impl From<rullst_orm::Error> for AssignmentGradeError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

pub async fn grade_assignment(
    context: &UserContext,
    input: AssignmentGradeInput,
) -> Result<AssignmentGradeReceipt, AssignmentGradeError> {
    grade_assignment_at(context, input, unix_now()?).await
}

pub async fn grade_assignment_at(
    context: &UserContext,
    input: AssignmentGradeInput,
    now_epoch: i64,
) -> Result<AssignmentGradeReceipt, AssignmentGradeError> {
    let grader_user_id = authorize_grader(context)?;
    let feedback = normalize_text(&input.feedback, 4_000)?;
    if input.submission_id <= 0 || now_epoch <= 0 || !valid_key(&input.grading_key, 96)
        || input.scores.is_empty() || input.scores.len() > 100
    {
        return Err(AssignmentGradeError::InvalidField("request"));
    }
    authorize_submission_scope(context, input.submission_id).await?;
    let school_id = school_service::context_school_id(context).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssignmentGradeError::Database(error),
            _ => AssignmentGradeError::Forbidden,
        })?;
    let mut scores = input.scores;
    for score in &mut scores {
        if score.criterion_id <= 0 || score.points_awarded < 0 {
            return Err(AssignmentGradeError::InvalidScore);
        }
        score.feedback = normalize_text(&score.feedback, 2_000)?.to_string();
    }
    scores.sort_by_key(|score| score.criterion_id);
    let criterion_ids = scores.iter().map(|score| score.criterion_id).collect::<BTreeSet<_>>();
    if criterion_ids.len() != scores.len() { return Err(AssignmentGradeError::InvalidRubric); }
    let request_json = serde_json::to_string(&serde_json::json!({
        "feedback": feedback, "scores": &scores,
    })).map_err(|_| AssignmentGradeError::InvalidField("request"))?;

    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT id, assignment_id, submission_id, grader_user_id, subject_user_id, points_awarded, max_points, ruleset_version, request_json FROM assignment_grades WHERE grading_key = $1",
        _ => "SELECT id, assignment_id, submission_id, grader_user_id, subject_user_id, points_awarded, max_points, ruleset_version, request_json FROM assignment_grades WHERE grading_key = ?",
    };
    if let Some(existing) = rullst::db::sqlx::query_as::<_, (i32, i32, i32, i32, i32, i32, i32, String, String)>(replay_sql)
        .bind(&input.grading_key).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?
    {
        if existing.2 != input.submission_id || existing.3 != grader_user_id
            || existing.8 != request_json
        {
            return Err(AssignmentGradeError::IdempotencyConflict);
        }
        transaction.commit().await
            .map_err(|error| AssignmentGradeError::Database(error.into()))?;
        return Ok(receipt(existing, input.grading_key, false));
    }

    let submission_sql = match driver {
        "postgres" => "SELECT s.assignment_id, s.subject_user_id, s.status, a.ruleset_version FROM assignment_submissions s INNER JOIN assignments a ON a.id = s.assignment_id WHERE s.id = $1",
        _ => "SELECT s.assignment_id, s.subject_user_id, s.status, a.ruleset_version FROM assignment_submissions s INNER JOIN assignments a ON a.id = s.assignment_id WHERE s.id = ?",
    };
    let submission = rullst::db::sqlx::query_as::<_, (i32, i32, String, String)>(submission_sql)
        .bind(input.submission_id).fetch_optional(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?
        .ok_or(AssignmentGradeError::NotFound)?;
    if submission.2 != "submitted" { return Err(AssignmentGradeError::InvalidState); }
    if submission.1 == grader_user_id { return Err(AssignmentGradeError::Forbidden); }
    if !valid_key(&submission.3, 96) { return Err(AssignmentGradeError::InvalidRubric); }

    let criteria_sql = match driver {
        "postgres" => "SELECT id, max_points FROM rubric_criteria WHERE assignment_id = $1 ORDER BY id ASC",
        _ => "SELECT id, max_points FROM rubric_criteria WHERE assignment_id = ? ORDER BY id ASC",
    };
    let criteria = rullst::db::sqlx::query_as::<_, (i32, i32)>(criteria_sql)
        .bind(submission.0).fetch_all(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    if criteria.is_empty() || criteria.len() != scores.len() || criteria.len() > 100 {
        return Err(AssignmentGradeError::InvalidRubric);
    }
    let mut points_awarded = 0_i32;
    let mut max_points = 0_i32;
    for (criterion, score) in criteria.iter().zip(&scores) {
        if criterion.0 != score.criterion_id || criterion.1 <= 0
            || score.points_awarded > criterion.1
        {
            return Err(AssignmentGradeError::InvalidScore);
        }
        points_awarded = points_awarded.checked_add(score.points_awarded)
            .ok_or(AssignmentGradeError::InvalidScore)?;
        max_points = max_points.checked_add(criterion.1)
            .ok_or(AssignmentGradeError::InvalidScore)?;
    }

    let grade_sql = match driver {
        "postgres" => "INSERT INTO assignment_grades (grading_key, assignment_id, submission_id, grader_user_id, subject_user_id, points_awarded, max_points, feedback, ruleset_version, request_json, graded_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO assignment_grades (grading_key, assignment_id, submission_id, grader_user_id, subject_user_id, points_awarded, max_points, feedback, ruleset_version, request_json, graded_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(grade_sql).bind(&input.grading_key).bind(submission.0)
        .bind(input.submission_id).bind(grader_user_id).bind(submission.1)
        .bind(points_awarded).bind(max_points).bind(feedback).bind(&submission.3)
        .bind(&request_json).bind(now_epoch).execute(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    let id_sql = match driver {
        "postgres" => "SELECT id FROM assignment_grades WHERE grading_key = $1",
        _ => "SELECT id FROM assignment_grades WHERE grading_key = ?",
    };
    let grade_id = rullst::db::sqlx::query_scalar::<_, i32>(id_sql)
        .bind(&input.grading_key).fetch_one(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    let score_sql = match driver {
        "postgres" => "INSERT INTO rubric_scores (assignment_grade_id, criterion_id, points_awarded, feedback, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO rubric_scores (assignment_grade_id, criterion_id, points_awarded, feedback, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    for score in &scores {
        rullst::db::sqlx::query(score_sql).bind(grade_id).bind(score.criterion_id)
            .bind(score.points_awarded).bind(&score.feedback).execute(&mut *transaction).await
            .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    }
    let update_sql = match driver {
        "postgres" => "UPDATE assignment_submissions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 AND status = $3",
        _ => "UPDATE assignment_submissions SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = ?",
    };
    if rullst::db::sqlx::query(update_sql).bind("graded").bind(input.submission_id)
        .bind("submitted").execute(&mut *transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?.rows_affected() != 1
    { return Err(AssignmentGradeError::InvalidState); }
    let payload = serde_json::json!({
        "schema_version":1,"actor_user_id":grader_user_id,"subject_user_id":submission.1,
        "assignment_id":submission.0,"submission_id":input.submission_id,"grade_id":grade_id,
        "grading_key":input.grading_key,"ruleset_version":submission.3,
        "points_awarded":points_awarded,"max_points":max_points,
    }).to_string();
    insert_outbox(&mut transaction, driver, &format!("assignment-graded:{}", input.grading_key),
        school_id, submission.1, &payload).await?;
    transaction.commit().await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    Ok(AssignmentGradeReceipt { grade_id, grading_key: input.grading_key,
        assignment_id: submission.0, submission_id: input.submission_id,
        subject_user_id: submission.1, points_awarded, max_points,
        ruleset_version: submission.3, applied: true })
}

async fn insert_outbox(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str, event_key: &str, school_id: i32, subject_user_id: i32, payload: &str,
) -> Result<(), AssignmentGradeError> {
    let sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    rullst::db::sqlx::query(sql).bind(school_id).bind(event_key).bind("assignment_graded")
        .bind(subject_user_id).bind(payload).bind("pending").bind("").bind("").bind("")
        .execute(&mut **transaction).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?;
    Ok(())
}

fn receipt(
    row: (i32, i32, i32, i32, i32, i32, i32, String, String), grading_key: String, applied: bool,
) -> AssignmentGradeReceipt {
    AssignmentGradeReceipt { grade_id: row.0, grading_key, assignment_id: row.1,
        submission_id: row.2, subject_user_id: row.4, points_awarded: row.5,
        max_points: row.6, ruleset_version: row.7, applied }
}

fn authorize_grader(context: &UserContext) -> Result<i32, AssignmentGradeError> {
    if RbacGuard::authorize(context, "evaluator").is_err()
        && RbacGuard::authorize(context, "instructor").is_err()
    { return Err(AssignmentGradeError::Forbidden); }
    context.user_id.parse::<i32>().ok().filter(|id| *id > 0)
        .ok_or(AssignmentGradeError::Forbidden)
}

async fn authorize_submission_scope(
    context: &UserContext,
    submission_id: i32,
) -> Result<(), AssignmentGradeError> {
    let driver = rullst::db::Orm::driver()?;
    let sql = match driver {
        "postgres" => "SELECT a.lesson_id FROM assignment_submissions s INNER JOIN assignments a ON a.id = s.assignment_id WHERE s.id = $1",
        _ => "SELECT a.lesson_id FROM assignment_submissions s INNER JOIN assignments a ON a.id = s.assignment_id WHERE s.id = ?",
    };
    let lesson_id = rullst::db::sqlx::query_scalar::<_, i32>(sql).bind(submission_id)
        .fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| AssignmentGradeError::Database(error.into()))?
        .ok_or(AssignmentGradeError::NotFound)?;
    school_service::authorize_lesson(context, lesson_id).await
        .map(|_| ())
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => AssignmentGradeError::Database(error),
            _ => AssignmentGradeError::Forbidden,
        })
}

fn normalize_text(value: &str, maximum: usize) -> Result<&str, AssignmentGradeError> {
    let value = value.trim();
    if value.len() > maximum || value.contains('\0') {
        return Err(AssignmentGradeError::InvalidField("feedback"));
    }
    Ok(value)
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn unix_now() -> Result<i64, AssignmentGradeError> {
    let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AssignmentGradeError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| AssignmentGradeError::Clock)
}
"##;

#[cfg(test)]
mod tests {
    use super::ASSIGNMENT_GRADING_SERVICE;

    #[test]
    fn grading_uses_exact_persisted_rubric_and_canonical_replay() {
        assert!(ASSIGNMENT_GRADING_SERVICE.contains("rubric_criteria"));
        assert!(ASSIGNMENT_GRADING_SERVICE.contains("score.points_awarded > criterion.1"));
        assert!(ASSIGNMENT_GRADING_SERVICE.contains("request_json"));
        assert!(ASSIGNMENT_GRADING_SERVICE.contains("assignment_graded"));
        assert!(ASSIGNMENT_GRADING_SERVICE.contains("submission.1 == grader_user_id"));
        assert!(!ASSIGNMENT_GRADING_SERVICE.contains("format!(\"SELECT"));
    }
}
