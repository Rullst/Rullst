use crate::ai::{AiError, AiProvider, Message};
use async_trait::async_trait;

/// Google Gemini API provider implementation.
pub struct GeminiProvider {
    api_key: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// Creates a new `GeminiProvider` with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gemini-1.5-flash".to_string(),
            embedding_model: "text-embedding-004".to_string(),
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
    /// Builds the JSON payload for a chat completion request to the Gemini API.
    pub fn build_chat_payload(messages: &[Message]) -> serde_json::Value {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in messages {
            if msg.role == "system" {
                system_instruction = Some(serde_json::json!({
                    "parts": [{"text": msg.content}]
                }));
            } else {
                let role = match msg.role.as_str() {
                    "assistant" => "model",
                    other => other,
                };
                contents.push(serde_json::json!({
                    "role": role,
                    "parts": [{"text": msg.content}]
                }));
            }
        }

        let mut body = serde_json::json!({
            "contents": contents,
        });

        if let Some(sys_inst) = system_instruction
            && let Some(obj) = body.as_object_mut()
        {
            obj.insert("systemInstruction".to_string(), sys_inst);
        }
        body
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        let messages = vec![Message::user(text)];
        self.chat(&messages).await
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = Self::build_chat_payload(messages);

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
                "Gemini error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let content = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                AiError::ApiError("No text returned in Gemini candidate content".to_string())
            })?;

        Ok(content.to_string())
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            self.embedding_model, self.api_key
        );

        let body = serde_json::json!({
            "model": format!("models/{}", self.embedding_model),
            "content": {
                "parts": [{"text": text}]
            }
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
                "Gemini error status {}: {}",
                status, err_text
            )));
        }

        let json: serde_json::Value = res.json().await?;
        let embedding = json["embedding"]["values"]
            .as_array()
            .ok_or_else(|| {
                AiError::ApiError("No embedding returned from Gemini response".to_string())
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_gemini_provider_builder() {
        let provider = GeminiProvider::new("test-key")
            .with_model("gemini-test")
            .with_embedding_model("text-emb");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "gemini-test");
        assert_eq!(provider.embedding_model, "text-emb");
    }

    #[test]
    fn test_gemini_build_chat_payload() {
        let msgs = vec![
            Message::system("You are a helpful AI"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];
        let payload = GeminiProvider::build_chat_payload(&msgs);

        let sys = payload.get("systemInstruction").unwrap();
        assert_eq!(sys["parts"][0]["text"], "You are a helpful AI");

        let contents = payload.get("contents").unwrap().as_array().unwrap();
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model"); // kills assistant match arm mutant
    }
}
