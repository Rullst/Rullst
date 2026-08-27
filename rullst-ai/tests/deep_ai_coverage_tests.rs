#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_ai::ai::providers::anthropic::AnthropicProvider;
use rullst_ai::ai::providers::deepseek::DeepSeekProvider;
use rullst_ai::ai::providers::gemini::GeminiProvider;
use rullst_ai::ai::providers::ollama::OllamaProvider;
use rullst_ai::ai::providers::openai::OpenAiProvider;
use rullst_ai::ai::rag::build_rag_prompt;
use rullst_ai::ai::tools::{
    AiTool, InMemoryToolAuditTrail, ToolExecutionContext, ToolExecutionPolicy, ToolParam,
    ToolRegistry, ToolRisk,
};
use rullst_ai::ai::{AiClient, AiProvider};
use serde_json::Value;

struct WeatherTool;
impl AiTool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
    fn description(&self) -> &str {
        "Get current weather in given city"
    }
    fn parameters(&self) -> Vec<ToolParam> {
        vec![ToolParam {
            name: "city".into(),
            param_type: "string".into(),
            description: "Target city name".into(),
            required: true,
        }]
    }
    fn risk(&self) -> ToolRisk {
        ToolRisk::ReadOnly
    }
    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let city = payload
            .get("city")
            .and_then(|c| c.as_str())
            .unwrap_or("Unknown");
        Ok(serde_json::json!({ "city": city, "temp_c": 22 }))
    }
}

#[tokio::test]
async fn test_ai_provider_offline_mock_fallbacks() {
    // 1. Gemini
    let gemini = GeminiProvider::new("mock_key");
    let res = gemini.prompt("Hello").await;
    assert!(res.is_ok());

    // 2. OpenAI
    let openai = OpenAiProvider::new("mock_key");
    let res = openai.prompt("Hello").await;
    assert!(res.is_ok());

    // 3. Anthropic
    let claude = AnthropicProvider::new("mock_key");
    let res = claude.prompt("Hello").await;
    assert!(res.is_ok());

    // 4. Ollama
    let ollama = OllamaProvider::new("mock_ollama", "llama3");
    let res = ollama.prompt("Hello").await;
    assert!(res.is_ok());

    // 5. DeepSeek
    let deepseek = DeepSeekProvider::new("mock_key");
    let res = deepseek.prompt("Hello").await;
    assert!(res.is_ok());
}

#[test]
fn test_rag_and_tools_helpers() {
    // RAG prompt builder
    let contexts = vec!["Rust is fast and memory safe.".to_string()];
    let prompt = build_rag_prompt("Why use Rust?", &contexts);
    assert!(prompt.contains("Why use Rust?"));
    assert!(prompt.contains("fast and memory safe"));

    // Tool registry
    let mut registry = ToolRegistry::new();
    registry.register(WeatherTool).unwrap();
    let policy = ToolExecutionPolicy::new(["get_weather"]).unwrap();
    let mut context = ToolExecutionContext::new("weather-user", ["get_weather"], 1).unwrap();
    let audit = InMemoryToolAuditTrail::new(16).unwrap();

    let schema = registry.export_openai_schema(&policy);
    assert!(schema.is_array());
    assert_eq!(schema.as_array().unwrap().len(), 1);

    let res = registry.execute(
        "get_weather",
        serde_json::json!({ "city": "Curitiba" }),
        &mut context,
        &policy,
        &audit,
    );
    assert!(res.is_ok());
    assert_eq!(res.unwrap().get("temp_c").unwrap().as_i64(), Some(22));
}

#[tokio::test]
async fn test_ai_client_and_messages() {
    let mock = OpenAiProvider::new("mock_key");
    let client = AiClient::new(mock);

    let prompt_res = client.prompt("Hello").await;
    assert!(prompt_res.is_ok());

    let chat_res = client
        .chat()
        .system("You are a helpful assistant")
        .user("Hello!")
        .send()
        .await;
    assert!(chat_res.is_ok());
}
