// Authenticated, versioned and idempotent score-event templates.

mod activity;
mod model;
mod policy;
mod replay;
mod test_template;

use activity::ACTIVITY_SCORE_SERVICE;
use model::SCORE_EVENT_MODEL;
use policy::SCORE_POLICY_SERVICE;
use replay::SCORE_REPLAY_SERVICE;
use test_template::SCORE_TESTS;

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/score_event.rs", SCORE_EVENT_MODEL.to_string()),
        ("src/services/score_service.rs", SCORE_SERVICE.to_string()),
        (
            "src/services/score_service/activity.rs",
            ACTIVITY_SCORE_SERVICE.to_string(),
        ),
        (
            "src/services/score_service/policy.rs",
            SCORE_POLICY_SERVICE.to_string(),
        ),
        (
            "src/services/score_service/replay.rs",
            SCORE_REPLAY_SERVICE.to_string(),
        ),
        (
            "src/services/score_service/tests.rs",
            SCORE_TESTS.to_string(),
        ),
    ]
}

const SCORE_SERVICE: &str = r##"use crate::models::leaderboard_entry::LeaderboardEntry;
use crate::services::school_service;
use rullst::{Cache, TenantCache};
use rullst_security::{RbacGuard, UserContext};
use std::sync::OnceLock;

mod activity;
mod policy;
mod replay;
pub use activity::record_activity_result;
use policy::lock_activity_policy;
use replay::persist_activity_attempt;

