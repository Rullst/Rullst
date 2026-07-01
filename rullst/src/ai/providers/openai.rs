use crate::ai::{AiError, AiProvider, Message};
use async_trait::async_trait;

/// OpenAI API provider implementation.
pub struct OpenAiProvider {
    api_key: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Creates a new `OpenAiProvider` with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gpt-4o-mini".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Sets a custom generation model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Sets a custom text embedding model name.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        let messages = vec![Message::user(text)];
        self.chat(&messages).await
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        let url = "https://api.openai.com/v1/chat/completions";

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!(
                "OpenAI error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                AiError::ApiError("No content returned from OpenAI chat response".to_string())
            })?;

        Ok(content.to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let url = "https://api.openai.com/v1/embeddings";

        let body = serde_json::json!({
            "model": self.embedding_model,
            "input": text,
        });

        let res = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!(
                "OpenAI error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| {
                AiError::ApiError("No embedding returned from OpenAI response".to_string())
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_openai_provider_builder() {
        let provider = OpenAiProvider::new("test-key")
            .with_model("gpt-4")
            .with_embedding_model("text-emb");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "gpt-4");
        assert_eq!(provider.embedding_model, "text-emb");
    }
}


