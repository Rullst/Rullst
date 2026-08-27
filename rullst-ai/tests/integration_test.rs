// tests/integration_test.rs — Comprehensive unit and integration tests for Rullst AI.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use rullst_ai::ai::rag::build_rag_prompt;
use rullst_ai::ai::tools::{
    AiTool, InMemoryToolAuditTrail, ToolExecutionContext, ToolExecutionPolicy, ToolParam,
    ToolRegistry, ToolRisk,
};
use rullst_ai::ai::{AiError, AiProvider, ChatBuilder, FallbackProvider, Message};
use serde_json::{Value, json};
use std::sync::Arc;

struct MockProvider {
    name: String,
    fail: bool,
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        if self.fail {
            Err(AiError::ApiError(format!("{} failed", self.name)))
        } else {
            Ok(format!("{}: response to '{}'", self.name, text))
        }
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        if self.fail {
            Err(AiError::ApiError(format!("{} failed", self.name)))
        } else {
            Ok(format!(
                "{}: chatted {} messages",
                self.name,
                messages.len()
            ))
        }
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        if self.fail {
            Err(AiError::ApiError("embedding failed".to_string()))
        } else {
            Ok(vec![0.1, 0.2, 0.3])
        }
    }
}

#[tokio::test]
async fn test_fallback_provider_failover() {
    let p1 = Arc::new(MockProvider {
        name: "Primary".to_string(),
        fail: true,
    });
    let p2 = Arc::new(MockProvider {
        name: "Secondary".to_string(),
        fail: false,
    });

    let fallback = FallbackProvider::new(vec![p1, p2]);

    let prompt_res = fallback.prompt("Hello AI").await;
    assert!(prompt_res.is_ok());
    assert_eq!(prompt_res.unwrap(), "Secondary: response to 'Hello AI'");

    let chat_res = fallback.chat(&[Message::user("hi")]).await;
    assert!(chat_res.is_ok());
    assert_eq!(chat_res.unwrap(), "Secondary: chatted 1 messages");

    let embed_res = fallback.embed("sample text").await;
    assert!(embed_res.is_ok());
    assert_eq!(embed_res.unwrap(), vec![0.1, 0.2, 0.3]);
}

#[tokio::test]
async fn test_chat_builder_flow() {
    let p = Arc::new(MockProvider {
        name: "ChatBot".to_string(),
        fail: false,
    });

    let builder = ChatBuilder::new(p)
        .system("You are a helpful assistant")
        .user("What is Rust?")
        .assistant("Rust is a systems programming language")
        .user("Tell me more");

    let reply = builder.send().await;
    assert!(reply.is_ok());
    assert_eq!(reply.unwrap(), "ChatBot: chatted 4 messages");
}

#[test]
fn test_rag_prompt_construction() {
    let contexts = vec![
        "Rullst is an AI-Native Rust Framework.".to_string(),
        "It supports async tokio tasks natively.".to_string(),
    ];
    let prompt = build_rag_prompt("What is Rullst?", &contexts);
    assert!(prompt.contains("Rullst is an AI-Native Rust Framework."));
    assert!(prompt.contains("It supports async tokio tasks natively."));
    assert!(prompt.contains("Question: What is Rullst?"));
    assert!(prompt.contains("Answer:"));
}

struct CalculatorTool;

impl AiTool for CalculatorTool {
    fn name(&self) -> &str {
        "calculate_sum"
    }

    fn description(&self) -> &str {
        "Adds two numbers together"
    }

    fn parameters(&self) -> Vec<ToolParam> {
        vec![
            ToolParam {
                name: "a".to_string(),
                param_type: "number".to_string(),
                description: "First number".to_string(),
                required: true,
            },
            ToolParam {
                name: "b".to_string(),
                param_type: "number".to_string(),
                description: "Second number".to_string(),
                required: true,
            },
        ]
    }

    fn risk(&self) -> ToolRisk {
        ToolRisk::ReadOnly
    }

    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let a = payload.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = payload.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok(json!({ "result": a + b }))
    }
}

#[test]
fn test_tool_registry_and_execution() {
    let mut registry = ToolRegistry::new();
    registry.register(CalculatorTool).unwrap();
    let policy = ToolExecutionPolicy::new(["calculate_sum"]).unwrap();
    let mut context = ToolExecutionContext::new("calculator-user", ["calculate_sum"], 1).unwrap();
    let audit = InMemoryToolAuditTrail::new(16).unwrap();

    // Schema export
    let schema = registry.export_openai_schema(&policy);
    assert!(schema.is_array());
    let schema_str = schema.to_string();
    assert!(schema_str.contains("calculate_sum"));
    assert!(schema_str.contains("Adds two numbers together"));

    // Tool execution
    let result = registry.execute(
        "calculate_sum",
        json!({ "a": 10.5, "b": 2.5 }),
        &mut context,
        &policy,
        &audit,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["result"], 13.0);

    // Unknown tool
    assert!(
        registry
            .execute("unknown_tool", json!({}), &mut context, &policy, &audit,)
            .is_err()
    );
}