pub const SCORE_EVENT_SCHEMA_VERSION: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScoreSubmission {
    pub idempotency_key: String,
    pub schema_version: i32,
    pub origin: String,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub activity_id: i32,
    pub attempt_key: String,
    pub points: i32,
    pub max_score: i32,
    pub ruleset_version: String,
    pub season_key: String,
    pub evidence_sha256: String,
    pub policy_binding: String,
    pub activity_kind: String,
    pub state_json: String,
    pub submission_key: String,
    pub started_at_epoch: i64,
    pub finished_at_epoch: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScoreReceipt {
    pub idempotency_key: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum ScoreError {
    Forbidden,
    InvalidIdentity,
    InvalidField(&'static str),
    UnsupportedSchemaVersion(i32),
    Cache(String),
    Database(rullst_orm::Error),
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("score submission denied"),
            Self::InvalidIdentity => formatter.write_str("authenticated actor is not a numeric LMS user"),
            Self::InvalidField(field) => write!(formatter, "invalid score field: {field}"),
            Self::UnsupportedSchemaVersion(version) => write!(formatter, "unsupported score schema version: {version}"),
            Self::Cache(error) => write!(formatter, "score cache error: {error}"),
            Self::Database(error) => write!(formatter, "score database error: {error}"),
        }
    }
}

impl std::error::Error for ScoreError {}

impl From<rullst_orm::Error> for ScoreError {
    fn from(error: rullst_orm::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedScore {
    actor_user_id: i32,
    submission: ScoreSubmission,
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn leaderboard_cache(context: &UserContext) -> Result<TenantCache, ScoreError> {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let tenant_id = context.tenant_id().ok_or(ScoreError::Forbidden)?;
    let tenant_context = rullst::security::TenantContext::try_new(tenant_id)
        .map_err(|_| ScoreError::Forbidden)?;
    Ok(TenantCache::from_context(
        CACHE.get_or_init(Cache::memory).clone(),
        &tenant_context,
    ))
}

fn leaderboard_cache_key(course_id: i32, season_key: &str) -> String {
    format!("leaderboard:course:{course_id}:season:{season_key}")
}

fn valid_cached_leaderboard(
    entries: &[LeaderboardEntry],
    course_id: i32,
    season_key: &str,
) -> bool {
    entries.len() <= 100
        && entries.iter().all(|entry| {
            entry.id > 0
                && entry.user_id > 0
                && entry.course_id == course_id
                && entry.season_key == season_key
                && entry.score >= 0
        })
}

pub async fn invalidate_leaderboard_cache(
    context: &UserContext,
    course_id: i32,
    season_key: &str,
) -> Result<(), ScoreError> {
    if course_id <= 0 || !valid_key(season_key, 64) {
        return Err(ScoreError::InvalidField("leaderboard cache key"));
    }
    leaderboard_cache(context)?
        .forget(&leaderboard_cache_key(course_id, season_key))
        .await
        .map_err(|error| ScoreError::Cache(error.to_string()))
}

#[cfg(test)]
pub async fn leaderboard_cache_contains(
    context: &UserContext,
    course_id: i32,
    season_key: &str,
) -> Result<bool, ScoreError> {
    if course_id <= 0 || !valid_key(season_key, 64) {
        return Err(ScoreError::InvalidField("leaderboard cache key"));
    }
    leaderboard_cache(context)?
        .has(&leaderboard_cache_key(course_id, season_key))
        .await
        .map_err(|error| ScoreError::Cache(error.to_string()))
}

fn validate(
    context: &UserContext,
    submission: ScoreSubmission,
) -> Result<ValidatedScore, ScoreError> {
    if submission.schema_version != SCORE_EVENT_SCHEMA_VERSION {
        return Err(ScoreError::UnsupportedSchemaVersion(submission.schema_version));
    }
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| ScoreError::InvalidIdentity)?;
    RbacGuard::authorize_owner_or_role(
        context,
        &submission.subject_user_id.to_string(),
        "admin",
    )
    .map_err(|_| ScoreError::Forbidden)?;

    if !matches!(submission.origin.as_str(), "quiz" | "activity" | "game") {
        return Err(ScoreError::InvalidField("origin"));
    }
    if !matches!(submission.activity_kind.as_str(), "quiz" | "exercise" | "game") {
        return Err(ScoreError::InvalidField("activity_kind"));
    }
    for (name, value, maximum) in [
        ("idempotency_key", submission.idempotency_key.as_str(), 192),
        ("attempt_key", submission.attempt_key.as_str(), 128),
        ("ruleset_version", submission.ruleset_version.as_str(), 64),
        ("season_key", submission.season_key.as_str(), 64),
        ("submission_key", submission.submission_key.as_str(), 128),
    ] {
        if !valid_key(value, maximum) {
            return Err(ScoreError::InvalidField(name));
        }
    }
    if submission.evidence_sha256.len() != 64
        || !submission
            .evidence_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ScoreError::InvalidField("evidence_sha256"));
    }
    if submission.subject_user_id <= 0
        || submission.course_id <= 0
        || submission.activity_id <= 0
        || submission.points < 0
        || submission.max_score <= 0
        || submission.points > submission.max_score
        || submission.max_score > 1_000_000
    {
        return Err(ScoreError::InvalidField("score bounds"));
    }
    if submission.started_at_epoch <= 0
        || submission.finished_at_epoch < submission.started_at_epoch
        || submission.state_json.is_empty()
        || submission.state_json.len() > 64 * 1024
        || !matches!(
            serde_json::from_str::<serde_json::Value>(&submission.state_json),
            Ok(serde_json::Value::Object(_))
        )
    {
        return Err(ScoreError::InvalidField("activity attempt"));
    }
    if submission.policy_binding.is_empty()
        || submission.policy_binding.len() > 8_192
        || !matches!(
            serde_json::from_str::<serde_json::Value>(&submission.policy_binding),
            Ok(serde_json::Value::Object(_))
        )
    {
        return Err(ScoreError::InvalidField("policy binding"));
    }

    Ok(ValidatedScore {
        actor_user_id,
        submission,
    })
}

pub async fn leaderboard(
    context: &UserContext,
    course_id: i32,
    season_key: &str,
    limit: u32,
) -> Result<Vec<LeaderboardEntry>, ScoreError> {
    if course_id <= 0 || !valid_key(season_key, 64) || !(1..=100).contains(&limit) {
        return Err(ScoreError::InvalidField("leaderboard query"));
    }
    let limit = usize::try_from(limit)
        .map_err(|_| ScoreError::InvalidField("leaderboard query"))?;
    school_service::authorize_course(context, course_id).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ScoreError::Database(error),
            _ => ScoreError::Forbidden,
        })?;
    let cache = leaderboard_cache(context)?;
    let cache_key = leaderboard_cache_key(course_id, season_key);
    if let Some(payload) = cache
        .get(&cache_key)
        .await
        .map_err(|error| ScoreError::Cache(error.to_string()))?
    {
        if let Ok(mut entries) = serde_json::from_str::<Vec<LeaderboardEntry>>(&payload) {
            if valid_cached_leaderboard(&entries, course_id, season_key) {
                entries.truncate(limit);
                return Ok(entries);
            }
        }
    }
    let query = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT * FROM leaderboard_entries WHERE course_id = $1 AND season_key = $2 ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT 100",
        _ => "SELECT * FROM leaderboard_entries WHERE course_id = ? AND season_key = ? ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT 100",
    };
    let mut entries = rullst::db::sqlx::query_as::<_, LeaderboardEntry>(query)
        .bind(course_id)
        .bind(season_key)
        .fetch_all(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    if let Ok(payload) = serde_json::to_string(&entries) {
        let _ = cache.put(&cache_key, &payload, Some(30)).await;
    }
    entries.truncate(limit);
    Ok(entries)
}

