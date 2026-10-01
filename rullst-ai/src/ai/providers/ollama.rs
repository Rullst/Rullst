use super::support::{DEFAULT_REQUEST_TIMEOUT, embedding_values, endpoint, success_response};
use super::support::{http_client, read_json};
use crate::ai::{
    AiError, AiGuardrails, AiProvider, ChatCompletion, JsonCapability, Message,
    ProviderCapabilities, StructuredOutputSchema, TokenUsage,
    guardrails::prepare_messages,
    mock::{self, ProviderMode},
};
use async_trait::async_trait;
use base64::Engine;
use std::time::Duration;

/// Port Ollama listens on when a scheme-less host names none.
const DEFAULT_PORT: u16 = 11434;

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
    ///
    /// The host is read the way Ollama reads `OLLAMA_HOST`: a URL such as
    /// `http://127.0.0.1:11434` is used as written, while a scheme-less
    /// `127.0.0.1:11434`, `localhost` or `[::1]` uses `http` and port 11434
    /// unless it names a valid port.
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        let host = host.into();
        let mode = ProviderMode::from_credential(&host);
        Self {
            host: normalize_host(&host),
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
        self.send_chat_completion(body)
            .await
            .map(ChatCompletion::into_text)
    }

    /// The answer plus `prompt_eval_count`/`eval_count`
    /// (<https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion>).
    async fn send_chat_completion(
        &self,
        body: serde_json::Value,
    ) -> Result<ChatCompletion, AiError> {
        let response = http_client()?
            .post(endpoint(&self.host, "api/chat"))
            .timeout(self.request_timeout)
            .json(&body)
            .send()
            .await
            .map_err(|error| AiError::RequestError(error.without_url()))?;
        let response = success_response(response, self.provider_name()).await?;
        let json = read_json(response, self.provider_name()).await?;
        let text = json["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| AiError::ApiError("Ollama returned no chat content".to_string()))?;
        Ok(ChatCompletion::new(text, TokenUsage::from_ollama(&json)))
    }
}

/// Mirrors Ollama's `OLLAMA_HOST` parsing: surrounding whitespace and quotes
/// are ignored, a value with `scheme://` keeps its scheme and port, and a value
/// without one uses `http` and [`DEFAULT_PORT`] (`ollama.com` alone means
/// `https://ollama.com:443`). An optional path is kept as a prefix.
fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_matches(['"', '\'']);
    if host.contains("://") {
        return host.trim_end_matches('/').to_string();
    }
    if host == "ollama.com" {
        return "https://ollama.com:443".to_string();
    }
    let (authority, path) = host.split_once('/').unwrap_or((host, ""));
    let authority = with_port(authority);
    match path.trim_end_matches('/') {
        "" => format!("http://{authority}"),
        path => format!("http://{authority}/{path}"),
    }
}

/// `host:port`, using [`DEFAULT_PORT`] when the port is absent or invalid and
/// `127.0.0.1` for an empty host. An unbracketed IPv6 address is bracketed.
fn with_port(authority: &str) -> String {
    let (name, port) = match authority
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
    {
        Some((address, rest)) => (format!("[{address}]"), rest.strip_prefix(':')),
        None if authority.matches(':').count() > 1 => (format!("[{authority}]"), None),
        None => match authority.split_once(':') {
            Some((name, port)) => (name.to_string(), Some(port)),
            None => (authority.to_string(), None),
        },
    };
    let name = if name.is_empty() {
        "127.0.0.1".to_string()
    } else {
        name
    };
    let port = port
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    format!("{name}:{port}")
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
        self.chat_with_usage(messages)
            .await
            .map(ChatCompletion::into_text)
    }

    async fn chat_with_usage(&self, messages: &[Message]) -> Result<ChatCompletion, AiError> {
        let messages = prepare_messages(messages)?;
        if self.mode.is_mock() {
            let text = mock::chat_response(self.provider_name(), &self.model, &messages);
            return Ok(ChatCompletion::new(text, None));
        }
        self.send_chat_completion(serde_json::json!({
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

    #[test]
    fn reads_ollama_host_values_the_way_ollama_does() {
        for (input, expected) in [
            ("127.0.0.1:11434", "http://127.0.0.1:11434"),
            ("0.0.0.0:11434", "http://0.0.0.0:11434"),
            ("localhost", "http://localhost:11434"),
            ("localhost:8080/", "http://localhost:8080"),
            (
                "gpu-box.internal:11434/ollama",
                "http://gpu-box.internal:11434/ollama",
            ),
            ("[::1]:11434", "http://[::1]:11434"),
            ("[::1]", "http://[::1]:11434"),
            ("::1", "http://[::1]:11434"),
            (":11434", "http://127.0.0.1:11434"),
            ("localhost:not-a-port", "http://localhost:11434"),
            ("localhost:70000", "http://localhost:11434"),
            ("\"127.0.0.1:11434\"", "http://127.0.0.1:11434"),
            ("ollama.com", "https://ollama.com:443"),
            (" https://ollama.example/ ", "https://ollama.example"),
            ("http://localhost:11434/", "http://localhost:11434"),
        ] {
            assert_eq!(
                OllamaProvider::new(input, "llama-test").host,
                expected,
                "input: {input:?}"
            );
        }
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
