//! Transaction-locked persisted policy check for every generated score event.

pub(super) const SCORE_POLICY_SERVICE: &str = r##"use super::{ScoreError, ScoreSubmission};

pub(super) async fn lock_activity_policy(
    transaction: &mut rullst::db::sqlx::Transaction<'_, rullst::db::sqlx::Any>,
    driver: &str,
    value: &ScoreSubmission,
) -> Result<(), ScoreError> {
    let policy_sql = match driver {
        "postgres" => "SELECT lessons.course_id, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.season_key, activities.evidence_sha256 FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = $1 FOR UPDATE",
        "mysql" => "SELECT lessons.course_id, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.season_key, activities.evidence_sha256 FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = ? FOR UPDATE",
        _ => "SELECT lessons.course_id, activities.activity_kind, activities.max_score, activities.ruleset_version, activities.season_key, activities.evidence_sha256 FROM activities INNER JOIN lessons ON lessons.id = activities.lesson_id WHERE activities.id = ?",
    };
    let policy = rullst::db::sqlx::query_as::<_, (i32, String, i32, String, String, String)>(
        policy_sql,
    )
    .bind(value.activity_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| ScoreError::Database(error.into()))?
    .ok_or(ScoreError::Forbidden)?;
    let origin = match policy.1.as_str() {
        "quiz" => "quiz",
        "exercise" => "activity",
        "game" => "game",
        _ => return Err(ScoreError::InvalidField("persisted activity kind")),
    };
    if policy.0 != value.course_id
        || origin != value.origin
        || policy.2 != value.max_score
        || policy.3 != value.ruleset_version
        || policy.4 != value.season_key
        || policy.5 != value.evidence_sha256
    {
        return Err(ScoreError::InvalidField("persisted activity policy"));
    }
    Ok(())
}
"##;