async fn record_score(
    context: &UserContext,
    submission: ScoreSubmission,
) -> Result<ScoreReceipt, ScoreError> {
    let validated = validate(context, submission)?;
    let value = &validated.submission;
    let driver = rullst::db::Orm::driver()?;
    let lesson_sql = match driver {
        "postgres" => "SELECT lesson_id FROM activities WHERE id = $1",
        _ => "SELECT lesson_id FROM activities WHERE id = ?",
    };
    let lesson_id = rullst::db::sqlx::query_scalar::<_, i32>(lesson_sql)
        .bind(value.activity_id).fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| ScoreError::Database(error.into()))?
        .ok_or(ScoreError::Forbidden)?;
    let scoped_course = school_service::authorize_lesson(context, lesson_id).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ScoreError::Database(error),
            _ => ScoreError::Forbidden,
        })?;
    if scoped_course != value.course_id { return Err(ScoreError::Forbidden); }
    let school_id = school_service::authorize_course_enrollment_at(
        context,
        value.subject_user_id,
        value.course_id,
        unix_now()?,
    ).await.map_err(|error| match error {
        school_service::SchoolError::Database(error) => ScoreError::Database(error),
        _ => ScoreError::Forbidden,
    })?;
    let pool = rullst::db::Orm::pool()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    lock_activity_policy(&mut transaction, driver, value).await?;
    if !persist_activity_attempt(
        &mut transaction,
        driver,
        validated.actor_user_id,
        value,
    )
    .await?
    {
        transaction
            .commit()
            .await
            .map_err(|error| ScoreError::Database(error.into()))?;
        return Ok(ScoreReceipt {
            idempotency_key: value.idempotency_key.clone(),
            applied: false,
        });
    }

    let event_sql = match driver {
        "postgres" => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, evidence_sha256, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, $11, $12, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, evidence_sha256, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, evidence_sha256, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    let insertion = rullst::db::sqlx::query(event_sql)
        .bind(&value.idempotency_key)
        .bind(value.schema_version)
        .bind(&value.origin)
        .bind(validated.actor_user_id)
        .bind(value.subject_user_id)
        .bind(value.course_id)
        .bind(value.activity_id)
        .bind(&value.attempt_key)
        .bind(value.points)
        .bind(value.max_score)
        .bind(&value.ruleset_version)
        .bind(&value.evidence_sha256)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;

    if insertion.rows_affected() != 1 {
        return Err(ScoreError::InvalidField("score event conflict"));
    }
    let leaderboard_sql = match driver {
        "postgres" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + EXCLUDED.score, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE score = score + VALUES(score), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + excluded.score, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(leaderboard_sql)
        .bind(value.subject_user_id)
        .bind(value.course_id)
        .bind(&value.season_key)
        .bind(value.points)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;

    let outbox_key = format!("score:{}", value.idempotency_key);
    let payload = serde_json::json!({
        "schema_version": SCORE_EVENT_SCHEMA_VERSION,
        "idempotency_key": value.idempotency_key,
        "origin": value.origin,
        "actor_user_id": validated.actor_user_id,
        "subject_user_id": value.subject_user_id,
        "course_id": value.course_id,
        "activity_id": value.activity_id,
        "attempt_key": value.attempt_key,
        "points": value.points,
        "max_score": value.max_score,
        "ruleset_version": value.ruleset_version,
        "season_key": value.season_key,
        "evidence_sha256": value.evidence_sha256,
    })
    .to_string();
    let outbox_sql = match driver {
        "postgres" => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO academy_outbox (school_id, event_key, event_kind, subject_user_id, payload_json, status, attempts, claimed_by, claim_key, last_error, available_at, available_at_epoch, claim_expires_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, 0, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(outbox_sql)
        .bind(school_id)
        .bind(outbox_key)
        .bind("score_recorded")
        .bind(value.subject_user_id)
        .bind(payload)
        .bind("pending")
        .bind(0_i32)
        .bind("")
        .bind("")
        .bind("")
        .execute(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;

    transaction
        .commit()
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    let _ = invalidate_leaderboard_cache(context, value.course_id, &value.season_key).await;
    Ok(ScoreReceipt {
        idempotency_key: value.idempotency_key.clone(),
        applied: true,
    })
}

fn unix_now() -> Result<i64, ScoreError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ScoreError::InvalidField("clock"))?;
    i64::try_from(elapsed.as_secs()).map_err(|_| ScoreError::InvalidField("clock"))
}

#[cfg(test)]
mod tests;
"##;

#[cfg(test)]
mod tests {
    use super::{
        ACTIVITY_SCORE_SERVICE, SCORE_POLICY_SERVICE, SCORE_REPLAY_SERVICE, SCORE_SERVICE,
    };

    #[test]
    fn score_template_binds_every_dynamic_value() {
        assert!(!SCORE_SERVICE.contains("format!(\"INSERT"));
        assert!(SCORE_SERVICE.contains("ON CONFLICT DO NOTHING"));
        assert!(SCORE_SERVICE.contains("INSERT IGNORE INTO score_events"));
        assert!(SCORE_SERVICE.contains("execute(&mut *transaction)"));
        assert!(SCORE_SERVICE.contains("INSERT INTO academy_outbox"));
        assert!(SCORE_SERVICE.contains("ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT"));
        assert!(ACTIVITY_SCORE_SERVICE.contains("ValidatedActivityResult"));
        assert!(ACTIVITY_SCORE_SERVICE.contains("persisted activity policy"));
        assert!(ACTIVITY_SCORE_SERVICE.contains("activity:{}:{}:{}"));
        assert!(SCORE_POLICY_SERVICE.contains("FOR UPDATE"));
        assert!(SCORE_POLICY_SERVICE.contains("evidence_sha256"));
        assert!(SCORE_REPLAY_SERVICE.contains("conflicting activity replay"));
    }
}
