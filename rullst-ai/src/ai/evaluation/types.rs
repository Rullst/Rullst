use serde::Serialize;
use std::{fmt, time::Duration};

const MAX_EVAL_TURNS: u8 = 32;
const MAX_EVAL_PROMPT_BYTES: usize = 16 * 1_024;
const MAX_EVAL_RESPONSE_BYTES: usize = 2 * 1_024 * 1_024;
const MAX_EVAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Hard budgets independently applied to one adaptive evaluation scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AiEvaluationPolicy {
    pub(super) max_turns: u8,
    pub(super) max_prompt_bytes: usize,
    pub(super) max_response_bytes: usize,
    pub(super) turn_timeout: Duration,
}

impl AiEvaluationPolicy {
    /// Builds a policy no larger than the crate-wide evaluation ceilings.
    pub fn try_new(
        max_turns: u8,
        max_prompt_bytes: usize,
        max_response_bytes: usize,
        turn_timeout: Duration,
    ) -> Result<Self, AiEvaluationError> {
        if max_turns == 0 || max_turns > MAX_EVAL_TURNS {
            return Err(AiEvaluationError::InvalidConfiguration(
                "AI evaluation turns must be between 1 and 32",
            ));
        }
        if max_prompt_bytes == 0 || max_prompt_bytes > MAX_EVAL_PROMPT_BYTES {
            return Err(AiEvaluationError::InvalidConfiguration(
                "AI evaluation prompt limit must be between 1 byte and 16 KiB",
            ));
        }
        if max_response_bytes == 0 || max_response_bytes > MAX_EVAL_RESPONSE_BYTES {
            return Err(AiEvaluationError::InvalidConfiguration(
                "AI evaluation response limit must be between 1 byte and 2 MiB",
            ));
        }
        if turn_timeout.is_zero() || turn_timeout > MAX_EVAL_TIMEOUT {
            return Err(AiEvaluationError::InvalidConfiguration(
                "AI evaluation turn timeout must be between 1 nanosecond and 5 minutes",
            ));
        }
        Ok(Self {
            max_turns,
            max_prompt_bytes,
            max_response_bytes,
            turn_timeout,
        })
    }

    /// Maximum model calls in one scenario.
    #[must_use]
    pub const fn max_turns(self) -> u8 {
        self.max_turns
    }

    /// Maximum UTF-8 bytes in each generated prompt.
    #[must_use]
    pub const fn max_prompt_bytes(self) -> usize {
        self.max_prompt_bytes
    }

    /// Maximum UTF-8 bytes exposed to the strategy from each response.
    #[must_use]
    pub const fn max_response_bytes(self) -> usize {
        self.max_response_bytes
    }

    /// Deadline independently applied around each provider call.
    #[must_use]
    pub const fn turn_timeout(self) -> Duration {
        self.turn_timeout
    }
}

impl Default for AiEvaluationPolicy {
    fn default() -> Self {
        Self {
            max_turns: 8,
            max_prompt_bytes: MAX_EVAL_PROMPT_BYTES,
            max_response_bytes: MAX_EVAL_RESPONSE_BYTES,
            turn_timeout: Duration::from_secs(30),
        }
    }
}

/// Low-cardinality observation classification supplied to an evaluation strategy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AiEvaluationOutcome {
    /// The provider produced a response within the configured budget.
    Response,
    /// Rullst's mandatory input guardrail rejected the generated prompt.
    GuardrailBlocked,
    /// The selected provider reported the requested operation unsupported.
    Unsupported,
    /// The provider or its transport returned another failure.
    ProviderFailure,
    /// The evaluator's independent per-turn deadline elapsed.
    Deadline,
}

/// One transient observation. Raw response text is never retained in the report.
#[non_exhaustive]
pub struct AiEvaluationObservation<'a> {
    pub(super) turn: u8,
    pub(super) outcome: AiEvaluationOutcome,
    pub(super) response: Option<&'a str>,
    pub(super) code: Option<&'a str>,
}

impl AiEvaluationObservation<'_> {
    /// One-based turn number.
    #[must_use]
    pub const fn turn(&self) -> u8 {
        self.turn
    }

    /// Low-cardinality provider/guardrail outcome.
    #[must_use]
    pub const fn outcome(&self) -> AiEvaluationOutcome {
        self.outcome
    }

    /// Bounded raw provider response, available only during strategy evaluation.
    #[must_use]
    pub const fn response(&self) -> Option<&str> {
        self.response
    }

    /// Stable guardrail/capability code when one exists.
    #[must_use]
    pub const fn code(&self) -> Option<&str> {
        self.code
    }
}

impl fmt::Debug for AiEvaluationObservation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AiEvaluationObservation")
            .field("turn", &self.turn)
            .field("outcome", &self.outcome)
            .field("response", &self.response.map(str::len))
            .field("code", &self.code)
            .finish()
    }
}

pub(super) enum DecisionKind {
    Pass,
    Fail,
    Inconclusive,
    Continue,
}

/// A strategy's terminal result or bounded next prompt.
#[non_exhaustive]
pub struct AiEvaluationDecision {
    pub(super) kind: DecisionKind,
    pub(super) value: String,
}

