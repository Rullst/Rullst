// Versioned server-side activity/attempt/result contract templates.

mod evaluator;
mod matching;
mod submit;
mod test_template;

use evaluator::SINGLE_CHOICE_EVALUATOR;
use matching::{MATCHING_ACTIVITY_CONTROLLER, MATCHING_ACTIVITY_SERVICE};
use submit::{ACTIVITY_ATTEMPT_MODEL, ACTIVITY_CONTROLLER, ACTIVITY_SUBMISSION_SERVICE};
use test_template::ACTIVITY_CONTRACT_TESTS;

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/services/activity_contract.rs",
            ACTIVITY_CONTRACT.to_string(),
        ),
        (
            "src/services/activity_contract/evaluator.rs",
            SINGLE_CHOICE_EVALUATOR.to_string(),
        ),
        (
            "src/services/activity_contract/matching.rs",
            MATCHING_ACTIVITY_SERVICE.to_string(),
        ),
        (
            "src/services/activity_contract/submit.rs",
            ACTIVITY_SUBMISSION_SERVICE.to_string(),
        ),
        (
            "src/models/activity_attempt.rs",
            ACTIVITY_ATTEMPT_MODEL.to_string(),
        ),
        (
            "src/services/activity_contract/tests.rs",
            ACTIVITY_CONTRACT_TESTS.to_string(),
        ),
        (
            "src/controllers/activity_controller.rs",
            ACTIVITY_CONTROLLER.to_string(),
        ),
        (
            "src/controllers/activity_matching_controller.rs",
            MATCHING_ACTIVITY_CONTROLLER.to_string(),
        ),
    ]
}

const ACTIVITY_CONTRACT: &str = r##"use rullst_security::{RbacGuard, UserContext};

mod evaluator;
pub use evaluator::{SingleChoiceEvaluator, SingleChoiceSubmission};
mod matching;
pub use matching::{MatchingEvaluator, MatchingPair, MatchingRequest, submit_matching};
mod submit;
pub use submit::{ActivitySubmissionError, SingleChoiceRequest, submit_single_choice};

#[cfg(test)]
mod tests;

pub const ACTIVITY_SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Quiz,
    Exercise,
    Game,
}

/// Explicit client boundary for one activity.
///
/// `SsrHtmx` is the server-rendered default. `CanvasWasm` is an opt-in rich client
/// whose same-origin artifact identity and bounded size must be supplied by the
/// application; neither variant makes the client authoritative for results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityClientKind {
    SsrHtmx,
    CanvasWasm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityClientManifest {
    pub schema_version: i32,
    pub kind: ActivityClientKind,
    pub launch_path: String,
    pub wasm_path: Option<String>,
    pub wasm_sha256: Option<String>,
    pub artifact_size_bytes: u64,
}

