//! Trusted activity-result and leaderboard service for the detached profile.

pub(super) const GAMIFICATION_SERVICE: &str = r##"use crate::models::enrollment::Enrollment;
use crate::models::leaderboard_entry::LeaderboardEntry;
use crate::services::learning_service::{LearningError, authorize_lesson};
use rullst_security::{RbacGuard, UserContext};

pub const SCORE_EVENT_SCHEMA_VERSION: i32 = 1;

/// Server-side activity adapters construct this value after validating game evidence.
/// It intentionally does not implement `Deserialize`, so generated HTTP handlers cannot
/// accept authoritative points directly from an untrusted request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedActivityResult {
    pub idempotency_key: String,
    pub schema_version: i32,
    pub origin: String,
    pub subject_user_id: i32,
    pub activity_id: i32,
    pub attempt_key: String,
    pub points: i32,
    pub ruleset_version: String,
    pub season_key: String,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreReceipt {
    pub idempotency_key: String,
    pub applied: bool,
}

#[derive(Debug)]
pub enum GamificationError {
    Access(LearningError),
    NotFound,
    NotPublished,
    AttemptLimit,
    IdempotencyConflict,
    UnsupportedSchemaVersion(i32),
    InvalidIdentity,
    InvalidField(&'static str),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for GamificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "activity access error: {error}"),
            Self::NotFound => formatter.write_str("activity not found"),
            Self::NotPublished => formatter.write_str("activity is not published"),
            Self::AttemptLimit => formatter.write_str("activity attempt limit reached"),
            Self::IdempotencyConflict => formatter.write_str("score idempotency conflict"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported score schema version: {version}")
            }
            Self::InvalidIdentity => formatter.write_str("authenticated learner identity is invalid"),
            Self::InvalidField(field) => write!(formatter, "invalid score field: {field}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "gamification database error: {error}"),
        }
    }
}

impl std::error::Error for GamificationError {}

impl From<LearningError> for GamificationError {
    fn from(error: LearningError) -> Self { Self::Access(error) }
}

impl From<rullst_orm::Error> for GamificationError {
    fn from(error: rullst_orm::Error) -> Self { Self::Database(error) }
}

#[derive(Debug)]
struct ActivityRules {
    lesson_id: i32,
    course_id: i32,
    activity_kind: String,
    max_score: i32,
    max_attempts: i32,
    ruleset_version: String,
    status: String,
}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn unix_now() -> Result<i64, GamificationError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| GamificationError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| GamificationError::Clock)
}

async fn activity_rules(activity_id: i32) -> Result<ActivityRules, GamificationError> {
    if activity_id <= 0 { return Err(GamificationError::InvalidField("activity")); }
    let sql = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT activities.lesson_id, lessons.course_id, activities.activity_kind, activities.max_score, activities.max_attempts, activities.ruleset_version, activities.status FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = $1",
        _ => "SELECT activities.lesson_id, lessons.course_id, activities.activity_kind, activities.max_score, activities.max_attempts, activities.ruleset_version, activities.status FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = ?",
    };
    let row = rullst::db::sqlx::query_as::<_, (i32, i32, String, i32, i32, String, String)>(sql)
        .bind(activity_id).fetch_optional(rullst::db::Orm::pool()?).await
        .map_err(|error| GamificationError::Database(error.into()))?
        .ok_or(GamificationError::NotFound)?;
    Ok(ActivityRules {
        lesson_id: row.0,
        course_id: row.1,
        activity_kind: row.2,
        max_score: row.3,
        max_attempts: row.4,
        ruleset_version: row.5,
        status: row.6,
    })
}

