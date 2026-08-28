// Audited and idempotent leaderboard correction templates.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/models/score_correction.rs",
            SCORE_CORRECTION_MODEL.to_string(),
        ),
        (
            "src/services/score_correction_service.rs",
            SCORE_CORRECTION_SERVICE.to_string(),
        ),
    ]
}

const SCORE_CORRECTION_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "score_corrections")]
pub struct ScoreCorrection {
    pub id: i32,
    pub correction_key: String,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub course_id: i32,
    pub season_key: String,
    pub previous_score: i32,
    pub corrected_score: i32,
    pub reason: String,
    pub ruleset_version: String,
    pub occurred_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ScoreCorrection {
    fn nexus_table() -> &'static str { "score_corrections" }
    fn nexus_label() -> &'static str { "Score Corrections" }
    fn nexus_icon() -> &'static str { "🛡️" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "correction_key", label: "Correction Key", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Administrator", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "course_id", label: "Course", kind: FieldKind::ForeignKey { table: "courses", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "season_key", label: "Season", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "previous_score", label: "Previous Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "corrected_score", label: "Corrected Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "reason", label: "Reason", kind: FieldKind::Textarea, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "occurred_at", label: "Occurred At", kind: FieldKind::DateTime, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

const SCORE_CORRECTION_SERVICE: &str = r##"use crate::services::school_service;
use crate::services::score_service::{ScoreError, ScoreReceipt, invalidate_leaderboard_cache};
use rullst_security::{RbacGuard, UserContext};

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

pub async fn correct_score(
    context: &UserContext,
    correction_key: &str,
    subject_user_id: i32,
    course_id: i32,
    season_key: &str,
    corrected_score: i32,
    reason: &str,
    ruleset_version: &str,
) -> Result<ScoreReceipt, ScoreError> {
    RbacGuard::authorize(context, "admin").map_err(|_| ScoreError::Forbidden)?;
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| ScoreError::InvalidIdentity)?;
    if !valid_key(correction_key, 128)
        || !valid_key(season_key, 64)
        || !valid_key(ruleset_version, 64)
        || subject_user_id <= 0
        || course_id <= 0
        || !(0..=1_000_000).contains(&corrected_score)
        || reason.trim().is_empty()
        || reason.len() > 500
    {
        return Err(ScoreError::InvalidField("score correction"));
    }
    school_service::authorize_course(context, course_id).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ScoreError::Database(error),
            _ => ScoreError::Forbidden,
        })?;
    school_service::authorize_school_membership_at(context, subject_user_id, unix_now()?).await
        .map_err(|error| match error {
            school_service::SchoolError::Database(error) => ScoreError::Database(error),
            _ => ScoreError::Forbidden,
        })?;

    let pool = rullst::db::Orm::pool()?;
    let driver = rullst::db::Orm::driver()?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    let select_sql = match driver {
        "postgres" => "SELECT score FROM leaderboard_entries WHERE user_id = $1 AND course_id = $2 AND season_key = $3",
        _ => "SELECT score FROM leaderboard_entries WHERE user_id = ? AND course_id = ? AND season_key = ?",
    };
    let previous_score = rullst::db::sqlx::query_scalar::<_, i32>(select_sql)
        .bind(subject_user_id)
        .bind(course_id)
        .bind(season_key)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?
        .unwrap_or_default();

    let correction_sql = match driver {
        "postgres" => "INSERT INTO score_corrections (correction_key, actor_user_id, subject_user_id, course_id, season_key, previous_score, corrected_score, reason, ruleset_version, occurred_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
        "mysql" => "INSERT IGNORE INTO score_corrections (correction_key, actor_user_id, subject_user_id, course_id, season_key, previous_score, corrected_score, reason, ruleset_version, occurred_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        _ => "INSERT INTO score_corrections (correction_key, actor_user_id, subject_user_id, course_id, season_key, previous_score, corrected_score, reason, ruleset_version, occurred_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT DO NOTHING",
    };
    let insertion = rullst::db::sqlx::query(correction_sql)
        .bind(correction_key)
        .bind(actor_user_id)
        .bind(subject_user_id)
        .bind(course_id)
        .bind(season_key)
        .bind(previous_score)
        .bind(corrected_score)
        .bind(reason.trim())
        .bind(ruleset_version)
        .execute(&mut *transaction)
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;

    let applied = insertion.rows_affected() == 1;
    if applied {
        let leaderboard_sql = match driver {
            "postgres" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = EXCLUDED.score, updated_at = CURRENT_TIMESTAMP",
            "mysql" => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON DUPLICATE KEY UPDATE score = VALUES(score), updated_at = CURRENT_TIMESTAMP",
            _ => "INSERT INTO leaderboard_entries (user_id, course_id, season_key, score, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (user_id, course_id, season_key) DO UPDATE SET score = excluded.score, updated_at = CURRENT_TIMESTAMP",
        };
        rullst::db::sqlx::query(leaderboard_sql)
            .bind(subject_user_id)
            .bind(course_id)
            .bind(season_key)
            .bind(corrected_score)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ScoreError::Database(error.into()))?;
    }

    transaction
        .commit()
        .await
        .map_err(|error| ScoreError::Database(error.into()))?;
    if applied {
        let _ = invalidate_leaderboard_cache(context, course_id, season_key).await;
    }
    Ok(ScoreReceipt {
        idempotency_key: correction_key.to_string(),
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

    #[tokio::test]
    async fn non_admin_correction_is_denied_before_database_access() {
        let learner = UserContext::new("7", vec!["student".to_string()]);
        let result = correct_score(
            &learner,
            "correction-1",
            7,
            1,
            "season-2026",
            100,
            "reviewed rubric",
            "rules-v1",
        )
        .await;
        assert!(matches!(result, Err(ScoreError::Forbidden)));
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::SCORE_CORRECTION_SERVICE;

    #[test]
    fn correction_template_is_admin_only_idempotent_and_transactional() {
        assert!(SCORE_CORRECTION_SERVICE.contains("RbacGuard::authorize(context, \"admin\")"));
        assert!(SCORE_CORRECTION_SERVICE.contains("ON CONFLICT DO NOTHING"));
        assert!(SCORE_CORRECTION_SERVICE.contains("score = EXCLUDED.score"));
        assert!(SCORE_CORRECTION_SERVICE.contains("commit()"));
    }
}
