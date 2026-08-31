//! Bounded server-authoritative pair-matching activity templates.

pub(super) const MATCHING_ACTIVITY_SERVICE: &str = r##"use super::{
    ACTIVITY_SCHEMA_VERSION, ActivityAttempt, ActivityContractError, ActivityEvaluator,
    ActivityKind, ActivitySubmissionError, AuthoritativeActivityOutcome,
    ValidatedActivityResult, evaluate_activity, valid_key, valid_policy_binding, valid_sha256,
};
use crate::services::learning_service::authorize_lesson;
use crate::services::score_service::{ScoreReceipt, record_activity_result};
use rullst_security::UserContext;
use serde::Deserialize;
use std::collections::HashSet;

const MAX_MATCHING_PAIRS: usize = 8;
const MAX_PAIR_ID: i32 = 999_999;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchingPair {
    pub left_id: i32,
    pub right_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchingRequest {
    pub attempt_key: String,
    pub pairs: Vec<MatchingPair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchingEvaluator {
    rules: Vec<MatchingPair>,
    max_score: i32,
    evidence_sha256: String,
    policy_binding: String,
}

impl MatchingEvaluator {
    pub fn new(
        rules: Vec<MatchingPair>,
        max_score: i32,
        evidence_sha256: impl Into<String>,
        policy_binding: impl Into<String>,
    ) -> Result<Self, ActivityContractError> {
        let evaluator = Self {
            rules,
            max_score,
            evidence_sha256: evidence_sha256.into(),
            policy_binding: policy_binding.into(),
        };
        evaluator.validate_rules()?;
        Ok(evaluator)
    }

    fn validate_rules(&self) -> Result<(), ActivityContractError> {
        if !(2..=MAX_MATCHING_PAIRS).contains(&self.rules.len())
            || !(1..=1_000_000).contains(&self.max_score)
            || !valid_sha256(&self.evidence_sha256)
            || !valid_policy_binding(&self.policy_binding)
        {
            return Err(ActivityContractError::InvalidField("matching rules"));
        }
        let mut left = HashSet::with_capacity(self.rules.len());
        let mut right = HashSet::with_capacity(self.rules.len());
        for pair in &self.rules {
            if !(1..=MAX_PAIR_ID).contains(&pair.left_id)
                || !(1..=MAX_PAIR_ID).contains(&pair.right_id)
                || !left.insert(pair.left_id)
                || !right.insert(pair.right_id)
            {
                return Err(ActivityContractError::InvalidField("matching rules"));
            }
        }
        Ok(())
    }
}

impl ActivityEvaluator for MatchingEvaluator {
    type Submission = [MatchingPair];

    fn kind(&self) -> ActivityKind {
        ActivityKind::Exercise
    }

    fn evaluate(
        &self,
        _attempt: &ActivityAttempt,
        submission: &Self::Submission,
    ) -> Result<AuthoritativeActivityOutcome, ActivityContractError> {
        if submission.len() != self.rules.len() {
            return Err(ActivityContractError::InvalidField("matching submission"));
        }
        let expected_left = self
            .rules
            .iter()
            .map(|pair| pair.left_id)
            .collect::<HashSet<_>>();
        let expected_right = self
            .rules
            .iter()
            .map(|pair| pair.right_id)
            .collect::<HashSet<_>>();
        let mut submitted_left = HashSet::with_capacity(submission.len());
        let mut submitted_right = HashSet::with_capacity(submission.len());
        for pair in submission {
            if !expected_left.contains(&pair.left_id)
                || !expected_right.contains(&pair.right_id)
                || !submitted_left.insert(pair.left_id)
                || !submitted_right.insert(pair.right_id)
            {
                return Err(ActivityContractError::InvalidField("matching submission"));
            }
        }
        let correct = submission
            .iter()
            .filter(|submitted| self.rules.contains(submitted))
            .count();
        let points = i64::from(self.max_score)
            * i64::try_from(correct)
                .map_err(|_| ActivityContractError::InvalidField("matching score"))?
            / i64::try_from(self.rules.len())
                .map_err(|_| ActivityContractError::InvalidField("matching score"))?;
        let points = i32::try_from(points)
            .map_err(|_| ActivityContractError::InvalidField("matching score"))?;
        let mut canonical = submission.to_vec();
        canonical.sort_by_key(|pair| pair.left_id);
        let submission_key = format!(
            "pairs:{}",
            canonical
                .iter()
                .map(|pair| format!("{}-{}", pair.left_id, pair.right_id))
                .collect::<Vec<_>>()
                .join(".")
        );
        if !valid_key(&submission_key, 128) {
            return Err(ActivityContractError::InvalidField("matching submission"));
        }
        Ok(AuthoritativeActivityOutcome {
            points,
            max_score: self.max_score,
            evidence_sha256: self.evidence_sha256.clone(),
            submission_key,
            policy_binding: self.policy_binding.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchingPolicyV1 {
    schema_version: i32,
    mode: String,
    pairs: Vec<MatchingPairPolicyV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchingPairPolicyV1 {
    left_id: i32,
    right_id: i32,
}

pub async fn submit_matching(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: MatchingRequest,
) -> Result<ScoreReceipt, ActivitySubmissionError> {
    submit_matching_at(
        context,
        subject_user_id,
        activity_id,
        request,
        super::submit::unix_now()?,
    )
    .await
}

pub async fn submit_matching_at(
    context: &UserContext,
    subject_user_id: i32,
    activity_id: i32,
    request: MatchingRequest,
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
    let config: MatchingPolicyV1 =
        serde_json::from_str(&policy.4).map_err(|_| ActivitySubmissionError::InvalidPolicy)?;
    if config.schema_version != 1 || config.mode != "matching" {
        return Err(ActivitySubmissionError::InvalidPolicy);
    }
    let rules = config
        .pairs
        .into_iter()
        .map(|pair| MatchingPair {
            left_id: pair.left_id,
            right_id: pair.right_id,
        })
        .collect();
    let evaluator = MatchingEvaluator::new(rules, policy.1, policy.3, policy.4)
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
            state_json: "{\"schema_version\":1,\"mode\":\"matching\"}".to_string(),
        },
        request.pairs.as_slice(),
        server_epoch_seconds,
        &evaluator,
    )
    .map_err(ActivitySubmissionError::Contract)?;
    record_activity_result(context, validated)
        .await
        .map_err(ActivitySubmissionError::Score)
}
"##;

pub(super) const MATCHING_ACTIVITY_CONTROLLER: &str = r##"use super::activity_controller::error_response;
use crate::services::activity_contract::{MatchingPair, MatchingRequest, submit_matching};
use rullst::server::{Extension, IntoResponse, Json, Path, Response};
use rullst_security::UserContext;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchingPairPayload {
    pub left_id: i32,
    pub right_id: i32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchingPayload {
    pub attempt_key: String,
    pub pairs: Vec<MatchingPairPayload>,
}

pub async fn submit(
    Path(activity_id): Path<i32>,
    Extension(user_id): Extension<i32>,
    Extension(context): Extension<UserContext>,
    Json(payload): Json<MatchingPayload>,
) -> Response {
    let request = MatchingRequest {
        attempt_key: payload.attempt_key,
        pairs: payload
            .pairs
            .into_iter()
            .map(|pair| MatchingPair {
                left_id: pair.left_id,
                right_id: pair.right_id,
            })
            .collect(),
    };
    match submit_matching(&context, user_id, activity_id, request).await {
        Ok(receipt) => Json(receipt).into_response(),
        Err(error) => error_response(error),
    }
}
"##;

#[cfg(test)]
mod tests {
    use super::{MATCHING_ACTIVITY_CONTROLLER, MATCHING_ACTIVITY_SERVICE};

    #[test]
    fn matching_boundary_is_bounded_and_never_accepts_points_or_answers() {
        assert!(MATCHING_ACTIVITY_SERVICE.contains("MAX_MATCHING_PAIRS: usize = 8"));
        assert!(MATCHING_ACTIVITY_SERVICE.contains("policy_binding"));
        assert!(MATCHING_ACTIVITY_SERVICE.contains("authorize_lesson"));
        assert!(MATCHING_ACTIVITY_CONTROLLER.contains("Extension(user_id)"));
        assert!(!MATCHING_ACTIVITY_CONTROLLER.contains("pub points"));
        assert!(!MATCHING_ACTIVITY_CONTROLLER.contains("correct"));
    }
}