fn validate_result(result: &TrustedActivityResult) -> Result<(), GamificationError> {
    if result.schema_version != SCORE_EVENT_SCHEMA_VERSION {
        return Err(GamificationError::UnsupportedSchemaVersion(result.schema_version));
    }
    if result.subject_user_id <= 0 || result.activity_id <= 0 || result.points < 0 {
        return Err(GamificationError::InvalidField("score bounds"));
    }
    if !matches!(result.origin.as_str(), "exercise" | "game") {
        return Err(GamificationError::InvalidField("origin"));
    }
    for (field, value, maximum) in [
        ("idempotency_key", result.idempotency_key.as_str(), 128),
        ("attempt_key", result.attempt_key.as_str(), 128),
        ("ruleset_version", result.ruleset_version.as_str(), 64),
        ("season_key", result.season_key.as_str(), 64),
    ] {
        if !valid_key(value, maximum) { return Err(GamificationError::InvalidField(field)); }
    }
    if !valid_digest(&result.evidence_digest) {
        return Err(GamificationError::InvalidField("evidence digest"));
    }
    Ok(())
}

pub async fn record_activity_result(
    context: &UserContext,
    result: TrustedActivityResult,
) -> Result<ScoreReceipt, GamificationError> {
    record_activity_result_at(context, result, unix_now()?).await
}

