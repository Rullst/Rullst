//! Exact durable replay gate for generated generic activity attempts.

pub(super) const SCORE_REPLAY_SERVICE: &str = r##"use super::{ScoreError, ScoreSubmission};

type StoredAttempt = (i32, i32, i32, String, String, String, String, i32, i32, i64, i64, String);

fn exact(existing: &StoredAttempt, actor_user_id: i32, value: &ScoreSubmission) -> bool {
    existing.0 == value.activity_id
        && existing.1 == actor_user_id
        && existing.2 == value.subject_user_id
        && existing.3 == value.activity_kind
        && existing.4 == value.ruleset_version
        && existing.5 == value.state_json
        && existing.6 == value.submission_key
        && existing.7 == value.points
        && existing.8 == value.max_score
        && existing.11 == value.evidence_sha256
}

async fn load_locked(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str,
    value: &ScoreSubmission,
) -> Result<Option<StoredAttempt>, ScoreError> {
    let sql = match driver {
        "postgres" => "SELECT activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256 FROM activity_attempts WHERE subject_user_id = $1 AND activity_id = $2 AND attempt_key = $3 FOR UPDATE",
        "mysql" => "SELECT activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256 FROM activity_attempts WHERE subject_user_id = ? AND activity_id = ? AND attempt_key = ? FOR UPDATE",
        _ => "SELECT activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256 FROM activity_attempts WHERE subject_user_id = ? AND activity_id = ? AND attempt_key = ?",
    };
    rullst::db::sqlx::query_as::<_, StoredAttempt>(sql)
        .bind(value.subject_user_id)
        .bind(value.activity_id)
        .bind(&value.attempt_key)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))
}

pub(super) async fn persist_activity_attempt(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str,
    actor_user_id: i32,
    value: &ScoreSubmission,
) -> Result<bool, ScoreError> {
    if let Some(existing) = load_locked(transaction, driver, value).await? {
        return if exact(&existing, actor_user_id, value) {
            Ok(false)
        } else {
            Err(ScoreError::InvalidField("conflicting activity replay"))
        };
    }
    let sql = match driver {
        "postgres" => "INSERT INTO activity_attempts (attempt_key, activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO activity_attempts (attempt_key, activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO activity_attempts (attempt_key, activity_id, actor_user_id, subject_user_id, activity_kind, ruleset_version, state_json, submission_key, points, max_score, started_at_epoch, finished_at_epoch, evidence_sha256, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    let inserted = rullst::db::sqlx::query(sql)
        .bind(&value.attempt_key)
        .bind(value.activity_id)
        .bind(actor_user_id)
        .bind(value.subject_user_id)
        .bind(&value.activity_kind)
        .bind(&value.ruleset_version)
        .bind(&value.state_json)
        .bind(&value.submission_key)
        .bind(value.points)
        .bind(value.max_score)
        .bind(value.started_at_epoch)
        .bind(value.finished_at_epoch)
        .bind(&value.evidence_sha256)
        .execute(&mut **transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?
        .rows_affected()
        == 1;
    if inserted {
        return Ok(true);
    }
    let existing = load_locked(transaction, driver, value)
        .await?
        .ok_or(ScoreError::InvalidField("activity attempt conflict"))?;
    if exact(&existing, actor_user_id, value) {
        Ok(false)
    } else {
        Err(ScoreError::InvalidField("conflicting activity replay"))
    }
}
"##;
