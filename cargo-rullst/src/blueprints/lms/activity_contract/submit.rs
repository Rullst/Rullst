//! Persisted single-choice submission service and authenticated HTTP boundary.

pub(super) const ACTIVITY_ATTEMPT_MODEL: &str = r##"use rullst::db::{FromRow, Orm};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "activity_attempts")]
pub struct ActivityAttempt {
    pub id: i32,
    pub attempt_key: String,
    pub activity_id: i32,
    pub actor_user_id: i32,
    pub subject_user_id: i32,
    pub activity_kind: String,
    pub ruleset_version: String,
    pub state_json: String,
    pub submission_key: String,
    pub points: i32,
    pub max_score: i32,
    pub started_at_epoch: i64,
    pub finished_at_epoch: i64,
    pub evidence_sha256: String,
    pub created_at: String,
    pub updated_at: String,
}

impl NexusModel for ActivityAttempt {
    fn nexus_table() -> &'static str { "activity_attempts" }
    fn nexus_label() -> &'static str { "Activity Attempts" }
    fn nexus_icon() -> &'static str { "🧩" }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
            FieldMeta { name: "attempt_key", label: "Attempt", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "activity_id", label: "Activity", kind: FieldKind::ForeignKey { table: "activities", label_col: "title" }, hidden: false, readonly: true },
            FieldMeta { name: "actor_user_id", label: "Actor", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "subject_user_id", label: "Learner", kind: FieldKind::ForeignKey { table: "users", label_col: "email" }, hidden: false, readonly: true },
            FieldMeta { name: "activity_kind", label: "Kind", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "ruleset_version", label: "Ruleset", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "state_json", label: "Bounded State", kind: FieldKind::Json, hidden: true, readonly: true },
            FieldMeta { name: "submission_key", label: "Submission", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "points", label: "Points", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "max_score", label: "Maximum Score", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "started_at_epoch", label: "Started Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "finished_at_epoch", label: "Finished Epoch", kind: FieldKind::Number, hidden: false, readonly: true },
            FieldMeta { name: "evidence_sha256", label: "Evidence SHA-256", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "created_at", label: "Created At", kind: FieldKind::Text, hidden: false, readonly: true },
            FieldMeta { name: "updated_at", label: "Updated At", kind: FieldKind::Text, hidden: false, readonly: true },
        ]
    }
}
"##;

pub(super) const ACTIVITY_SUBMISSION_SERVICE: &str = r##"use super::{
    ACTIVITY_SCHEMA_VERSION, ActivityAttempt, ActivityContractError, ActivityKind,
    SingleChoiceEvaluator, SingleChoiceSubmission, evaluate_activity, valid_key,
};
use crate::services::learning_service::{LearningError, authorize_lesson};
use crate::services::score_service::{ScoreError, ScoreReceipt, record_activity_result};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleChoiceRequest {
    pub attempt_key: String,
    pub selected_option_id: i32,
}

#[derive(Debug)]
pub enum ActivitySubmissionError {
    Access(LearningError),
    NotFound,
    InvalidInput(&'static str),
    InvalidPolicy,
    Contract(ActivityContractError),
    Score(ScoreError),
    Clock,
    Database(rullst_orm::Error),
}

impl std::fmt::Display for ActivitySubmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(error) => write!(formatter, "activity access error: {error}"),
            Self::NotFound => formatter.write_str("activity not found"),
            Self::InvalidInput(field) => write!(formatter, "invalid activity input: {field}"),
            Self::InvalidPolicy => formatter.write_str("invalid persisted activity policy"),
            Self::Contract(error) => write!(formatter, "activity contract error: {error}"),
            Self::Score(error) => write!(formatter, "activity score error: {error}"),
            Self::Clock => formatter.write_str("system clock is before the Unix epoch"),
            Self::Database(error) => write!(formatter, "activity database error: {error}"),
        }
    }
}

