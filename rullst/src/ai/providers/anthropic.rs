use crate::ai::{AiError, AiProvider, Message};
use async_trait::async_trait;

/// Anthropic Claude API provider implementation.
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Creates a new `AnthropicProvider` with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "claude-3-5-sonnet-latest".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Sets a custom generation model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Builds the JSON payload for a chat completion request to the Anthropic API.
    pub fn build_chat_payload(&self, messages: &[Message]) -> serde_json::Value {
        let mut system_text = None;
        let mut chat_messages = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                system_text = Some(msg.content.clone());
            } else {
                let role = match msg.role.as_str() {
                    "assistant" => "assistant",
                    _ => "user",
                };
                chat_messages.push(serde_json::json!({
                    "role": role,
                    "content": msg.content,
                }));
            }
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": chat_messages,
        });

        if let Some(sys_prompt) = system_text
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("system".to_string(), serde_json::json!(sys_prompt));
        }
        body
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        let messages = vec![Message::user(text)];
        self.chat(&messages).await
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        let url = "https://api.anthropic.com/v1/messages";

        let body = self.build_chat_payload(messages);

        let res = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!(
                "Anthropic error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let content = json["content"][0]["text"].as_str().ok_or_else(|| {
            AiError::ApiError("No text returned from Anthropic response".to_string())
        })?;

        Ok(content.to_string())
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        Err(AiError::Other(
            "Anthropic does not support native text embeddings. Please use OpenAiProvider, GeminiProvider, or OllamaProvider for embeddings.".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_anthropic_provider_builder() {
        let provider = AnthropicProvider::new("test-key").with_model("claude-test");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-test");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_anthropic_build_chat_payload() {
        let provider = AnthropicProvider::new("test-key");
        let msgs = vec![
            Message::system("You are a helpful AI"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];
        let payload = provider.build_chat_payload(&msgs);

        assert_eq!(payload["system"], "You are a helpful AI");

        let messages = payload["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "assistant"); // Kills match arm mutant
    }
    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_anthropic_embed_error() {
        let provider = AnthropicProvider::new("test-key");
        let result = provider.embed("hello").await;
        assert!(result.is_err());
    }
}
