#![allow(clippy::expect_used)]

use rullst_ai::{
    AdaptiveAiEvaluator, AiCancellation, AiError, AiEvaluationDecision, AiEvaluationObservation,
    AiEvaluationPolicy, AiEvaluationStatus, AiEvaluationStrategy, AiProvider, Message,
    providers::openai::OpenAiProvider,
};
use std::time::Duration;

struct TwoTurnOfflineScenario;

impl AiEvaluationStrategy for TwoTurnOfflineScenario {
    fn initial_prompt(&mut self) -> String {
        "Provide one short policy-safe greeting".to_string()
    }

    fn observe(&mut self, observation: &AiEvaluationObservation<'_>) -> AiEvaluationDecision {
        if observation.turn() == 1 {
            let response_bytes = observation.response().map_or(0, str::len);
            AiEvaluationDecision::continue_with(format!(
                "Provide another safe greeting after a {response_bytes}-byte response"
            ))
        } else if observation.response().is_some() {
            AiEvaluationDecision::pass("two_turn_contract_passed")
        } else {
            AiEvaluationDecision::inconclusive("provider_unavailable")
        }
    }
}

#[tokio::test]
async fn public_adaptive_runner_emits_a_bounded_content_free_report() {
    let provider = OpenAiProvider::new("mock_eval").with_model("offline-eval-model");
    let evaluator = AdaptiveAiEvaluator::new(provider);
    let mut scenario = TwoTurnOfflineScenario;
    let report = evaluator
        .run(
            "rullst-ai-adaptive-smoke-v1",
            "openai:offline-eval-model",
            &mut scenario,
            &AiCancellation::new(),
        )
        .await
        .expect("offline adaptive scenario runs");

    assert_eq!(report.status(), AiEvaluationStatus::Passed);
    assert_eq!(report.turns().len(), 2);
    let json = serde_json::to_string(&report).expect("report serializes");
    assert!(!json.contains("policy-safe greeting"));
    assert!(!json.contains("Mock response"));
}

/// Answers every prompt with three bytes.
struct ThreeBytes;

#[async_trait::async_trait]
impl AiProvider for ThreeBytes {
    async fn prompt(&self, _text: &str) -> Result<String, AiError> {
        Ok("abc".to_string())
    }

    async fn chat(&self, _messages: &[Message]) -> Result<String, AiError> {
        Ok("abc".to_string())
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        Ok(vec![1.0])
    }
}

/// A 64-byte terminal code, the longest accepted.
const CODE_64: &str = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijkl";

/// Sends a 24-byte prompt and passes with the longest code.
struct AtTheCeilings;

impl AiEvaluationStrategy for AtTheCeilings {
    fn initial_prompt(&mut self) -> String {
        "p".repeat(24)
    }

    fn observe(&mut self, _observation: &AiEvaluationObservation<'_>) -> AiEvaluationDecision {
        AiEvaluationDecision::pass(CODE_64)
    }
}

#[tokio::test]
async fn every_evaluation_bound_accepts_its_exact_ceiling() {
    assert_eq!(CODE_64.len(), 64);
    // A 24-byte prompt budget, a 3-byte response budget and a 128-byte suite ID.
    let policy = AiEvaluationPolicy::try_new(1, 24, 3, Duration::from_secs(5)).expect("policy");
    let suite_id = "s".repeat(128);
    let report = AdaptiveAiEvaluator::new(ThreeBytes)
        .with_policy(policy)
        .run(
            suite_id.clone(),
            "fixture:model",
            &mut AtTheCeilings,
            &AiCancellation::new(),
        )
        .await
        .expect("every value at its ceiling is accepted");
    assert_eq!(report.status(), AiEvaluationStatus::Passed);
    assert_eq!(report.suite_id(), suite_id);
    assert_eq!(report.terminal_code(), CODE_64);
    assert_eq!(
        (
            report.turns()[0].prompt_bytes(),
            report.turns()[0].response_bytes()
        ),
        (24, 3)
    );
}