impl AiEvaluationDecision {
    /// Marks the scenario passed with a stable low-cardinality reason code.
    pub fn pass(code: &'static str) -> Self {
        Self {
            kind: DecisionKind::Pass,
            value: code.to_string(),
        }
    }

    /// Marks the scenario failed with a stable low-cardinality reason code.
    pub fn fail(code: &'static str) -> Self {
        Self {
            kind: DecisionKind::Fail,
            value: code.to_string(),
        }
    }

    /// Marks the scenario inconclusive without converting infrastructure failure into a pass.
    pub fn inconclusive(code: &'static str) -> Self {
        Self {
            kind: DecisionKind::Inconclusive,
            value: code.to_string(),
        }
    }

    /// Continues the adaptive scenario with a strategy-generated prompt.
    pub fn continue_with(prompt: impl Into<String>) -> Self {
        Self {
            kind: DecisionKind::Continue,
            value: prompt.into(),
        }
    }
}

impl fmt::Debug for AiEvaluationDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, value) = match self.kind {
            DecisionKind::Pass => ("pass", self.value.as_str()),
            DecisionKind::Fail => ("fail", self.value.as_str()),
            DecisionKind::Inconclusive => ("inconclusive", self.value.as_str()),
            DecisionKind::Continue => ("continue", "[REDACTED]"),
        };
        formatter
            .debug_struct("AiEvaluationDecision")
            .field("kind", &kind)
            .field("value", &value)
            .finish()
    }
}

/// Application-defined adaptive scenario.
///
/// `observe` may inspect bounded raw model output to choose its next prompt.
/// The strategy must not persist or log that text unless application policy
/// explicitly permits it.
pub trait AiEvaluationStrategy: Send {
    /// Produces the first prompt in the scenario.
    fn initial_prompt(&mut self) -> String;

    /// Classifies an observation as terminal or produces the next prompt.
    fn observe(&mut self, observation: &AiEvaluationObservation<'_>) -> AiEvaluationDecision;
}

/// Terminal status of one bounded scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AiEvaluationStatus {
    /// The strategy's stated assertions passed.
    Passed,
    /// The strategy observed a behavior that violated its assertions.
    Failed,
    /// Infrastructure or the turn ceiling prevented a valid conclusion.
    Inconclusive,
}

/// Secret-minimized metadata for one completed turn.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AiEvaluationTurn {
    pub(super) turn: u8,
    pub(super) outcome: AiEvaluationOutcome,
    pub(super) prompt_bytes: usize,
    pub(super) response_bytes: usize,
}

impl AiEvaluationTurn {
    /// One-based turn number.
    #[must_use]
    pub const fn turn(&self) -> u8 {
        self.turn
    }

    /// Low-cardinality outcome without provider error or model text.
    #[must_use]
    pub const fn outcome(&self) -> AiEvaluationOutcome {
        self.outcome
    }

    /// UTF-8 size of the prompt, not its content.
    #[must_use]
    pub const fn prompt_bytes(&self) -> usize {
        self.prompt_bytes
    }

    /// UTF-8 size of a successful response, not its content.
    #[must_use]
    pub const fn response_bytes(&self) -> usize {
        self.response_bytes
    }
}

/// Versioned, raw-content-free result suitable for JSON evidence export.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct AiEvaluationReport {
    pub(super) schema_version: u8,
    pub(super) suite_id: String,
    pub(super) subject: String,
    pub(super) provider: &'static str,
    pub(super) status: AiEvaluationStatus,
    pub(super) terminal_code: String,
    pub(super) turns: Vec<AiEvaluationTurn>,
}

impl AiEvaluationReport {
    /// Report schema version. v12 emits version 1.
    #[must_use]
    pub const fn schema_version(&self) -> u8 {
        self.schema_version
    }

    /// Caller-selected versioned suite identifier.
    #[must_use]
    pub fn suite_id(&self) -> &str {
        &self.suite_id
    }

    /// Exact provider/model/configuration label supplied by the evaluator caller.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Stable transport name reported by [`crate::ai::AiProvider`].
    #[must_use]
    pub const fn provider(&self) -> &'static str {
        self.provider
    }

    /// Passed, failed or explicitly inconclusive.
    #[must_use]
    pub const fn status(&self) -> AiEvaluationStatus {
        self.status
    }

    /// Stable strategy reason code or `turn_limit`.
    #[must_use]
    pub fn terminal_code(&self) -> &str {
        &self.terminal_code
    }

    /// Bounded turn metadata without prompts, responses or provider errors.
    #[must_use]
    pub fn turns(&self) -> &[AiEvaluationTurn] {
        &self.turns
    }
}

/// Failures in evaluator configuration or resource enforcement.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum AiEvaluationError {
    /// A policy, suite label, subject label, prompt or terminal code was invalid.
    #[error("invalid AI evaluation configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// Cancellation stopped the scenario before a terminal strategy decision.
    #[error("AI evaluation was cancelled")]
    Cancelled,
    /// A provider response exceeded the independent evaluator ceiling.
    #[error("AI evaluation response exceeded its configured limit")]
    ResponseTooLarge,
}
