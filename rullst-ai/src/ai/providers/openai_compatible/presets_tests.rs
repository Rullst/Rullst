//! The DeepSeek and Ollama presets against a loopback fixture server.

use super::tests::serve_once;
use super::*;
use crate::ai::JsonCapability;
use std::time::Duration;

fn body(request: &str) -> serde_json::Value {
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("request body exists");
    serde_json::from_str(body).expect("request body is JSON")
}

fn schema() -> StructuredOutputSchema {
    StructuredOutputSchema::new(
        "answer",
        serde_json::json!({
            "type": "object",
            "properties": {"ok": {"type": "boolean"}},
            "required": ["ok"]
        }),
    )
    .expect("valid schema")
}

/// The DeepSeek preset pointed at the fixture's root.
fn deepseek(url: &str) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::deepseek("sk-fixture")
        .expect("valid key")
        .with_test_base_url(url.trim_end_matches("/v1"))
}

#[test]
fn deepseek_declares_its_service_and_capabilities() {
    let provider = OpenAiCompatibleProvider::deepseek("sk-live").expect("valid key");
    assert_eq!(provider.provider_name(), "DeepSeek");
    assert_eq!(provider.base_url, "https://api.deepseek.com");
    assert_eq!(provider.model, "deepseek-flash");
    assert!(!provider.mode.is_mock());
    let capabilities = provider.capabilities();
    assert!(capabilities.json_schema && capabilities.streaming);
    assert_eq!(capabilities.json, JsonCapability::NativeMode);
    assert!(!capabilities.embeddings && !capabilities.vision);
    assert!(format!("{provider:?}").contains("DeepSeek"));
    assert!(!format!("{provider:?}").contains("sk-live"));

    for key in ["", " mock_key ", "mock_deepseek"] {
        let offline = OpenAiCompatibleProvider::deepseek(key).expect("offline key");
        assert!(offline.mode.is_mock(), "{key:?}");
    }
    assert!(OpenAiCompatibleProvider::deepseek("bad key").is_err());

    let chat = provider
        .try_with_model("deepseek-chat")
        .expect("valid model");
    assert!(!chat.capabilities().json_schema);
    assert!(chat.capabilities().streaming);
    let pro = chat.try_with_model("deepseek-v4-pro").expect("valid model");
    assert!(pro.capabilities().json_schema);
    assert!(pro.try_with_model(" ").is_err());
}

#[tokio::test]
async fn deepseek_offline_mode_covers_its_capabilities_without_http() {
    let provider = OpenAiCompatibleProvider::deepseek("mock_offline").expect("offline key");
    assert!(provider.prompt("hello").await.is_ok());
    assert!(provider.prompt_json("hello").await.is_ok());
    assert!(provider.structured_output("hello", &schema()).await.is_ok());
    assert!(matches!(
        provider.embed("hello").await,
        Err(AiError::UnsupportedCapability {
            provider: "DeepSeek",
            ..
        })
    ));
    assert!(matches!(
        provider.prompt_with_image("hello", b"image").await,
        Err(AiError::UnsupportedCapability { .. })
    ));
    let chat = provider
        .try_with_model("deepseek-chat")
        .expect("valid model");
    assert!(matches!(
        chat.structured_output("hello", &schema()).await,
        Err(AiError::UnsupportedCapability {
            capability: "JSON Schema output for this DeepSeek model",
            ..
        })
    ));
}

