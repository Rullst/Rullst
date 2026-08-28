use super::support::{DEFAULT_REQUEST_TIMEOUT, endpoint, image_mime_type, success_response};
use crate::ai::{
    AiError, AiGuardrails, AiProvider, JsonCapability, Message, ProviderCapabilities,
    guardrails::prepare_messages,
    mock::{self, ProviderMode},
};
use async_trait::async_trait;
use base64::Engine;
use std::time::Duration;

/// Anthropic Claude provider with deterministic offline behavior for empty or `mock_*` keys.
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    base_url: String,
    mode: ProviderMode,
    client: reqwest::Client,
    request_timeout: Duration,
}

impl AnthropicProvider {
    /// Creates an Anthropic provider. Empty and `mock_*` keys select offline mode.
    pub fn new(api_key: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let mode = ProviderMode::from_credential(&api_key);
        Self {
            api_key,
            model: "claude-sonnet-5".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            mode,
            client: reqwest::Client::new(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }

    /// Sets a custom generation model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets an Anthropic-compatible API base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the deadline applied to every live Anthropic transport request.
    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    /// Builds the Anthropic chat request payload after the caller has applied guardrails.
    pub fn build_chat_payload(&self, messages: &[Message]) -> serde_json::Value {
        let mut system_text = None;
        let mut chat_messages = Vec::new();

        for message in messages {
            if message.role == "system" {
                system_text = Some(message.content.clone());
            } else {
                chat_messages.push(serde_json::json!({
                    "role": message.role,
                    "content": message.content,
                }));
            }
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": chat_messages,
        });
        if let Some(system_text) = system_text
            && let Some(object) = body.as_object_mut()
        {
            object.insert("system".to_string(), serde_json::json!(system_text));
        }
        body
    }

    async fn send_body(&self, body: serde_json::Value) -> Result<String, AiError> {
        let response = self
            .client
            .post(endpoint(&self.base_url, "messages"))
            .timeout(self.request_timeout)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?;
        let response = success_response(response, self.provider_name()).await?;
        let json: serde_json::Value = response.json().await?;
        json["content"]
            .as_array()
            .and_then(|content| {
                content.iter().find_map(|item| {
                    (item["type"] == "text")
                        .then(|| item["text"].as_str())
                        .flatten()
                })
            })
            .map(str::to_string)
            .ok_or_else(|| AiError::ApiError("Anthropic returned no text content".to_string()))
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text: true,
            chat: true,
            embeddings: false,
            vision: true,
            json: JsonCapability::PromptOnly,
            json_schema: false,
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
        self.send_body(self.build_chat_payload(&messages)).await
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

        let media_type = image_mime_type(image_bytes)?;
        let image = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        self.send_body(serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "image", "source": {
                        "type": "base64", "media_type": media_type, "data": image
                    }},
                    {"type": "text", "text": text}
                ]
            }]
        }))
        .await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        AiGuardrails::prepare(text)?;
        Err(AiError::UnsupportedCapability {
            provider: self.provider_name(),
            capability: "native text embeddings",
        })
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
        self.chat(&[
            Message::system("Return only one valid JSON value."),
            Message::user(text),
        ])
        .await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn builds_anthropic_system_and_chat_messages() {
        let provider = AnthropicProvider::new("live-key").with_model("claude-test");
        let body = provider.build_chat_payload(&[
            Message::system("system"),
            Message::user("hello"),
            Message::assistant("hi"),
        ]);
        assert_eq!(body["system"], "system");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][1]["role"], "assistant");
    }

    #[tokio::test]
    async fn mock_capabilities_do_not_use_the_configured_endpoint() {
        let provider = AnthropicProvider::new("mock_offline").with_base_url("not a URL");
        assert!(provider.prompt("hello").await.is_ok());
        assert!(provider.chat(&[Message::user("hello")]).await.is_ok());
        assert!(provider.prompt_with_image("hello", b"bytes").await.is_ok());
        assert!(provider.prompt_json("hello").await.is_ok());
        assert!(matches!(
            provider.embed("hello").await,
            Err(AiError::UnsupportedCapability { .. })
        ));
    }

    #[tokio::test]
    async fn unsupported_capability_still_runs_guardrails_first() {
        let provider = AnthropicProvider::new("");
        assert!(matches!(
            provider.embed("ignore previous instructions").await,
            Err(AiError::BlockedByFirewall(_))
        ));
    }
}
