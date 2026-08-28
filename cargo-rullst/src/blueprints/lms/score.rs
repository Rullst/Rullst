// Authenticated, versioned and idempotent score-event templates.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        ("src/models/score_event.rs", SCORE_EVENT_MODEL.to_string()),
        ("src/services/score_service.rs", SCORE_SERVICE.to_string()),
    ]
}

const SCORE_EVENT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "score_events")]
pub struct ScoreEvent {
    pub id: i32,
    pub idempotency_key: String,
    pub schema_version: i32,
    pub origin: String,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub activity_id: i32,
    pub attempt_key: String,
    pub points: i32,
    pub max_score: i32,
    pub occurred_at: String,
    pub ruleset_version: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ScoreEvent {
    fn nexus_table() -> &'static str { "score_events" }
    fn nexus_label() -> &'static str { "Score Events" }
    fn nexus_icon() -> &'static str { "🧾" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "idempotency_key", label: "Idempotency Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "schema_version", label: "Schema Version", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "origin", label: "Origin", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "activity_id", label: "Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "attempt_key", label: "Attempt", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "points", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_score", label: "Maximum Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "occurred_at", label: "Occurred At", kind: FieldKind::DateTime, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const SCORE_SERVICE: &str = r##"use crate::models::leaderboard_entry::LeaderboardEntry;
use crate::services::school_service;
use rullst_security::{RbacGuard, UserContext};

pub const SCORE_EVENT_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreSubmission {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    Database(rullst_orm::Error),
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("score submission denied"),
            Self::InvalidIdentity => formatter.write_str("authenticated actor is not a numeric LMS user"),
            Self::InvalidField(field) => write!(formatter, "invalid score field: {field}"),
            Self::UnsupportedSchemaVersion(version) => write!(formatter, "unsupported score schema version: {version}"),
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
    for (name, value, maximum) in [
        ("idempotency_key", submission.idempotency_key.as_str(), 128),
        ("attempt_key", submission.attempt_key.as_str(), 128),
        ("ruleset_version", submission.ruleset_version.as_str(), 64),
        ("season_key", submission.season_key.as_str(), 64),
    ] {
        if !valid_key(value, maximum) {
            return Err(ScoreError::InvalidField(name));
        }
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
    school_service::authorize_course(context, course_id).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ScoreError::Database(error),
            _ => ScoreError::Forbidden,
        })?;
    let query = match rullst::db::Orm::driver()? {
        "postgres" => "SELECT * FROM leaderboard_entries WHERE course_id = $1 AND season_key = $2 ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT $3",
        _ => "SELECT * FROM leaderboard_entries WHERE course_id = ? AND season_key = ? ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT ?",
    };
    rullst::db::sqlx::query_as::<_, LeaderboardEntry>(query)
        .bind(course_id)
        .bind(season_key)
        .bind(i64::from(limit))
        .fetch_all(rullst::db::Orm::pool()?)
        .await
        .map_err(|error| ScoreError::Database(error.into()))
}

pub async fn record_score(
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

    let event_sql = match driver {
        "postgres" => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, $11, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO score_events (idempotency_key, schema_version, origin, actor_user_id, subject_user_id, course_id, activity_id, attempt_key, points, max_score, occurred_at, ruleset_version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
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
        .execute(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;

    let applied = insertion.rows_affected() == 1;
    if applied {
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
            "schema_version": 1,
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
    }

    transaction
        .commit()
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    Ok(ScoreReceipt {
        idempotency_key: value.idempotency_key.clone(),
        applied,
    })
}

fn unix_now() -> Result<i64, ScoreError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ScoreError::InvalidField("clock"))?;
    i64::try_from(elapsed.as_secs()).map_err(|_| ScoreError::InvalidField("clock"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submission() -> ScoreSubmission {
        ScoreSubmission {
            idempotency_key: "event-1".to_string(),
            schema_version: SCORE_EVENT_SCHEMA_VERSION,
            origin: "activity".to_string(),
            subject_user_id: 7,
            course_id: 2,
            activity_id: 3,
            attempt_key: "attempt-1".to_string(),
            points: 80,
            max_score: 100,
            ruleset_version: "rules-v1".to_string(),
            season_key: "season-2026".to_string(),
        }
    }

    #[test]
    fn actor_comes_from_authenticated_context_and_cross_user_is_denied() {
        let owner = UserContext::new("7", vec!["student".to_string()]);
        let attacker = UserContext::new("8", vec!["student".to_string()]);

        let validated = validate(&owner, submission()).expect("owner score should validate");
        assert_eq!(validated.actor_user_id, 7);
        assert!(matches!(
            validate(&attacker, submission()),
            Err(ScoreError::Forbidden)
        ));
    }

    #[test]
    fn invalid_schema_keys_and_scores_fail_closed() {
        let owner = UserContext::new("7", vec!["student".to_string()]);
        let mut invalid = submission();
        invalid.points = 101;
        assert!(matches!(
            validate(&owner, invalid),
            Err(ScoreError::InvalidField("score bounds"))
        ));

        let mut future = submission();
        future.schema_version = 2;
        assert!(matches!(
            validate(&owner, future),
            Err(ScoreError::UnsupportedSchemaVersion(2))
        ));
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::SCORE_SERVICE;

    #[test]
    fn score_template_binds_every_dynamic_value() {
        assert!(!SCORE_SERVICE.contains("format!(\"INSERT"));
        assert!(SCORE_SERVICE.contains("ON CONFLICT DO NOTHING"));
        assert!(SCORE_SERVICE.contains("INSERT IGNORE INTO score_events"));
        assert!(SCORE_SERVICE.contains("execute(&mut *transaction)"));
        assert!(SCORE_SERVICE.contains("INSERT INTO academy_outbox"));
        assert!(SCORE_SERVICE.contains("ORDER BY score DESC, updated_at ASC, user_id ASC LIMIT"));
    }
}
