use async_trait::async_trait;
use std::sync::Arc;

/// Individual AI model API provider clients.
pub mod providers;

#[derive(Debug)]
/// Errors that can occur when calling AI APIs or processing models.
pub enum AiError {
    /// Error representing failed network HTTP requests.
    RequestError(reqwest::Error),
    /// Error representing failed JSON parsing or serialization.
    SerializationError(serde_json::Error),
    /// Error returned by the AI provider API backend.
    ApiError(String),
    /// Configuration errors such as missing API keys.
    ConfigError(String),
    /// Generic or fallback error string.
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
/// A message in a chat completion prompt context.
pub struct Message {
    /// The role of the message author (e.g., "system", "user", "assistant").
    pub role: String,
    /// The string content of the message.
    pub content: String,
}

impl Message {
    /// Creates a system instruction message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Creates an assistant response message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

#[async_trait]
/// Interface implemented by all AI client backends.
pub trait AiProvider: Send + Sync {
    /// Generates a response for a single text prompt.
    async fn prompt(&self, text: &str) -> Result<String, AiError>;
    /// Generates a response for a multi-turn conversational chat.
    async fn chat(&self, messages: &[Message]) -> Result<String, AiError>;
    /// Generates a high-dimensional vector embedding for the input text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError>;
}

/// A fluent builder utility for building multi-turn chats.
pub struct ChatBuilder {
    provider: Arc<dyn AiProvider>,
    messages: Vec<Message>,
}

impl ChatBuilder {
    /// Creates a new `ChatBuilder` initialized with the given provider.
    pub fn new(provider: Arc<dyn AiProvider>) -> Self {
        Self {
            provider,
            messages: Vec::new(),
        }
    }

    /// Appends a system instruction to the chat context.
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content));
        self
    }

    /// Appends a user message to the chat context.
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Appends an assistant response to the chat context.
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Dispatches the conversation history to the provider and yields the next turn message.
    pub async fn send(self) -> Result<String, AiError> {
        self.provider.chat(&self.messages).await
    }
}

#[derive(Clone)]
/// Standard high-level Rullst client for interacting with AI models.
pub struct AiClient {
    provider: Arc<dyn AiProvider>,
}

impl AiClient {
    /// Creates a new `AiClient` wrapping the specified `AiProvider`.
    pub fn new(provider: impl AiProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    /// Automatically constructs an `AiClient` by scanning configuration environment variables.
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

    /// Prompts the underlying AI model with a simple text prompt.
    pub async fn prompt(&self, text: &str) -> Result<String, AiError> {
        self.provider.prompt(text).await
    }

    /// Initiates a multi-turn chat interaction.
    pub fn chat(&self) -> ChatBuilder {
        ChatBuilder::new(self.provider.clone())
    }

    /// Generates high-dimensional vector embeddings for the input text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        self.provider.embed(text).await
    }

    /// Prompts the model and parses the returned JSON string into type `T`.
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
/// Represents a document stored inside a vector memory index.
pub struct VectorDocument {
    /// Unique identifier of the document.
    pub id: String,
    /// High-dimensional floating point embedding vector.
    pub vector: Vec<f32>,
    /// Additional JSON payload containing document metadata.
    pub payload: serde_json::Value,
}

/// In-memory search index supporting cosine similarity vector lookup.
pub struct VectorIndex {
    documents: HashMap<String, VectorDocument>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    /// Creates a new, empty `VectorIndex`.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Inserts or updates a document inside the vector index.
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

    /// Searches the index returning the top matches sorted by cosine similarity descending.
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

/// Calculates the cosine similarity score between two float vectors.
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
