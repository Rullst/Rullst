#![allow(clippy::expect_used)]

use rullst_ai::{
    AdaptiveAiEvaluator, AiCancellation, AiEvaluationDecision, AiEvaluationObservation,
    AiEvaluationStatus, AiEvaluationStrategy, providers::openai::OpenAiProvider,
};

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