impl ActivityClientManifest {
    /// Creates the server-rendered default for forms, navigation and simple activities.
    pub fn ssr_htmx(launch_path: impl Into<String>) -> Result<Self, ActivityContractError> {
        let manifest = Self {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            kind: ActivityClientKind::SsrHtmx,
            launch_path: launch_path.into(),
            wasm_path: None,
            wasm_sha256: None,
            artifact_size_bytes: 0,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Creates an explicitly opted-in Canvas/WebAssembly client manifest.
    pub fn canvas_wasm(
        launch_path: impl Into<String>,
        wasm_path: impl Into<String>,
        wasm_sha256: impl Into<String>,
        artifact_size_bytes: u64,
    ) -> Result<Self, ActivityContractError> {
        let manifest = Self {
            schema_version: ACTIVITY_SCHEMA_VERSION,
            kind: ActivityClientKind::CanvasWasm,
            launch_path: launch_path.into(),
            wasm_path: Some(wasm_path.into()),
            wasm_sha256: Some(wasm_sha256.into()),
            artifact_size_bytes,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validates schema, same-origin paths and the bounded rich-client artifact identity.
    pub fn validate(&self) -> Result<(), ActivityContractError> {
        if self.schema_version != ACTIVITY_SCHEMA_VERSION {
            return Err(ActivityContractError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if !valid_same_origin_path(&self.launch_path) {
            return Err(ActivityContractError::InvalidField("launch_path"));
        }
        match self.kind {
            ActivityClientKind::SsrHtmx
                if self.wasm_path.is_none()
                    && self.wasm_sha256.is_none()
                    && self.artifact_size_bytes == 0 =>
            {
                Ok(())
            }
            ActivityClientKind::CanvasWasm => {
                let path = self
                    .wasm_path
                    .as_deref()
                    .ok_or(ActivityContractError::InvalidField("wasm_path"))?;
                let digest = self
                    .wasm_sha256
                    .as_deref()
                    .ok_or(ActivityContractError::InvalidField("wasm_sha256"))?;
                if !valid_same_origin_path(path) || !path.ends_with(".wasm") {
                    return Err(ActivityContractError::InvalidField("wasm_path"));
                }
                if digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                {
                    return Err(ActivityContractError::InvalidField("wasm_sha256"));
                }
                if !(1..=16 * 1024 * 1024).contains(&self.artifact_size_bytes) {
                    return Err(ActivityContractError::InvalidField(
                        "artifact_size_bytes",
                    ));
                }
                Ok(())
            }
            ActivityClientKind::SsrHtmx => {
                Err(ActivityContractError::InvalidField("ssr_htmx_bundle"))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityAttempt {
    pub schema_version: i32,
    pub attempt_key: String,
    pub activity_id: i32,
    pub subject_user_id: i32,
    pub kind: ActivityKind,
    pub ruleset_version: String,
    pub started_at_epoch_seconds: i64,
    pub state_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityResult {
    pub schema_version: i32,
    pub attempt_key: String,
    pub activity_id: i32,
    pub subject_user_id: i32,
    pub ruleset_version: String,
    pub points: i32,
    pub max_score: i32,
    pub finished_at_epoch_seconds: i64,
    pub evidence_sha256: String,
    pub submission_key: String,
    pub policy_binding: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeActivityOutcome {
    pub points: i32,
    pub max_score: i32,
    pub evidence_sha256: String,
    pub submission_key: String,
    pub policy_binding: String,
}

/// Static-dispatch boundary that turns an untrusted submission into a
/// server-authored outcome. Implementations must load rules and answer keys
/// from trusted application state rather than from the submission.
pub trait ActivityEvaluator {
    type Submission: ?Sized;

    fn kind(&self) -> ActivityKind;

    fn evaluate(
        &self,
        attempt: &ActivityAttempt,
        submission: &Self::Submission,
    ) -> Result<AuthoritativeActivityOutcome, ActivityContractError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedActivityResult {
    actor_user_id: i32,
    attempt: ActivityAttempt,
    result: ActivityResult,
}

impl ValidatedActivityResult {
    pub fn actor_user_id(&self) -> i32 {
        self.actor_user_id
    }

    pub fn attempt(&self) -> &ActivityAttempt {
        &self.attempt
    }

    pub fn result(&self) -> &ActivityResult {
        &self.result
    }

    pub fn into_parts(self) -> (i32, ActivityAttempt, ActivityResult) {
        (self.actor_user_id, self.attempt, self.result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityContractError {
    Forbidden,
    InvalidIdentity,
    UnsupportedSchemaVersion(i32),
    InvalidField(&'static str),
}

impl std::fmt::Display for ActivityContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden => formatter.write_str("activity access denied"),
            Self::InvalidIdentity => formatter.write_str("authenticated activity actor is invalid"),
            Self::UnsupportedSchemaVersion(version) => write!(formatter, "unsupported activity schema version: {version}"),
            Self::InvalidField(field) => write!(formatter, "invalid activity contract field: {field}"),
        }
    }
}

impl std::error::Error for ActivityContractError {}

fn valid_key(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn valid_same_origin_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 256
        && !value.contains(['\\', '?', '#'])
        && value
            .split('/')
            .all(|segment| !matches!(segment, "." | ".."))
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b':')
        })
}

pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn valid_policy_binding(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 8_192
        && matches!(
            serde_json::from_str::<serde_json::Value>(value),
            Ok(serde_json::Value::Object(_))
        )
}

fn validate_attempt(
    context: &UserContext,
    attempt: &ActivityAttempt,
) -> Result<i32, ActivityContractError> {
    if attempt.schema_version != ACTIVITY_SCHEMA_VERSION {
        return Err(ActivityContractError::UnsupportedSchemaVersion(
            attempt.schema_version,
        ));
    }
    let actor_user_id = context
        .user_id
        .parse::<i32>()
        .map_err(|_| ActivityContractError::InvalidIdentity)?;
    RbacGuard::authorize_owner_or_role(
        context,
        &attempt.subject_user_id.to_string(),
        "admin",
    )
    .map_err(|_| ActivityContractError::Forbidden)?;

    if attempt.activity_id <= 0
        || attempt.subject_user_id <= 0
        || !valid_key(&attempt.attempt_key, 128)
        || !valid_key(&attempt.ruleset_version, 64)
    {
        return Err(ActivityContractError::InvalidField("identity binding"));
    }
    if attempt.started_at_epoch_seconds <= 0 {
        return Err(ActivityContractError::InvalidField("time ordering"));
    }
    if attempt.state_json.is_empty()
        || attempt.state_json.len() > 64 * 1024
        || !matches!(
            serde_json::from_str::<serde_json::Value>(&attempt.state_json),
            Ok(serde_json::Value::Object(_))
        )
    {
        return Err(ActivityContractError::InvalidField("state_json"));
    }
    Ok(actor_user_id)
}

fn validate_result(
    attempt: &ActivityAttempt,
    result: &ActivityResult,
) -> Result<(), ActivityContractError> {
    if result.schema_version != ACTIVITY_SCHEMA_VERSION {
        return Err(ActivityContractError::UnsupportedSchemaVersion(
            result.schema_version,
        ));
    }
    if attempt.attempt_key != result.attempt_key
        || attempt.activity_id != result.activity_id
        || attempt.subject_user_id != result.subject_user_id
        || attempt.ruleset_version != result.ruleset_version
    {
        return Err(ActivityContractError::InvalidField("identity binding"));
    }
    if result.finished_at_epoch_seconds < attempt.started_at_epoch_seconds {
        return Err(ActivityContractError::InvalidField("time ordering"));
    }
    if result.points < 0
        || result.max_score <= 0
        || result.points > result.max_score
        || result.max_score > 1_000_000
    {
        return Err(ActivityContractError::InvalidField("score bounds"));
    }
    if !valid_sha256(&result.evidence_sha256) {
        return Err(ActivityContractError::InvalidField("evidence_sha256"));
    }
    if !valid_key(&result.submission_key, 128) {
        return Err(ActivityContractError::InvalidField("submission_key"));
    }
    if !valid_policy_binding(&result.policy_binding) {
        return Err(ActivityContractError::InvalidField("policy_binding"));
    }
    Ok(())
}

pub fn evaluate_activity<E: ActivityEvaluator>(
    context: &UserContext,
    attempt: ActivityAttempt,
    submission: &E::Submission,
    finished_at_epoch_seconds: i64,
    evaluator: &E,
) -> Result<ValidatedActivityResult, ActivityContractError> {
    let actor_user_id = validate_attempt(context, &attempt)?;
    if evaluator.kind() != attempt.kind {
        return Err(ActivityContractError::InvalidField("activity kind"));
    }
    let outcome = evaluator.evaluate(&attempt, submission)?;
    let result = ActivityResult {
        schema_version: ACTIVITY_SCHEMA_VERSION,
        attempt_key: attempt.attempt_key.clone(),
        activity_id: attempt.activity_id,
        subject_user_id: attempt.subject_user_id,
        ruleset_version: attempt.ruleset_version.clone(),
        points: outcome.points,
        max_score: outcome.max_score,
        finished_at_epoch_seconds,
        evidence_sha256: outcome.evidence_sha256,
        submission_key: outcome.submission_key,
        policy_binding: outcome.policy_binding,
    };
    validate_result(&attempt, &result)?;

    Ok(ValidatedActivityResult {
        actor_user_id,
        attempt,
        result,
    })
}

"##;

#[cfg(test)]
mod tests {
    use super::{ACTIVITY_CONTRACT, ACTIVITY_CONTRACT_TESTS, SINGLE_CHOICE_EVALUATOR};

    #[test]
    fn generated_contract_binds_attempt_result_rules_and_evidence() {
        let contract =
            format!("{ACTIVITY_CONTRACT}{SINGLE_CHOICE_EVALUATOR}{ACTIVITY_CONTRACT_TESTS}");
        for field in [
            "attempt_key",
            "ruleset_version",
            "state_json",
            "evidence_sha256",
            "finished_at_epoch_seconds",
            "ActivityClientManifest",
            "ActivityEvaluator",
            "evaluate_activity",
            "SingleChoiceEvaluator",
            "SsrHtmx",
            "CanvasWasm",
            "artifact_size_bytes",
        ] {
            assert!(contract.contains(field));
        }
    }
}