#[tokio::test]
async fn deepseek_chat_and_json_mode_use_chat_completions_with_bearer() {
    let (url, request) = serve_once(
        r#"{"choices":[{"message":{"content":"{}"}}],"usage":{"prompt_tokens":9,"completion_tokens":1,"prompt_cache_hit_tokens":8}}"#,
        None,
    );
    let answer = deepseek(&url)
        .chat_with_usage(&[Message::user("hello")])
        .await
        .expect("DeepSeek answer");
    let usage = answer.usage().expect("usage");
    assert_eq!(
        (usage.total_tokens(), usage.cached_input_tokens()),
        (Some(10), Some(8))
    );
    let request = request
        .recv_timeout(Duration::from_secs(2))
        .expect("request");
    assert!(request.starts_with("POST /chat/completions HTTP/1.1\r\n"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer sk-fixture")
    );
    assert_eq!(body(&request)["model"], "deepseek-flash");

    let (url, request) = serve_once(r#"{"choices":[{"message":{"content":"{}"}}]}"#, None);
    assert_eq!(
        deepseek(&url).prompt_json("list").await.expect("JSON"),
        "{}"
    );
    let sent = body(
        &request
            .recv_timeout(Duration::from_secs(2))
            .expect("request"),
    );
    assert_eq!(sent["response_format"]["type"], "json_object");
    assert_eq!(sent["messages"][0]["role"], "system");
    assert!(
        sent["messages"][0]["content"]
            .as_str()
            .is_some_and(|text| text.contains("JSON"))
    );
    assert_eq!(sent["messages"][1]["content"], "list");
}

#[tokio::test]
async fn deepseek_structured_output_uses_its_responses_endpoint() {
    let (url, request) = serve_once(
        r#"{"output":[{"type":"message","content":[{"type":"output_text","text":"{\"ok\":true}"}]}]}"#,
        None,
    );
    let answer = deepseek(&url)
        .structured_output("hello", &schema())
        .await
        .expect("structured output");
    assert_eq!(answer, r#"{"ok":true}"#);
    let request = request
        .recv_timeout(Duration::from_secs(2))
        .expect("request");
    assert!(request.starts_with("POST /responses HTTP/1.1\r\n"));
    let sent = body(&request);
    assert_eq!(sent["model"], "deepseek-flash");
    assert_eq!(sent["input"], "hello");
    assert_eq!(sent["text"]["format"]["type"], "json_schema");
    assert_eq!(sent["text"]["format"]["name"], "answer");
    assert_eq!(sent["text"]["format"]["schema"]["required"][0], "ok");

    let (url, _request) = serve_once(r#"{"output":[]}"#, None);
    assert!(matches!(
        deepseek(&url).structured_output("hello", &schema()).await,
        Err(AiError::ApiError(message)) if message.contains("no structured output")
    ));
}

#[test]
fn ollama_declares_its_service_and_capabilities() {
    let provider = OpenAiCompatibleProvider::ollama("127.0.0.1:11434", "llama3").expect("host");
    assert_eq!(provider.provider_name(), "Ollama");
    assert!(!provider.mode.is_mock());
    assert!(matches!(
        provider.authentication,
        CompatibleAuthentication::None
    ));
    let capabilities = provider.capabilities();
    assert!(capabilities.embeddings && capabilities.vision && capabilities.json_schema);
    assert!(capabilities.streaming && capabilities.explicit_cancellation);
    assert!(OpenAiCompatibleProvider::ollama("127.0.0.1", "").is_err());
}

#[tokio::test]
async fn ollama_offline_mode_covers_every_capability_without_http() {
    let provider = OpenAiCompatibleProvider::ollama(" mock_ollama", "mock-model").expect("mock");
    assert!(provider.mode.is_mock());
    assert!(provider.prompt("hello").await.is_ok());
    assert!(provider.prompt_with_image("hello", b"bytes").await.is_ok());
    assert!(provider.embed("hello").await.is_ok());
    assert!(provider.prompt_json("hello").await.is_ok());
    assert!(provider.structured_output("hello", &schema()).await.is_ok());
    let response = provider.prompt("alice@example.com").await.expect("masked");
    assert!(!response.contains("alice@example.com"));
}

#[tokio::test]
async fn ollama_uses_its_v1_api_without_a_credential() {
    let (url, request) = serve_once(
        r#"{"choices":[{"message":{"content":"ok"}}],"usage":{"prompt_tokens":3,"completion_tokens":4,"total_tokens":7}}"#,
        None,
    );
    // OLLAMA_HOST is commonly `127.0.0.1:11434`, without a scheme.
    let host = url.trim_start_matches("http://").trim_end_matches("/v1");
    let answer = OpenAiCompatibleProvider::ollama(host, "llama3")
        .expect("scheme-less host")
        .chat_with_usage(&[Message::user("hi")])
        .await
        .expect("Ollama answer");
    assert_eq!(
        answer.usage().and_then(|usage| usage.total_tokens()),
        Some(7)
    );
    let request = request
        .recv_timeout(Duration::from_secs(2))
        .expect("request");
    assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
    assert!(!request.to_ascii_lowercase().contains("authorization:"));

    let (url, request) = serve_once(r#"{"data":[{"embedding":[0.5,-1.0]}]}"#, None);
    let host = url.trim_end_matches("/v1");
    let embedding = OpenAiCompatibleProvider::ollama(host, "llama3")
        .expect("host")
        .embed("hello")
        .await
        .expect("embedding");
    assert_eq!(embedding, [0.5, -1.0]);
    let request = request
        .recv_timeout(Duration::from_secs(2))
        .expect("request");
    assert!(request.starts_with("POST /v1/embeddings HTTP/1.1\r\n"));
    assert_eq!(body(&request)["model"], "nomic-embed-text");

    let (url, request) = serve_once(
        r#"{"choices":[{"message":{"content":"{\"ok\":true}"}}]}"#,
        None,
    );
    let host = url.trim_end_matches("/v1");
    OpenAiCompatibleProvider::ollama(host, "llama3")
        .expect("host")
        .structured_output("hello", &schema())
        .await
        .expect("structured output");
    let sent = body(
        &request
            .recv_timeout(Duration::from_secs(2))
            .expect("request"),
    );
    assert_eq!(sent["response_format"]["type"], "json_schema");
    assert_eq!(
        sent["response_format"]["json_schema"]["schema"]["required"][0],
        "ok"
    );
}
