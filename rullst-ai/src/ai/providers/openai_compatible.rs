//! Capability-declared transport for OpenAI-compatible local and cloud APIs.

use super::support::{DEFAULT_REQUEST_TIMEOUT, embedding_values, endpoint, image_mime_type};
use crate::ai::{
    AiError, AiGuardrails, AiProvider, ChatCompletion, Message, ProviderCapabilities,
    StructuredOutputSchema, TokenUsage,
    guardrails::prepare_messages,
    mock::{self, ProviderMode},
};
use async_trait::async_trait;
use base64::Engine;
use futures_util::StreamExt;
use std::{fmt, time::Duration};
use zeroize::Zeroizing;

mod capabilities;
mod config;
mod stream;
pub use capabilities::OpenAiCompatibleCapabilities;
use config::{EndpointScope, validate_api_key, validate_base_url, validate_model};

const MAX_IMAGE_BYTES: usize = 10 * 1_024 * 1_024;
const MAX_RESPONSE_BYTES: usize = 2 * 1_024 * 1_024;

enum CompatibleAuthentication {
    None,
    Bearer(Zeroizing<String>),
}

/// Explicit adapter for servers implementing the bounded OpenAI-compatible
/// request/response shapes documented by Rullst.
///
/// It is not an arbitrary-HTTP adapter. APIs with different authentication,
/// paths, streaming, message, tool, or response semantics should implement
/// [`AiProvider`] directly.
pub struct OpenAiCompatibleProvider {
    base_url: String,
    model: String,
    embedding_model: String,
    authentication: CompatibleAuthentication,
    mode: ProviderMode,
    capabilities: OpenAiCompatibleCapabilities,
    client: reqwest::Client,
    request_timeout: Duration,
}

