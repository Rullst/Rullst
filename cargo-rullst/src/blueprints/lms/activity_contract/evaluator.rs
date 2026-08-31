//! Static-dispatch evaluator emitted beside the generated activity contract.

pub(super) const SINGLE_CHOICE_EVALUATOR: &str = r##"use super::{
    ActivityAttempt, ActivityContractError, ActivityEvaluator, ActivityKind,
    AuthoritativeActivityOutcome, valid_policy_binding, valid_sha256,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleChoiceSubmission {
    pub selected_option_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleChoiceEvaluator {
    correct_option_id: i32,
    max_score: i32,
    evidence_sha256: String,
    policy_binding: String,
}

impl SingleChoiceEvaluator {
    pub fn new(
        correct_option_id: i32,
        max_score: i32,
        evidence_sha256: impl Into<String>,
        policy_binding: impl Into<String>,
    ) -> Result<Self, ActivityContractError> {
        let evaluator = Self {
            correct_option_id,
            max_score,
            evidence_sha256: evidence_sha256.into(),
            policy_binding: policy_binding.into(),
        };
        if evaluator.correct_option_id <= 0
            || !(1..=1_000_000).contains(&evaluator.max_score)
            || !valid_sha256(&evaluator.evidence_sha256)
            || !valid_policy_binding(&evaluator.policy_binding)
        {
            return Err(ActivityContractError::InvalidField(
                "single choice rules",
            ));
        }
        Ok(evaluator)
    }
}

impl ActivityEvaluator for SingleChoiceEvaluator {
    type Submission = SingleChoiceSubmission;

    fn kind(&self) -> ActivityKind {
        ActivityKind::Exercise
    }

    fn evaluate(
        &self,
        _attempt: &ActivityAttempt,
        submission: &Self::Submission,
    ) -> Result<AuthoritativeActivityOutcome, ActivityContractError> {
        if submission.selected_option_id <= 0 {
            return Err(ActivityContractError::InvalidField(
                "selected_option_id",
            ));
        }
        Ok(AuthoritativeActivityOutcome {
            points: if submission.selected_option_id == self.correct_option_id {
                self.max_score
            } else {
                0
            },
            max_score: self.max_score,
            evidence_sha256: self.evidence_sha256.clone(),
            submission_key: format!("option:{}", submission.selected_option_id),
            policy_binding: self.policy_binding.clone(),
        })
    }
}
"##;
