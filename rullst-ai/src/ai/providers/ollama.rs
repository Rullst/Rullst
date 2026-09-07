use super::support::{DEFAULT_REQUEST_TIMEOUT, embedding_values, endpoint, success_response};
use super::support::{http_client, read_json};
use crate::ai::{
    AiError, AiGuardrails, AiProvider, JsonCapability, Message, ProviderCapabilities,
    StructuredOutputSchema,
    guardrails::prepare_messages,
    mock::{self, ProviderMode},
};
use async_trait::async_trait;
use base64::Engine;
use std::time::Duration;

/// Ollama local provider. Empty and `mock_*` hosts select deterministic offline mode.
pub struct OllamaProvider {
    host: String,
    model: String,
    embedding_model: String,
    mode: ProviderMode,
    request_timeout: Duration,
}

impl OllamaProvider {
    /// Creates an Ollama provider from a host and generation model.
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        let host = host.into();
        let mode = ProviderMode::from_credential(&host);
        Self {
            host: host.trim_end_matches('/').to_string(),
            model: model.into(),
            embedding_model: "nomic-embed-text".to_string(),
            mode,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }

    /// Sets a custom embedding model name.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }

    /// Sets the deadline applied to every live Ollama transport request.
    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    async fn send_chat_body(&self, body: serde_json::Value) -> Result<String, AiError> {
        let response = http_client()?
            .post(endpoint(&self.host, "api/chat"))
            .timeout(self.request_timeout)
            .json(&body)
            .send()
            .await
            .map_err(|error| AiError::RequestError(error.without_url()))?;
        let response = success_response(response, self.provider_name()).await?;
        let json = read_json(response, self.provider_name()).await?;
        json["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| AiError::ApiError("Ollama returned no chat content".to_string()))
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn provider_name(&self) -> &'static str {
        "Ollama"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text: true,
            chat: true,
            embeddings: true,
            vision: true,
            json: JsonCapability::NativeMode,
            json_schema: true,
            streaming: false,
            tools: false,
            request_timeout: true,
            retries: false,
            explicit_cancellation: false,
        }
    }

    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        self.chat(&[Message::user(text)]).await
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        let messages = prepare_messages(messages)?;
        if self.mode.is_mock() {
            return Ok(mock::chat_response(
                self.provider_name(),
                &self.model,
                &messages,
            ));
        }
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        }))
        .await
    }

    async fn prompt_with_image(&self, text: &str, image_bytes: &[u8]) -> Result<String, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if self.mode.is_mock() {
            return Ok(mock::vision_response(
                self.provider_name(),
                &self.model,
                &text,
                image_bytes,
            ));
        }
        let image = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": text, "images": [image]}],
            "stream": false,
        }))
        .await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if self.mode.is_mock() {
            return Ok(mock::embedding(&text));
        }
        let response = http_client()?
            .post(endpoint(&self.host, "api/embed"))
            .timeout(self.request_timeout)
            .json(&serde_json::json!({
                "model": self.embedding_model,
                "input": text,
            }))
            .send()
            .await
            .map_err(|error| AiError::RequestError(error.without_url()))?;
        let response = success_response(response, self.provider_name()).await?;
        let json = read_json(response, self.provider_name()).await?;
        embedding_values(json["embeddings"][0].as_array(), self.provider_name())
    }

    async fn prompt_json(&self, text: &str) -> Result<String, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if self.mode.is_mock() {
            return Ok(mock::json_response(
                self.provider_name(),
                &self.model,
                &text,
            ));
        }
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": format!(
                "Return only valid JSON for this input:\n{text}"
            )}],
            "format": "json",
            "stream": false,
        }))
        .await
    }

    async fn structured_output(
        &self,
        text: &str,
        schema: &StructuredOutputSchema,
    ) -> Result<String, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if self.mode.is_mock() {
            return mock::structured_response(schema);
        }
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": format!(
                "Respond according to this JSON Schema:\n{}\n\nInput:\n{text}",
                schema.schema()
            )}],
            "format": schema.schema(),
            "stream": false,
        }))
        .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_a_live_host() {
        let provider = OllamaProvider::new("http://localhost:11434/", "llama-test")
            .with_embedding_model("nomic-test");
        assert_eq!(provider.host, "http://localhost:11434");
        assert_eq!(provider.embedding_model, "nomic-test");
    }

    #[tokio::test]
    async fn mock_mode_covers_every_capability_without_http() {
        let provider = OllamaProvider::new("mock_offline", "mock-model");
        assert!(provider.prompt("hello").await.is_ok());
        assert!(provider.chat(&[Message::user("hello")]).await.is_ok());
        assert!(provider.prompt_with_image("hello", b"bytes").await.is_ok());
        assert!(provider.embed("hello").await.is_ok());
        assert!(provider.prompt_json("hello").await.is_ok());
        let schema = StructuredOutputSchema::new(
            "answer",
            serde_json::json!({
                "type": "object",
                "properties": {"ok": {"type": "boolean"}},
                "required": ["ok"]
            }),
        )
        .unwrap();
        assert!(provider.structured_output("hello", &schema).await.is_ok());
    }

    #[tokio::test]
    async fn direct_calls_apply_guardrails() {
        let provider = OllamaProvider::new("", "mock-model");
        assert!(matches!(
            provider.prompt_with_image("<|im_start|>", b"bytes").await,
            Err(AiError::BlockedByFirewall(_))
        ));
        let response = provider.prompt("alice@example.com").await.unwrap();
        assert!(!response.contains("alice@example.com"));
    }
}