impl OpenAiCompatibleProvider {
    /// Creates a live unauthenticated provider for an exact loopback endpoint.
    ///
    /// Plain HTTP is accepted only for a literal loopback IP. Rullst
    /// never probes or starts a local model server implicitly.
    pub fn try_local(
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, AiError> {
        Self::try_build(
            base_url.into(),
            model.into(),
            CompatibleAuthentication::None,
            ProviderMode::Live,
            EndpointScope::Loopback,
        )
    }

    /// Creates a Bearer-authenticated provider for an exact loopback endpoint.
    ///
    /// Empty and `mock_*` keys select deterministic offline mode. Use
    /// [`Self::try_local`] when the local server requires no credential.
    pub fn try_local_with_bearer(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, AiError> {
        let api_key = api_key.into();
        validate_api_key(&api_key)?;
        let mode = ProviderMode::from_credential(&api_key);
        Self::try_build(
            base_url.into(),
            model.into(),
            CompatibleAuthentication::Bearer(Zeroizing::new(api_key)),
            mode,
            EndpointScope::Loopback,
        )
    }

    /// Creates a cloud provider using HTTPS and Bearer authentication.
    ///
    /// Empty and `mock_*` API keys select deterministic offline mode without
    /// contacting the configured endpoint.
    pub fn try_cloud(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, AiError> {
        let api_key = api_key.into();
        validate_api_key(&api_key)?;
        let mode = ProviderMode::from_credential(&api_key);
        Self::try_build(
            base_url.into(),
            model.into(),
            CompatibleAuthentication::Bearer(Zeroizing::new(api_key)),
            mode,
            EndpointScope::Cloud,
        )
    }

    /// Creates an explicit deterministic offline fixture.
    pub fn mock(model: impl Into<String>) -> Result<Self, AiError> {
        Self::try_build(
            "https://offline.invalid/v1".to_string(),
            model.into(),
            CompatibleAuthentication::Bearer(Zeroizing::new("mock_compatible".to_string())),
            ProviderMode::Mock,
            EndpointScope::Cloud,
        )
    }

    /// Declares the optional request shapes supported by this exact endpoint
    /// and model configuration.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: OpenAiCompatibleCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Selects the model sent to the `/embeddings` endpoint.
    pub fn try_with_embedding_model(mut self, model: impl Into<String>) -> Result<Self, AiError> {
        let model = model.into();
        validate_model("embedding model", &model)?;
        self.embedding_model = model;
        Ok(self)
    }

    /// Sets the deadline applied to every live transport request.
    #[must_use]
    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    fn try_build(
        base_url: String,
        model: String,
        authentication: CompatibleAuthentication,
        mode: ProviderMode,
        scope: EndpointScope,
    ) -> Result<Self, AiError> {
        validate_model("generation model", &model)?;
        let base_url = validate_base_url(base_url, scope)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()?;
        Ok(Self {
            base_url,
            embedding_model: model.clone(),
            model,
            authentication,
            mode,
            capabilities: OpenAiCompatibleCapabilities::chat_only(),
            client,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        })
    }

    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self
            .client
            .post(endpoint(&self.base_url, path))
            .timeout(self.request_timeout);
        match &self.authentication {
            CompatibleAuthentication::None => request,
            CompatibleAuthentication::Bearer(api_key) => request.bearer_auth(api_key.as_str()),
        }
    }

    async fn request_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AiError> {
        let response = self
            .request(path)
            .json(&body)
            .send()
            .await
            .map_err(redacted_transport_error)?;
        if !response.status().is_success() {
            return Err(AiError::ApiError(format!(
                "{} returned HTTP {}",
                self.provider_name(),
                response.status()
            )));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(response_size_error());
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(redacted_transport_error)?;
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(response_size_error());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(AiError::from)
    }

    async fn send_chat_body(&self, body: serde_json::Value) -> Result<String, AiError> {
        self.send_chat_completion(body)
            .await
            .map(ChatCompletion::into_text)
    }

    /// The answer plus the `usage` object a compatible server may send.
    async fn send_chat_completion(
        &self,
        body: serde_json::Value,
    ) -> Result<ChatCompletion, AiError> {
        let json = self.request_json("chat/completions", body).await?;
        let text = super::support::openai_chat_content(&json, self.provider_name())?;
        Ok(ChatCompletion::new(
            text,
            TokenUsage::from_openai(&json["usage"]),
        ))
    }

    fn unsupported(&self, capability: &'static str) -> AiError {
        AiError::UnsupportedCapability {
            provider: self.provider_name(),
            capability,
        }
    }
}

impl fmt::Debug for OpenAiCompatibleProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiCompatibleProvider")
            .field("base_url", &"[CONFIGURED]")
            .field("model", &self.model)
            .field("embedding_model", &self.embedding_model)
            .field("authentication", &"[REDACTED]")
            .field("mode", &self.mode)
            .field("capabilities", &self.capabilities)
            .field("request_timeout", &self.request_timeout)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn provider_name(&self) -> &'static str {
        "OpenAI-compatible"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities.provider_capabilities()
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

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if !self.capabilities.embeddings {
            return Err(self.unsupported("text embeddings"));
        }
        if self.mode.is_mock() {
            return Ok(mock::embedding(&text));
        }
        let json = self
            .request_json(
                "embeddings",
                serde_json::json!({
                    "model": self.embedding_model,
                    "input": text,
                }),
            )
            .await?;
        embedding_values(
            json["data"][0]["embedding"].as_array(),
            self.provider_name(),
        )
    }

    async fn prompt_with_image(&self, text: &str, image_bytes: &[u8]) -> Result<String, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if !self.capabilities.vision {
            return Err(self.unsupported("vision input"));
        }
        if image_bytes.len() > MAX_IMAGE_BYTES {
            return Err(AiError::ConfigError(
                "OpenAI-compatible vision input exceeds 10 MiB".to_string(),
            ));
        }
        if self.mode.is_mock() {
            return Ok(mock::vision_response(
                self.provider_name(),
                &self.model,
                &text,
                image_bytes,
            ));
        }
        let mime_type = image_mime_type(image_bytes)?;
        let image = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": text},
                    {"type": "image_url", "image_url": {
                        "url": format!("data:{mime_type};base64,{image}")
                    }}
                ]
            }],
            "stream": false,
        }))
        .await
    }

    async fn prompt_json(&self, text: &str) -> Result<String, AiError> {
        let text = AiGuardrails::prepare(text)?;
        if !self.capabilities.json_mode {
            return Err(self.unsupported("native JSON response mode"));
        }
        if self.mode.is_mock() {
            return Ok(mock::json_response(
                self.provider_name(),
                &self.model,
                &text,
            ));
        }
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": text}],
            "response_format": {"type": "json_object"},
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
        if !self.capabilities.json_schema {
            return Err(self.unsupported("native JSON Schema structured output"));
        }
        if self.mode.is_mock() {
            return mock::structured_response(schema);
        }
        self.send_chat_body(serde_json::json!({
            "model": self.model,
            "messages": [{"role": "user", "content": text}],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": schema.name(),
                    "description": schema.description(),
                    "schema": schema.schema(),
                    "strict": true
                }
            },
            "stream": false,
        }))
        .await
    }
}

fn response_size_error() -> AiError {
    AiError::ApiError(format!(
        "OpenAI-compatible response exceeds {MAX_RESPONSE_BYTES} bytes"
    ))
}

/// Drops the configured endpoint from a transport error, matching the native
/// providers and the redacted `Debug` output of this adapter.
fn redacted_transport_error(error: reqwest::Error) -> AiError {
    AiError::RequestError(error.without_url())
}

#[cfg(test)]
#[path = "openai_compatible_tests.rs"]
mod tests;