impl std::error::Error for ActivitySubmissionError {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SingleChoicePolicyV1 {
    schema_version: i32,
    mode: String,
    correct_option_id: i32,
}

pub async fn submit_single_choice(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: SingleChoiceRequest,
) -> Result<ScoreReceipt, ActivitySubmissionError> {
    submit_single_choice_at(
        context,
        subject_user_id,
        activity_id,
        request,
        unix_now()?,
    )
    .await
}

pub async fn submit_single_choice_at(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: SingleChoiceRequest,
    server_epoch_seconds: i64,
) -> Result<ScoreReceipt, ActivitySubmissionError> {
    if subject_user_id <= 0
        || activity_id <= 0
        || request.selected_option_id <= 0
        || !valid_key(&request.attempt_key, 128)
        || server_epoch_seconds <= 0
    {
        return Err(ActivitySubmissionError::InvalidInput("request"));
    }
    let pool = rullst::db::Orm::pool().map_err(ActivitySubmissionError::Database)?;
    let driver = rullst::db::Orm::driver().map_err(ActivitySubmissionError::Database)?;
    let lesson_sql = match driver {
        "postgres" => "SELECT lesson_id FROM activities WHERE id = $1",
        _ => "SELECT lesson_id FROM activities WHERE id = ?",
    };
    let lesson_id = rullst::db::sqlx::query_scalar::<_, i32>(lesson_sql)
        .bind(activity_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| ActivitySubmissionError::Database(error.into()))?
        .ok_or(ActivitySubmissionError::NotFound)?;
    authorize_lesson(subject_user_id, context, lesson_id)
        .await
        .map_err(ActivitySubmissionError::Access)?;

    let policy_sql = match driver {
        "postgres" => "SELECT activity_kind, max_score, ruleset_version, evidence_sha256, config_json FROM activities WHERE id = $1",
        _ => "SELECT activity_kind, max_score, ruleset_version, evidence_sha256, config_json FROM activities WHERE id = ?",
    };
    let policy = rullst::db::sqlx::query_as::<_, (String, i32, String, String, String)>(
        policy_sql,
    )
    .bind(activity_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ActivitySubmissionError::Database(error.into()))?
    .ok_or(ActivitySubmissionError::NotFound)?;
    if policy.0 != "exercise" || policy.4.len() > 8_192 {
        return Err(ActivitySubmissionError::InvalidPolicy);
    }
    let config: SingleChoicePolicyV1 =
        serde_json::from_str(&policy.4).map_err(|_| ActivitySubmissionError::InvalidPolicy)?;
    if config.schema_version != 1
        || config.mode != "single_choice"
        || config.correct_option_id <= 0
    {
        return Err(ActivitySubmissionError::InvalidPolicy);
    }
    let evaluator = SingleChoiceEvaluator::new(
        config.correct_option_id,
        policy.1,
        policy.3,
        policy.4,
    )
    .map_err(ActivitySubmissionError::Contract)?;
    let validated = evaluate_activity(
        context,
        ActivityAttempt {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            attempt_key: request.attempt_key,
            activity_id,
            subject_user_id,
            kind: ActivityKind::Exercise,
            ruleset_version: policy.2,
            started_at_epoch_seconds: server_epoch_seconds,
            state_json: "{\"schema_version\":1,\"mode\":\"single_choice\"}".to_string(),
        },
        &SingleChoiceSubmission {
            selected_option_id: request.selected_option_id,
        },
        server_epoch_seconds,
        &evaluator,
    )
    .map_err(ActivitySubmissionError::Contract)?;
    record_activity_result(context, validated)
        .await
        .map_err(ActivitySubmissionError::Score)
}

fn unix_now() -> Result<i64, ActivitySubmissionError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ActivitySubmissionError::Clock)?;
    i64::try_from(elapsed.as_secs()).map_err(|_| ActivitySubmissionError::Clock)
}
"##;

pub(super) const ACTIVITY_CONTROLLER: &str = r##"use crate::services::activity_contract::{
    ActivitySubmissionError, SingleChoiceRequest, submit_single_choice,
};
use crate::services::learning_service::LearningError;
use crate::services::score_service::ScoreError;
use rullst::server::{Extension, IntoResponse, Json, Path, Response, StatusCode};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingleChoicePayload {
    pub attempt_key: String,
    pub selected_option_id: i32,
}

fn error_response(error: ActivitySubmissionError) -> Response {
    match error {
        ActivitySubmissionError::Access(LearningError::Database(error))
        | ActivitySubmissionError::Database(error)
        | ActivitySubmissionError::Score(ScoreError::Database(error)) => {
            eprintln!("Activity submission failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        ActivitySubmissionError::Score(ScoreError::Cache(error)) => {
            eprintln!("Activity cache update failed: {error}");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
        ActivitySubmissionError::NotFound => StatusCode::NOT_FOUND.into_response(),
        ActivitySubmissionError::Access(_)
        | ActivitySubmissionError::Score(ScoreError::Forbidden | ScoreError::InvalidIdentity) => {
            StatusCode::FORBIDDEN.into_response()
        }
        ActivitySubmissionError::InvalidPolicy
        | ActivitySubmissionError::Score(
            ScoreError::InvalidField(_) | ScoreError::UnsupportedSchemaVersion(_),
        ) => StatusCode::CONFLICT.into_response(),
        ActivitySubmissionError::InvalidInput(_)
        | ActivitySubmissionError::Contract(_) => {
            StatusCode::UNPROCESSABLE_ENTITY.into_response()
        }
        ActivitySubmissionError::Clock => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn submit(
    Path(activity_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<SingleChoicePayload>,
) -> Response {
    match submit_single_choice(
        &context,
        user_id,
        activity_id,
        SingleChoiceRequest {
            attempt_key: payload.attempt_key,
            selected_option_id: payload.selected_option_id,
        },
    )
    .await
    {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{ACTIVITY_CONTROLLER, ACTIVITY_SUBMISSION_SERVICE};

    #[test]
    fn submission_boundary_derives_identity_policy_points_and_clock_server_side() {
        assert!(ACTIVITY_CONTROLLER.contains("Extension(user_id)"));
        assert!(ACTIVITY_CONTROLLER.contains("Path(activity_id)"));
        assert!(!ACTIVITY_CONTROLLER.contains("pub points"));
        assert!(!ACTIVITY_CONTROLLER.contains("pub subject_user_id"));
        assert!(!ACTIVITY_CONTROLLER.contains("pub ruleset_version"));
        assert!(ACTIVITY_SUBMISSION_SERVICE.contains("SingleChoicePolicyV1"));
        assert!(ACTIVITY_SUBMISSION_SERVICE.contains("unix_now()?"));
        assert!(ACTIVITY_SUBMISSION_SERVICE.contains("authorize_lesson"));
    }
}
