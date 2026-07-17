use crate::ai::{AiError, AiProvider, Message};
use async_trait::async_trait;

/// Ollama API provider implementation.
pub struct OllamaProvider {
    host: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Creates a new `OllamaProvider` with the host endpoint and model.
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        let host_str = host.into();
        // Remove trailing slash if present
        let host_clean = host_str.trim_end_matches('/').to_string();

        Self {
            host: host_clean,
            model: model.into(),
            embedding_model: "nomic-embed-text".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Sets a custom text embedding model name.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        let messages = vec![Message::user(text)];
        self.chat(&messages).await
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        let url = format!("{}/api/chat", self.host);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!(
                "Ollama error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let content = json["message"]["content"].as_str().ok_or_else(|| {
            AiError::ApiError("No content returned from Ollama chat response".to_string())
        })?;

        Ok(content.to_string())
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let url = format!("{}/api/embeddings", self.host);

        let body = serde_json::json!({
            "model": self.embedding_model,
            "prompt": text,
        });

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            return Err(AiError::ApiError(format!(
                "Ollama error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let embedding = json["embedding"]
            .as_array()
            .ok_or_else(|| {
                AiError::ApiError("No embedding returned from Ollama response".to_string())
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
    fn test_ollama_provider_builder() {
        let provider = OllamaProvider::new("http://localhost:11434/", "llama-test")
            .with_embedding_model("nomic-test");
        assert_eq!(provider.host, "http://localhost:11434");
        assert_eq!(provider.model, "llama-test");
        assert_eq!(provider.embedding_model, "nomic-test");
    }
}
