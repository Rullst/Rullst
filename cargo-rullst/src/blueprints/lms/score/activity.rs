//! Durable bridge from a validated generic activity to Academy scoring.

pub(super) const ACTIVITY_SCORE_SERVICE: &str = r##"use super::{
    SCORE_EVENT_SCHEMA_VERSION, ScoreError, ScoreReceipt, ScoreSubmission, record_score,
};
use crate::services::activity_contract::{
    ActivityKind, ValidatedActivityResult,
};
use rullst_security::UserContext;

pub async fn record_activity_result(
    context: &UserContext,
    validated: ValidatedActivityResult,
) -> Result<ScoreReceipt, ScoreError> {
    let context_actor = context
        .user_id
        .parse::<i32>()
        .map_err(|_| ScoreError::InvalidIdentity)?;
    if context_actor != validated.actor_user_id() {
        return Err(ScoreError::Forbidden);
    }
    let activity_id = validated.attempt().activity_id;
    let driver = rullst::db::Orm::driver()?;
    let policy_sql = match driver {
        "postgres" => "SELECT lessons.course_id, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.season_key, activities.evidence_sha256 FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = $1",
        _ => "SELECT lessons.course_id, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.season_key, activities.evidence_sha256 FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = ?",
    };
    let policy = rullst::db::sqlx::query_as::<_, (i32, String, i32, String, String, String)>(
        policy_sql,
    )
    .bind(activity_id)
    .fetch_optional(rullst::db::Orm::pool()?)
    .await
    .map_err(|error| ScoreError::Database(error.into()))?
    .ok_or(ScoreError::Forbidden)?;

    let (actor_user_id, attempt, result) = validated.into_parts();
    let (expected_kind, origin) = match policy.1.as_str() {
        "quiz" => (ActivityKind::Quiz, "quiz"),
        "exercise" => (ActivityKind::Exercise, "activity"),
        "game" => (ActivityKind::Game, "game"),
        _ => return Err(ScoreError::InvalidField("persisted activity kind")),
    };
    if attempt.kind != expected_kind
        || attempt.ruleset_version != policy.3
        || result.max_score != policy.2
        || result.evidence_sha256 != policy.5
        || result.attempt_key != attempt.attempt_key
        || result.activity_id != activity_id
        || result.subject_user_id != attempt.subject_user_id
        || result.ruleset_version != attempt.ruleset_version
        || actor_user_id != context_actor
    {
        return Err(ScoreError::InvalidField("persisted activity policy"));
    }

    record_score(
        context,
        ScoreSubmission {
            idempotency_key: attempt.attempt_key.clone(),
            schema_version: SCORE_EVENT_SCHEMA_VERSION,
            origin: origin.to_string(),
            subject_user_id: attempt.subject_user_id,
            course_id: policy.0,
            activity_id,
            attempt_key: attempt.attempt_key,
            points: result.points,
            max_score: result.max_score,
            ruleset_version: attempt.ruleset_version,
            season_key: policy.4,
            evidence_sha256: result.evidence_sha256,
        },
    )
    .await
}
"##;
