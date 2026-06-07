use async_trait::async_trait;
use std::sync::Arc;

/// [TODO] Missing documentation.
pub mod providers;

#[derive(Debug)]
/// [TODO] Missing documentation.
pub enum AiError {
    /// [TODO] Missing documentation.
    RequestError(reqwest::Error),
    /// [TODO] Missing documentation.
    SerializationError(serde_json::Error),
    /// [TODO] Missing documentation.
    ApiError(String),
    /// [TODO] Missing documentation.
    ConfigError(String),
    /// [TODO] Missing documentation.
    Other(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::RequestError(err) => write!(f, "Request error: {}", err),
            AiError::SerializationError(err) => write!(f, "Serialization error: {}", err),
            AiError::ApiError(err) => write!(f, "API error: {}", err),
            AiError::ConfigError(err) => write!(f, "Configuration error: {}", err),
            AiError::Other(err) => write!(f, "Error: {}", err),
        }
    }
}

impl std::error::Error for AiError {}

impl From<reqwest::Error> for AiError {
    fn from(err: reqwest::Error) -> Self {
        AiError::RequestError(err)
    }
}

impl From<serde_json::Error> for AiError {
    fn from(err: serde_json::Error) -> Self {
        AiError::SerializationError(err)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// [TODO] Missing documentation.
pub struct Message {
    /// [TODO] Missing documentation.
    pub role: String,
    /// [TODO] Missing documentation.
    pub content: String,
}

impl Message {
    /// [TODO] Missing documentation.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// [TODO] Missing documentation.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// [TODO] Missing documentation.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

#[async_trait]
/// [TODO] Missing documentation.
pub trait AiProvider: Send + Sync {
    /// [TODO] Missing documentation.
    async fn prompt(&self, text: &str) -> Result<String, AiError>;
    /// [TODO] Missing documentation.
    async fn chat(&self, messages: &[Message]) -> Result<String, AiError>;
    /// [TODO] Missing documentation.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError>;
}

/// [TODO] Missing documentation.
pub struct ChatBuilder {
    provider: Arc<dyn AiProvider>,
    messages: Vec<Message>,
}

impl ChatBuilder {
    /// [TODO] Missing documentation.
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            messages: Vec::new(),
        }
    }

    /// [TODO] Missing documentation.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// [TODO] Missing documentation.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// [TODO] Missing documentation.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// [TODO] Missing documentation.
    pub async fn send(self) -> Result<String, AiError> {
        self.provider.chat(&self.messages).await
    }
}

#[derive(Clone)]
/// [TODO] Missing documentation.
pub struct AiClient {
    provider: Arc<dyn AiProvider>,
}

impl AiClient {
    /// [TODO] Missing documentation.
    pub fn new(provider: impl AiProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    /// [TODO] Missing documentation.
    pub fn auto() -> Result<Self, AiError> {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            Ok(Self::new(providers::openai::OpenAiProvider::new(key)))
        } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            Ok(Self::new(providers::gemini::GeminiProvider::new(key)))
        } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            Ok(Self::new(providers::anthropic::AnthropicProvider::new(key)))
        } else if let Ok(host) = std::env::var("OLLAMA_HOST") {
            let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_string());
            Ok(Self::new(providers::ollama::OllamaProvider::new(
                host, model,
            )))
        } else {
            // Default check fallback if nothing is present, but try Ollama at localhost
            Ok(Self::new(providers::ollama::OllamaProvider::new(
                "http://localhost:11434".to_string(),
                "llama3".to_string(),
            )))
        }
    }

    /// [TODO] Missing documentation.
    pub async fn prompt(&self, text: &str) -> Result<String, AiError> {
        self.provider.prompt(text).await
    }

    /// [TODO] Missing documentation.
    pub fn chat(&self) -> ChatBuilder {
        ChatBuilder::new(self.provider.clone())
    }

    /// [TODO] Missing documentation.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        self.provider.embed(text).await
    }

    /// [TODO] Missing documentation.
    pub async fn structured_prompt<T>(&self, text: &str) -> Result<T, AiError>
    where
        T: serde::de::DeserializeOwned,
    {
        let system_instruction =
            "Return ONLY valid JSON. Do not write markdown, code blocks, or explanations.";
        let full_prompt = format!("{}\n\nTarget data:\n{}", system_instruction, text);
        let res = self.provider.prompt(&full_prompt).await?;
        let clean_res = clean_json_markdown(&res);
        let parsed: T = serde_json::from_str(&clean_res)?;
        Ok(parsed)
    }
}

fn clean_json_markdown(s: &str) -> String {
    let mut s = s.trim().to_string();
    if s.starts_with("```") {
        if s.starts_with("```json") {
            s = s[7..].to_string();
        } else {
            s = s[3..].to_string();
        }
        if s.ends_with("```") {
            s = s[..s.len() - 3].to_string();
        }
    }
    s.trim().to_string()
}

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// [TODO] Missing documentation.
pub struct VectorDocument {
    /// [TODO] Missing documentation.
    pub id: String,
    /// [TODO] Missing documentation.
    pub vector: Vec<f32>,
    /// [TODO] Missing documentation.
    pub payload: serde_json::Value,
}

/// [TODO] Missing documentation.
pub struct VectorIndex {
    documents: HashMap<String, VectorDocument>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    /// [TODO] Missing documentation.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// [TODO] Missing documentation.
    pub fn add(&mut self, id: impl Into<String>, vector: Vec<f32>, payload: serde_json::Value) {
        let id_str = id.into();
        self.documents.insert(
            id_str.clone(),
            VectorDocument {
                id: id_str,
                vector,
                payload,
            },
        );
    }

    /// [TODO] Missing documentation.
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(f32, &VectorDocument)> {
        if query_vector.is_empty() || self.documents.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<(f32, &VectorDocument)> = self
            .documents
            .values()
            .map(|doc| {
                let similarity = cosine_similarity(query_vector, &doc.vector);
                (similarity, doc)
            })
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }
}

/// [TODO] Missing documentation.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (val_a, val_b) in a.iter().zip(b.iter()) {
        dot_product += val_a * val_b;
        norm_a += val_a * val_a;
        norm_b += val_b * val_b;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        let d = vec![-1.0, 0.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&a, &c).abs() < 1e-6);
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 1e-6);

        // Mismatched lengths
        assert_eq!(cosine_similarity(&a, &[1.0, 0.0]), 0.0);
        // Empty
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_vector_index() {
        let mut idx = VectorIndex::new();
        idx.add("doc1", vec![1.0, 0.0], serde_json::json!({"name": "doc1"}));
        idx.add("doc2", vec![0.0, 1.0], serde_json::json!({"name": "doc2"}));

        // Search with query vector [0.9, 0.1]
        let results = idx.search(&[0.9, 0.1], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "doc1");

        // Search with query vector [0.1, 0.9]
        let results2 = idx.search(&[0.1, 0.9], 1);
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].1.id, "doc2");
    }
}
