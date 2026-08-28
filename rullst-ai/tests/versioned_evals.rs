//! Versioned deterministic eval runner for the bounded built-in guardrails.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_ai::ai::providers::{
    anthropic::AnthropicProvider, deepseek::DeepSeekProvider, gemini::GeminiProvider,
    ollama::OllamaProvider, openai::OpenAiProvider,
};
use rullst_ai::ai::{AiError, AiGuardrails, AiProvider};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvalSuite {
    schema_version: u32,
    suite_id: String,
    scope: String,
    cases: Vec<EvalCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvalCase {
    id: String,
    category: String,
    input: String,
    expected_threat: Option<String>,
    expected_redacted: Option<String>,
}

fn suite() -> EvalSuite {
    serde_json::from_str(include_str!("../evals/guardrails-v1.json"))
        .expect("versioned eval dataset must parse")
}

fn offline_providers() -> Vec<Arc<dyn AiProvider>> {
    vec![
        Arc::new(OpenAiProvider::new("mock_eval")),
        Arc::new(AnthropicProvider::new("mock_eval")),
        Arc::new(GeminiProvider::new("mock_eval")),
        Arc::new(OllamaProvider::new("mock_eval", "mock-model")),
        Arc::new(DeepSeekProvider::new("mock_eval")),
    ]
}

#[tokio::test]
// TM-AI-01 and TM-AI-02: the versioned offline corpus must match every built-in transport.
async fn versioned_guardrail_evals_match_every_offline_provider() {
    let suite = suite();
    assert_eq!(suite.schema_version, 1);
    assert_eq!(suite.suite_id, "rullst-ai-guardrails-v1");
    assert!(suite.scope.contains("not a safety benchmark"));
    assert!(!suite.cases.is_empty());
    let ids = suite
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), suite.cases.len(), "eval IDs must be unique");

    for case in &suite.cases {
        let report = AiGuardrails::inspect(&case.input);
        match case.category.as_str() {
            "prompt_injection" | "jailbreak" => {
                let expected = case
                    .expected_threat
                    .as_deref()
                    .expect("blocked cases declare a threat code");
                assert_eq!(
                    report.threat().map(|threat| threat.code()),
                    Some(expected),
                    "{} local guardrail regression",
                    case.id
                );
                assert!(case.expected_redacted.is_none());
            }
            "pii" => {
                assert!(report.threat().is_none());
                assert!(report.pii_was_masked());
                assert_eq!(
                    report.redacted_text(),
                    case.expected_redacted
                        .as_deref()
                        .expect("PII cases declare exact redaction"),
                    "{} local PII regression",
                    case.id
                );
                assert!(case.expected_threat.is_none());
            }
            category => panic!("unsupported eval category {category}"),
        }
    }

    for provider in offline_providers() {
        for case in &suite.cases {
            let first = provider.prompt(&case.input).await;
            match case.category.as_str() {
                "prompt_injection" | "jailbreak" => assert!(
                    matches!(
                        &first,
                        Err(AiError::BlockedByFirewall(code))
                            if Some(code.as_str()) == case.expected_threat.as_deref()
                    ),
                    "{} {} did not return the expected block: {first:?}",
                    case.id,
                    provider.provider_name()
                ),
                "pii" => {
                    let first = first.expect("offline PII eval");
                    let second = provider
                        .prompt(&case.input)
                        .await
                        .expect("deterministic offline PII eval");
                    assert_eq!(first, second);
                    assert!(
                        !first.contains(&case.input),
                        "{} {} retained raw PII",
                        case.id,
                        provider.provider_name()
                    );
                    assert!(
                        first.contains(
                            case.expected_redacted
                                .as_deref()
                                .expect("PII redaction fixture")
                        ),
                        "{} {} did not dispatch the expected redaction",
                        case.id,
                        provider.provider_name()
                    );
                }
                _ => unreachable!("dataset categories were validated above"),
            }
        }
    }
}
