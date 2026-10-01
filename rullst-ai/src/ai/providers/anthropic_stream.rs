//! Anthropic Messages streaming (v13).
//!
//! Event flow (<https://docs.anthropic.com/en/docs/build-with-claude/streaming>):
//! `message_start` (with initial `usage`), `content_block_start`,
//! `content_block_delta` (`text_delta` carries text), `content_block_stop`,
//! `message_delta` (`stop_reason` and cumulative `usage`), `message_stop`,
//! plus `ping` and `error` events. Unknown event types are ignored, as the
//! documentation asks. A truncated or refused reply fails like the
//! non-streaming path.

use super::super::sse::{self, Flow};
use super::super::support::{MAX_RESPONSE_BYTES, endpoint, http_client};
use super::{AnthropicProvider, reject_incomplete};
use crate::ai::{
    AiCancellation, AiError, AiProvider, AiStreamSink, Message, StreamLimits, StreamingAiProvider,
    TokenUsage, guardrails::prepare_messages, mock,
};
use async_trait::async_trait;

fn merged(current: Option<TokenUsage>, newer: Option<TokenUsage>) -> Option<TokenUsage> {
    match (current, newer) {
        (Some(current), Some(newer)) => Some(current.updated_by(newer)),
        (current, newer) => newer.or(current),
    }
}

/// An error type name, kept only when it is a short identifier.
fn error_kind(event: &serde_json::Value) -> &str {
    event["error"]["type"]
        .as_str()
        .filter(|kind| {
            !kind.is_empty()
                && kind.len() <= 64
                && kind
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        })
        .unwrap_or("unknown")
}

#[async_trait]
impl StreamingAiProvider for AnthropicProvider {
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
        let mut body = self.build_chat_payload(&messages);
        body["stream"] = serde_json::Value::Bool(true);
        let request = http_client()?
            .post(endpoint(&self.base_url, "messages"))
            .timeout(self.request_timeout)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        let max_tokens = self.max_tokens;
        let mut usage = None;
        let stopped = sse::read_stream(
            request,
            self.provider_name(),
            MAX_RESPONSE_BYTES,
            cancellation,
            |frame| {
                let event: serde_json::Value = serde_json::from_str(&frame.data).map_err(|_| {
                    AiError::StreamProtocol("server-sent event data is not valid JSON")
                })?;
                let kind = event["type"]
                    .as_str()
                    .or(frame.event.as_deref())
                    .unwrap_or_default();
                match kind {
                    "message_start" => {
                        usage = merged(
                            usage,
                            TokenUsage::from_anthropic(&event["message"]["usage"]),
                        );
                    }
                    "content_block_start" => {
                        if let Some(text) = event["content_block"]["text"].as_str()
                            && !text.is_empty()
                        {
                            sink.send(text)?;
                        }
                    }
                    "content_block_delta" if event["delta"]["type"] == "text_delta" => {
                        let text = event["delta"]["text"]
                            .as_str()
                            .ok_or(AiError::StreamProtocol("a text delta has no text"))?;
                        if !text.is_empty() {
                            sink.send(text)?;
                        }
                    }
                    "message_delta" => {
                        usage = merged(usage, TokenUsage::from_anthropic(&event["usage"]));
                        reject_incomplete(&event["delta"], max_tokens)?;
                    }
                    "message_stop" => return Ok(Flow::Stop),
                    "error" => {
                        return Err(AiError::ApiError(format!(
                            "Anthropic stream error: {}",
                            error_kind(&event)
                        )));
                    }
                    _ => {}
                }
                Ok(Flow::Continue)
            },
        )
        .await?;
        if !stopped {
            return Err(AiError::StreamProtocol("stream ended without message_stop"));
        }
        if let Some(usage) = usage {
            sink.usage(usage);
        }
        Ok(())
    }
}
