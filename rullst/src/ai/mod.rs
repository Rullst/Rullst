use async_trait::async_trait;
use std::sync::Arc;

pub mod providers;

#[derive(Debug)]
pub enum AiError {
    RequestError(reqwest::Error),
    SerializationError(serde_json::Error),
    ApiError(String),
    ConfigError(String),
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
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn prompt(&self, text: &str) -> Result<String, AiError>;
    async fn chat(&self, messages: &[Message]) -> Result<String, AiError>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError>;
}

pub struct ChatBuilder {
    provider: Arc<dyn AiProvider>,
    messages: Vec<Message>,
}

impl ChatBuilder {
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            messages: Vec::new(),
        }
    }

    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    pub async fn send(self) -> Result<String, AiError> {
        self.provider.chat(&self.messages).await
    }
}

#[derive(Clone)]
pub struct AiClient {
    provider: Arc<dyn AiProvider>,
}

impl AiClient {
    pub fn new(provider: impl AiProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    pub fn auto() -> Result<Self, AiError> {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            Ok(Self::new(providers::openai::OpenAiProvider::new(key)))
        } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
            Ok(Self::new(providers::gemini::GeminiProvider::new(key)))
        } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            Ok(Self::new(providers::anthropic::AnthropicProvider::new(key)))
        } else if let Ok(host) = std::env::var("OLLAMA_HOST") {
            let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3".to_string());
            Ok(Self::new(providers::ollama::OllamaProvider::new(host, model)))
        } else {
            // Default check fallback if nothing is present, but try Ollama at localhost
            Ok(Self::new(providers::ollama::OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string())))
        }
    }

    pub async fn prompt(&self, text: &str) -> Result<String, AiError> {
        self.provider.prompt(text).await
    }

    pub fn chat(&self) -> ChatBuilder {
        ChatBuilder::new(self.provider.clone())
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        self.provider.embed(text).await
    }

    pub async fn structured_prompt<T>(&self, text: &str) -> Result<T, AiError>
    where
        T: serde::de::DeserializeOwned,
    {
        let system_instruction = "Return ONLY valid JSON. Do not write markdown, code blocks, or explanations.";
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
pub struct VectorDocument {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
}

pub struct VectorIndex {
    documents: HashMap<String, VectorDocument>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn add(&mut self, id: impl Into<String>, vector: Vec<f32>, payload: serde_json::Value) {
        let id_str = id.into();
        self.documents.insert(id_str.clone(), VectorDocument {
            id: id_str,
            vector,
            payload,
        });
    }

    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(f32, &VectorDocument)> {
        if query_vector.is_empty() || self.documents.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<(f32, &VectorDocument)> = self.documents
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
