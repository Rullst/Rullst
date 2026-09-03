//! Bounded adaptive evaluation orchestration for configured AI providers.

use super::{AiCancellation, AiError, AiGuardrails, AiProvider};
mod types;

use types::DecisionKind;
pub use types::{
    AiEvaluationDecision, AiEvaluationError, AiEvaluationObservation, AiEvaluationOutcome,
    AiEvaluationPolicy, AiEvaluationReport, AiEvaluationStatus, AiEvaluationStrategy,
    AiEvaluationTurn,
};

const MAX_EVAL_IDENTIFIER_BYTES: usize = 128;
const MAX_EVAL_CODE_BYTES: usize = 64;

/// Static-dispatch adaptive evaluator over the guarded high-level client.
#[non_exhaustive]
pub struct AdaptiveAiEvaluator<P> {
    provider_client: P,
    provider: &'static str,
    policy: AiEvaluationPolicy,
}

impl<P> AdaptiveAiEvaluator<P>
where
    P: AiProvider,
{
    /// Creates an evaluator with the crate-wide bounded default policy.
    #[must_use]
    pub fn new(provider: P) -> Self {
        let provider_name = provider.provider_name();
        Self {
            provider_client: provider,
            provider: provider_name,
            policy: AiEvaluationPolicy::default(),
        }
    }

    /// Selects a validated scenario policy.
    #[must_use]
    pub fn with_policy(mut self, policy: AiEvaluationPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Runs an adaptive scenario without retaining prompts or responses in the report.
    pub async fn run<S>(
        &self,
        suite_id: impl Into<String>,
        subject: impl Into<String>,
        strategy: &mut S,
        cancellation: &AiCancellation,
    ) -> Result<AiEvaluationReport, AiEvaluationError>
    where
        S: AiEvaluationStrategy,
    {
        let suite_id = suite_id.into();
        let subject = subject.into();
        validate_identifier(&suite_id, "AI evaluation suite ID is invalid")?;
        validate_identifier(&subject, "AI evaluation subject is invalid")?;
        if cancellation.is_cancelled() {
            return Err(AiEvaluationError::Cancelled);
        }

        let mut prompt = strategy.initial_prompt();
        let mut turns = Vec::with_capacity(usize::from(self.policy.max_turns));
        let mut turn = 1_u8;
        loop {
            validate_prompt(&prompt, self.policy.max_prompt_bytes)?;
            let guarded_prompt = AiGuardrails::prepare(&prompt);
            let call = async {
                let guarded_prompt = guarded_prompt?;
                self.provider_client.prompt(&guarded_prompt).await
            };
            let call = tokio::time::timeout(self.policy.turn_timeout, call);
            let result = tokio::select! {
                () = cancellation.cancelled() => return Err(AiEvaluationError::Cancelled),
                result = call => result,
            };
            if cancellation.is_cancelled() {
                return Err(AiEvaluationError::Cancelled);
            }
            let (outcome, response, code) = classify_observation(result)?;
            let response_bytes = response.as_ref().map_or(0, String::len);
            if response_bytes > self.policy.max_response_bytes {
                return Err(AiEvaluationError::ResponseTooLarge);
            }
            turns.push(AiEvaluationTurn {
                turn,
                outcome,
                prompt_bytes: prompt.len(),
                response_bytes,
            });
            let observation = AiEvaluationObservation {
                turn,
                outcome,
                response: response.as_deref(),
                code: code.as_deref(),
            };
            let decision = strategy.observe(&observation);
            match decision.kind {
                DecisionKind::Pass => {
                    return build_report(
                        suite_id,
                        subject,
                        self.provider,
                        AiEvaluationStatus::Passed,
                        decision.value,
                        turns,
                    );
                }
                DecisionKind::Fail => {
                    return build_report(
                        suite_id,
                        subject,
                        self.provider,
                        AiEvaluationStatus::Failed,
                        decision.value,
                        turns,
                    );
                }
                DecisionKind::Inconclusive => {
                    return build_report(
                        suite_id,
                        subject,
                        self.provider,
                        AiEvaluationStatus::Inconclusive,
                        decision.value,
                        turns,
                    );
                }
                DecisionKind::Continue if turn == self.policy.max_turns => {
                    return build_report(
                        suite_id,
                        subject,
                        self.provider,
                        AiEvaluationStatus::Inconclusive,
                        "turn_limit".to_string(),
                        turns,
                    );
                }
                DecisionKind::Continue => {
                    prompt = decision.value;
                    turn = turn.saturating_add(1);
                }
            }
        }
    }
}

type ProviderObservation = (AiEvaluationOutcome, Option<String>, Option<String>);

fn classify_observation(
    result: Result<Result<String, AiError>, tokio::time::error::Elapsed>,
) -> Result<ProviderObservation, AiEvaluationError> {
    match result {
        Err(_) => Ok((AiEvaluationOutcome::Deadline, None, None)),
        Ok(Ok(response)) => Ok((AiEvaluationOutcome::Response, Some(response), None)),
        Ok(Err(AiError::BlockedByFirewall(code))) => {
            Ok((AiEvaluationOutcome::GuardrailBlocked, None, Some(code)))
        }
        Ok(Err(AiError::UnsupportedCapability { capability, .. })) => Ok((
            AiEvaluationOutcome::Unsupported,
            None,
            Some(capability.to_string()),
        )),
        Ok(Err(AiError::Cancelled)) => Err(AiEvaluationError::Cancelled),
        Ok(Err(_)) => Ok((AiEvaluationOutcome::ProviderFailure, None, None)),
    }
}

fn build_report(
    suite_id: String,
    subject: String,
    provider: &'static str,
    status: AiEvaluationStatus,
    terminal_code: String,
    turns: Vec<AiEvaluationTurn>,
) -> Result<AiEvaluationReport, AiEvaluationError> {
    validate_code(&terminal_code)?;
    Ok(AiEvaluationReport {
        schema_version: 1,
        suite_id,
        subject,
        provider,
        status,
        terminal_code,
        turns,
    })
}

fn validate_identifier(value: &str, error: &'static str) -> Result<(), AiEvaluationError> {
    if value.is_empty()
        || value.len() > MAX_EVAL_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AiEvaluationError::InvalidConfiguration(error));
    }
    Ok(())
}

fn validate_code(code: &str) -> Result<(), AiEvaluationError> {
    if code.is_empty()
        || code.len() > MAX_EVAL_CODE_BYTES
        || !code.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
    {
        return Err(AiEvaluationError::InvalidConfiguration(
            "AI evaluation terminal code is invalid",
        ));
    }
    Ok(())
}

fn validate_prompt(prompt: &str, limit: usize) -> Result<(), AiEvaluationError> {
    if prompt.trim().is_empty() || prompt.len() > limit || prompt.contains('\0') {
        return Err(AiEvaluationError::InvalidConfiguration(
            "AI evaluation prompt is empty, oversized or contains NUL",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