pub async fn record_activity_result_at(
    context: &UserContext,
    result: TrustedActivityResult,
    occurred_at_epoch: i64,
) -> Result<ScoreReceipt, GamificationError> {
    validate_result(&result)?;
    if occurred_at_epoch <= 0 { return Err(GamificationError::InvalidField("clock")); }
    let actor_user_id = context.user_id.parse::<i32>()
        .map_err(|_| GamificationError::InvalidIdentity)?;
    let rules = activity_rules(result.activity_id).await?;
    authorize_lesson(context, result.subject_user_id, rules.lesson_id).await?;
    if rules.status != "published" { return Err(GamificationError::NotPublished); }
    if rules.activity_kind != result.origin || rules.ruleset_version != result.ruleset_version {
        return Err(GamificationError::InvalidField("activity rules"));
    }
    if !(1..=1_000_000).contains(&rules.max_score)
        || !(1..=1_000).contains(&rules.max_attempts)
        || result.points > rules.max_score
    {
        return Err(GamificationError::InvalidField("score bounds"));
    }

    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool.begin().await
        .map_err(|error| GamificationError::Database(error.into()))?;
    let replay_sql = match driver {
        "postgres" => "SELECT schema_version, origin, subject_user_id, course_id, activity_id, attempt_key, points, max_score, ruleset_version, season_key, evidence_digest FROM score_events WHERE idempotency_key = $1",
        _ => "SELECT schema_version, origin, subject_user_id, course_id, activity_id, attempt_key, points, max_score, ruleset_version, season_key, evidence_digest FROM score_events WHERE idempotency_key = ?",
    };
    if let Some(event) = rullst::db::sqlx::query_as::<_, (i32, String, i32, i32, i32, String, i32, i32, String, String, String)>(replay_sql)
        .bind(&result.idempotency_key).fetch_optional(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?
    {
        if event.0 != result.schema_version || event.1 != result.origin
            || event.2 != result.subject_user_id || event.3 != rules.course_id
            || event.4 != result.activity_id || event.5 != result.attempt_key
            || event.6 != result.points || event.7 != rules.max_score
            || event.8 != result.ruleset_version || event.9 != result.season_key
            || event.10 != result.evidence_digest
        {
            return Err(GamificationError::IdempotencyConflict);
        }
        transaction.commit().await
            .map_err(|error| GamificationError::Database(error.into()))?;
        return Ok(ScoreReceipt { idempotency_key: result.idempotency_key, applied: false });
    }

    let lock_sql = match driver {
        "postgres" => "SELECT id FROM activities WHERE id = $1 FOR UPDATE",
        "mysql" => "SELECT id FROM activities WHERE id = ? FOR UPDATE",
        _ => "SELECT id FROM activities WHERE id = ?",
    };
    rullst::db::sqlx::query_scalar::<_, i32>(lock_sql).bind(result.activity_id)
        .fetch_one(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?;
    let attempt_sql = match driver {
        "postgres" => "SELECT idempotency_key FROM score_events WHERE origin = $1 AND subject_user_id = $2 AND activity_id = $3 AND attempt_key = $4 AND ruleset_version = $5",
        _ => "SELECT idempotency_key FROM score_events WHERE origin = ? AND subject_user_id = ? AND activity_id = ? AND attempt_key = ? AND ruleset_version = ?",
    };
    if rullst::db::sqlx::query_scalar::<_, String>(attempt_sql)
        .bind(&result.origin).bind(result.subject_user_id).bind(result.activity_id)
        .bind(&result.attempt_key).bind(&result.ruleset_version)
        .fetch_optional(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?.is_some()
    {
        return Err(GamificationError::IdempotencyConflict);
    }
    let count_sql = match driver {
        "postgres" => "SELECT COUNT(*) FROM score_events WHERE subject_user_id = $1 AND activity_id = $2 AND ruleset_version = $3",
        _ => "SELECT COUNT(*) FROM score_events WHERE subject_user_id = ? AND activity_id = ? AND ruleset_version = ?",
    };
    let attempts = rullst::db::sqlx::query_scalar::<_, i64>(count_sql)
        .bind(result.subject_user_id).bind(result.activity_id).bind(&result.ruleset_version)
        .fetch_one(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?;
    if attempts >= i64::from(rules.max_attempts) {
        return Err(GamificationError::AttemptLimit);
    }

    let event_sql = match driver {
        "postgres" => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, ruleset_version, season_key, evidence_digest, occurred_at_epoch, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, ruleset_version, season_key, evidence_digest, occurred_at_epoch, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    };
    rullst::db::sqlx::query(event_sql).bind(&result.idempotency_key)
        .bind(result.schema_version).bind(&result.origin).bind(actor_user_id)
        .bind(result.subject_user_id).bind(rules.course_id).bind(result.activity_id)
        .bind(&result.attempt_key).bind(result.points).bind(rules.max_score)
        .bind(&result.ruleset_version).bind(&result.season_key).bind(&result.evidence_digest)
        .bind(occurred_at_epoch).execute(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?;
    let leaderboard_sql = match driver {
        "postgres" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + EXCLUDED.score, updated_at = CURRENT_TIMESTAMP",
        "mysql" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE score = score + VALUES(score), updated_at = CURRENT_TIMESTAMP",
        _ => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = leaderboard_entries.score + excluded.score, updated_at = CURRENT_TIMESTAMP",
    };
    rullst::db::sqlx::query(leaderboard_sql).bind(result.subject_user_id)
        .bind(rules.course_id).bind(&result.season_key).bind(result.points)
        .execute(&mut *transaction).await
        .map_err(|error| GamificationError::Database(error.into()))?;
    transaction.commit().await
        .map_err(|error| GamificationError::Database(error.into()))?;
    Ok(ScoreReceipt { idempotency_key: result.idempotency_key, applied: true })
}

pub async fn leaderboard(
    context: &UserContext,
    user_id: i32,
    course_id: i32,
    season_key: &str,
    limit: u32,
) -> Result<Vec<LeaderboardEntry>, GamificationError> {
    RbacGuard::authorize_owner_or_role(context, &user_id.to_string(), "admin")
        .map_err(|_| GamificationError::Access(LearningError::Forbidden))?;
    if user_id <= 0 || course_id <= 0 || !valid_key(season_key, 64)
        || !(1..=100).contains(&limit)
    {
        return Err(GamificationError::InvalidField("leaderboard query"));
    }
    if Enrollment::active_for(user_id, course_id).await?.is_none() {
        return Err(GamificationError::Access(LearningError::Forbidden));
    }
    let query = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT * FROM leaderboard_entries WHERE course_id = $1 AND season_key = $2 ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT $3",
        _ => "SELECT * FROM leaderboard_entries WHERE course_id = ? AND season_key = ? ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT ?",
    };
    rullst::db::sqlx::query_as::<_, LeaderboardEntry>(query)
        .bind(course_id).bind(season_key).bind(i64::from(limit))
        .fetch_all(rullst::db::Orm::pool()?).await
        .map_err(|error| GamificationError::Database(error.into()))
}
"##;
