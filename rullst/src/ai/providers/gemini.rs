use crate::ai::{AiError, AiProvider, Message};
use async_trait::async_trait;

/// [TODO] Missing documentation.
pub struct GeminiProvider {
    api_key: String,
    model: String,
    embedding_model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// [TODO] Missing documentation.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "gemini-1.5-flash".to_string(),
            embedding_model: "text-embedding-004".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// [TODO] Missing documentation.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// [TODO] Missing documentation.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
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
