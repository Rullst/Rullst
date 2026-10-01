//! Gemini streaming through `streamGenerateContent?alt=sse` (v13).
//!
//! Each event's data is one `GenerateContentResponse`
//! (<https://ai.google.dev/api/generate-content#method:-models.streamgeneratecontent>).
//! Text comes from the first candidate's non-thought parts, `usageMetadata`
//! carries running totals and the final chunk sets `finishReason`. The
//! stream has no terminal sentinel, so a body that ends before any finish
//! reason is reported as truncated. Truncated or withheld replies fail like
//! the non-streaming path.

use super::super::sse::{self, Flow};
use super::super::support::{MAX_RESPONSE_BYTES, endpoint, http_client};
use super::{GeminiProvider, reject_incomplete};
use crate::ai::{
    AiCancellation, AiError, AiProvider, AiStreamSink, Message, StreamLimits, StreamingAiProvider,
    TokenUsage, guardrails::prepare_messages, mock,
};
use async_trait::async_trait;

/// A short status or reason token from the provider, or `unknown`.
fn token(value: &serde_json::Value) -> &str {
    value
        .as_str()
        .filter(|text| {
            !text.is_empty()
                && text.len() <= 64
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
        .unwrap_or("unknown")
}

#[async_trait]
impl StreamingAiProvider for GeminiProvider {
    async fn stream_chat<S>(
        &self,
        messages: &[Message],
        _limits: StreamLimits,
        cancellation: &AiCancellation,
        sink: &mut S,
    ) -> Result<(), AiError>
    where
        S: AiStreamSink,
    {
        let messages = prepare_messages(messages)?;
        if cancellation.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        if self.mode.is_mock() {
            let response = mock::chat_response(self.provider_name(), &self.model, &messages);
            return sink.send(&response);
        }
        let path = format!("models/{}:streamGenerateContent?alt=sse", self.model);
        let request = http_client()?
            .post(endpoint(&self.base_url, &path))
            .timeout(self.request_timeout)
            .header("x-goog-api-key", &self.api_key)
            .json(&Self::build_chat_payload(&messages));
        let mut usage: Option<TokenUsage> = None;
        let mut finished = false;
        sse::read_stream(
            request,
            self.provider_name(),
            MAX_RESPONSE_BYTES,
            cancellation,
            |frame| {
                let chunk: serde_json::Value = serde_json::from_str(&frame.data).map_err(|_| {
                    AiError::StreamProtocol("server-sent event data is not valid JSON")
                })?;
                if chunk.get("error").is_some() {
                    return Err(AiError::ApiError(format!(
                        "Gemini stream error: {}",
                        token(&chunk["error"]["status"])
                    )));
                }
                if chunk["promptFeedback"]["blockReason"].is_string() {
                    return Err(AiError::ApiError(format!(
                        "Gemini blocked the prompt (blockReason {})",
                        token(&chunk["promptFeedback"]["blockReason"])
                    )));
                }
                if let Some(reported) = TokenUsage::from_gemini(&chunk["usageMetadata"]) {
                    usage = Some(usage.map_or(reported, |usage| usage.updated_by(reported)));
                }
                let candidate = &chunk["candidates"][0];
                for part in candidate["content"]["parts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    if part["thought"] == true {
                        continue;
                    }
                    if let Some(text) = part["text"].as_str()
                        && !text.is_empty()
                    {
                        sink.send(text)?;
                    }
                }
                if candidate["finishReason"].is_string() {
                    reject_incomplete(candidate)?;
                    finished = true;
                }
                Ok(Flow::Continue)
            },
        )
        .await?;
        if !finished {
            return Err(AiError::StreamProtocol(
                "stream ended without a finish reason",
            ));
        }
        if let Some(usage) = usage {
            sink.usage(usage);
        }
        Ok(())
    }
}
