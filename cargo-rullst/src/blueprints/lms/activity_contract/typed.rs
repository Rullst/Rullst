//! Bounded server-authoritative typed-answer activity templates.

pub(super) const TYPED_ACTIVITY_SERVICE: &str = r##"use super::{
    ACTIVITY_SCHEMA_VERSION, ActivityAttempt, ActivityContractError, ActivityEvaluator,
    ActivityKind, ActivitySubmissionError, AuthoritativeActivityOutcome,
    ValidatedActivityResult, evaluate_activity, valid_key, valid_policy_binding, valid_sha256,
};
use crate::services::learning_service::authorize_lesson;
use crate::services::score_service::{ScoreReceipt, record_activity_result};
use rullst_security::{UserContext, sha256_hex};
use serde::Deserialize;
use std::collections::HashSet;

const MAX_ACCEPTED_ANSWERS: usize = 16;
const MAX_TYPED_ANSWER_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedAnswerRequest {
    pub attempt_key: String,
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedAnswerEvaluator {
    accepted_answers: HashSet<String>,
    case_sensitive: bool,
    max_score: i32,
    evidence_sha256: String,
    policy_binding: String,
}

impl TypedAnswerEvaluator {
    pub fn new(
        accepted_answers: Vec<String>,
        case_sensitive: bool,
        max_score: i32,
        evidence_sha256: impl Into<String>,
        policy_binding: impl Into<String>,
    ) -> Result<Self, ActivityContractError> {
        if !(1..=MAX_ACCEPTED_ANSWERS).contains(&accepted_answers.len()) {
            return Err(ActivityContractError::InvalidField("typed answer rules"));
        }
        let mut normalized = HashSet::with_capacity(accepted_answers.len());
        for answer in accepted_answers {
            let answer = normalize(&answer, case_sensitive)?;
            if !normalized.insert(answer) {
                return Err(ActivityContractError::InvalidField("typed answer rules"));
            }
        }
        let evaluator = Self {
            accepted_answers: normalized,
            case_sensitive,
            max_score,
            evidence_sha256: evidence_sha256.into(),
            policy_binding: policy_binding.into(),
        };
        if !(1..=1_000_000).contains(&evaluator.max_score)
            || !valid_sha256(&evaluator.evidence_sha256)
            || !valid_policy_binding(&evaluator.policy_binding)
        {
            return Err(ActivityContractError::InvalidField("typed answer rules"));
        }
        Ok(evaluator)
    }
}

impl ActivityEvaluator for TypedAnswerEvaluator {
    type Submission = str;

    fn kind(&self) -> ActivityKind {
        ActivityKind::Exercise
    }

    fn evaluate(
        &self,
        _attempt: &ActivityAttempt,
        submission: &Self::Submission,
    ) -> Result<AuthoritativeActivityOutcome, ActivityContractError> {
        let normalized = normalize(submission, self.case_sensitive)?;
        let mut digest_input = Vec::with_capacity(
            self.policy_binding
                .len()
                .saturating_add(normalized.len())
                .saturating_add(1),
        );
        digest_input.extend_from_slice(self.policy_binding.as_bytes());
        digest_input.push(0_u8);
        digest_input.extend_from_slice(normalized.as_bytes());
        let submission_key = format!("text:{}", sha256_hex(digest_input));
        if !valid_key(&submission_key, 128) {
            return Err(ActivityContractError::InvalidField("typed answer"));
        }
        Ok(AuthoritativeActivityOutcome {
            points: if self.accepted_answers.contains(&normalized) {
                self.max_score
            } else {
                0
            },
            max_score: self.max_score,
            evidence_sha256: self.evidence_sha256.clone(),
            submission_key,
            policy_binding: self.policy_binding.clone(),
        })
    }
}

fn normalize(value: &str, case_sensitive: bool) -> Result<String, ActivityContractError> {
    if value.is_empty()
        || value.len() > MAX_TYPED_ANSWER_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(ActivityContractError::InvalidField("typed answer"));
    }
    let trimmed = value.trim();
    let normalized = if case_sensitive {
        trimmed.to_string()
    } else {
        trimmed.to_lowercase()
    };
    if normalized.is_empty() || normalized.len() > MAX_TYPED_ANSWER_BYTES {
        return Err(ActivityContractError::InvalidField("typed answer"));
    }
    Ok(normalized)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypedAnswerPolicyV1 {
    schema_version: i32,
    mode: String,
    case_sensitive: bool,
    accepted_answers: Vec<String>,
}

pub async fn submit_typed_answer(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: TypedAnswerRequest,
) -> Result<ScoreReceipt, ActivitySubmissionError> {
    submit_typed_answer_at(
        context,
        subject_user_id,
        activity_id,
        request,
        super::submit::unix_now()?,
    )
    .await
}

pub async fn submit_typed_answer_at(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: TypedAnswerRequest,
    server_epoch_seconds: i64,
) -> Result<ScoreReceipt, ActivitySubmissionError> {
    if subject_user_id <= 0
        || activity_id <= 0
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
    let config: TypedAnswerPolicyV1 =
        serde_json::from_str(&policy.4).map_err(|_| ActivitySubmissionError::InvalidPolicy)?;
    if config.schema_version != 1 || config.mode != "typed_answer" {
        return Err(ActivitySubmissionError::InvalidPolicy);
    }
    let evaluator = TypedAnswerEvaluator::new(
        config.accepted_answers,
        config.case_sensitive,
        policy.1,
        policy.3,
        policy.4,
    )
    .map_err(ActivitySubmissionError::Contract)?;
    let validated: ValidatedActivityResult = evaluate_activity(
        context,
        ActivityAttempt {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            attempt_key: request.attempt_key,
            activity_id,
            subject_user_id,
            kind: ActivityKind::Exercise,
            ruleset_version: policy.2,
            started_at_epoch_seconds: server_epoch_seconds,
            state_json: "{\"schema_version\":1,\"mode\":\"typed_answer\"}".to_string(),
        },
        request.answer.as_str(),
        server_epoch_seconds,
        &evaluator,
    )
    .map_err(ActivitySubmissionError::Contract)?;
    record_activity_result(context, validated)
        .await
        .map_err(ActivitySubmissionError::Score)
}
"##;

pub(super) const TYPED_ACTIVITY_CONTROLLER: &str = r##"use super::activity_controller::error_response;
use crate::services::activity_contract::{TypedAnswerRequest, submit_typed_answer};
use rullst::server::{Extension, IntoResponse, Json, Path, Response};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedAnswerPayload {
    pub attempt_key: String,
    pub answer: String,
}

pub async fn submit(
    Path(activity_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<TypedAnswerPayload>,
) -> Response {
    let request = TypedAnswerRequest {
        attempt_key: payload.attempt_key,
        answer: payload.answer,
    };
    match submit_typed_answer(&context, user_id, activity_id, request).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{TYPED_ACTIVITY_CONTROLLER, TYPED_ACTIVITY_SERVICE};

    #[test]
    fn typed_boundary_hashes_normalized_input_and_never_accepts_points_or_answers() {
        assert!(TYPED_ACTIVITY_SERVICE.contains("MAX_TYPED_ANSWER_BYTES: usize = 512"));
        assert!(TYPED_ACTIVITY_SERVICE.contains("sha256_hex"));
        assert!(TYPED_ACTIVITY_SERVICE.contains("policy_binding"));
        assert!(TYPED_ACTIVITY_CONTROLLER.contains("Extension(user_id)"));
        assert!(!TYPED_ACTIVITY_CONTROLLER.contains("pub points"));
        assert!(!TYPED_ACTIVITY_CONTROLLER.contains("accepted_answers"));
    }
}
