#![cfg(feature = "ai")]
#![allow(clippy::expect_used)]

use rullst::ai::{
    AdaptiveAiEvaluator, AiCancellation, AiEvaluationDecision, AiEvaluationObservation,
    AiEvaluationStatus, AiEvaluationStrategy, providers::openai::OpenAiProvider,
};

struct FacadeScenario;

impl AiEvaluationStrategy for FacadeScenario {
    fn initial_prompt(&mut self) -> String {
        "Return a short safe response".to_string()
    }

    fn observe(&mut self, observation: &AiEvaluationObservation<'_>) -> AiEvaluationDecision {
        if observation.response().is_some() {
            AiEvaluationDecision::pass("facade_contract_passed")
        } else {
            AiEvaluationDecision::inconclusive("provider_unavailable")
        }
    }
}

#[tokio::test]
async fn umbrella_ai_feature_exposes_adaptive_evaluation() {
    let evaluator = AdaptiveAiEvaluator::new(OpenAiProvider::new("mock_eval"));
    let report = evaluator
        .run(
            "facade-adaptive-v1",
            "openai:offline",
            &mut FacadeScenario,
            &AiCancellation::new(),
        )
        .await
        .expect("facade evaluation succeeds");
    assert_eq!(report.status(), AiEvaluationStatus::Passed);
}
