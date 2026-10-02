//! Strict OpenAI-compatible server-sent-event streaming transport.
//!
//! Chunks follow the Chat Completions streaming shape
//! (<https://platform.openai.com/docs/api-reference/chat-streaming>): text in
//! `choices[0].delta.content`, a terminal `data: [DONE]`, and, when
//! `stream_options.include_usage` is requested, a final chunk with empty
//! `choices` and a `usage` object. A `finish_reason` of `length` or
//! `content_filter` fails the stream, as the non-streaming OpenAI transport
//! does, so a truncated or withheld reply is never presented as complete.

use super::*;
use crate::ai::providers::sse::{self, Flow, SseFrame};
use crate::ai::providers::support::reject_incomplete_choice;
use crate::ai::{
    AiCancellation, AiStreamSink, StreamLimits, StreamingAiProvider, TokenUsage,
    guardrails::prepare_messages,
};

#[async_trait]
impl StreamingAiProvider for OpenAiCompatibleProvider {
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
        if !self.capabilities.streaming {
            return Err(self.unsupported("incremental streaming"));
        }
        if cancellation.is_cancelled() {
            return Err(AiError::Cancelled);
        }
        if self.mode.is_mock() {
            let response = mock::chat_response(self.provider_name(), &self.model, &messages);
            return sink.send(&response);
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });
        if self.capabilities.stream_usage {
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }
        let request = self.request("chat/completions").json(&body);
        let mut usage = None;
        let done = sse::read_stream(
            request,
            self.provider_name(),
            MAX_RESPONSE_BYTES,
            cancellation,
            |frame| {
                let Some(chunk) = interpret(&frame, self.provider_name())? else {
                    return Ok(Flow::Stop);
                };
                if chunk.usage.is_some() {
                    usage = chunk.usage;
                }
                if let Some(text) = chunk.text {
                    sink.send(&text)?;
                }
                Ok(Flow::Continue)
            },
        )
        .await?;
        if !done {
            return Err(AiError::StreamProtocol(
                "stream ended without the [DONE] sentinel",
            ));
        }
        if let Some(usage) = usage {
            sink.usage(usage);
        }
        Ok(())
    }
}

/// One interpreted chunk; text is never empty.
#[derive(Debug, PartialEq, Eq)]
struct Chunk {
    text: Option<String>,
    usage: Option<TokenUsage>,
}

/// `None` for the `[DONE]` sentinel.
fn interpret(frame: &SseFrame, provider: &'static str) -> Result<Option<Chunk>, AiError> {
    if frame.data == "[DONE]" {
        return Ok(None);
    }
    let event: serde_json::Value = serde_json::from_str(&frame.data)
        .map_err(|_| AiError::StreamProtocol("server-sent event data is not valid JSON"))?;
    reject_incomplete_choice(&event, provider)?;
    let usage = event
        .get("usage")
        .filter(|usage| usage.is_object())
        .and_then(TokenUsage::from_openai);
    let choice = &event["choices"][0];
    let text = match choice["delta"]["content"].as_str() {
        Some(text) if !text.is_empty() => Some(text.to_string()),
        Some(_) | None
            if choice["finish_reason"].is_string() || choice["delta"]["role"].is_string() =>
        {
            None
        }
        // The usage chunk requested by `stream_options.include_usage`.
        _ if usage.is_some() && event["choices"].as_array().is_some_and(Vec::is_empty) => None,
        _ => {
            return Err(AiError::StreamProtocol(
                "server-sent event has no supported chat delta",
            ));
        }
    };
    Ok(Some(Chunk { text, usage }))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn interpret_test(frame: &SseFrame) -> Result<Option<Chunk>, AiError> {
        interpret(frame, "fixture")
    }

    fn frame(data: &str) -> SseFrame {
        SseFrame {
            event: None,
            data: data.to_string(),
        }
    }

    #[test]
    fn chunks_carry_text_usage_or_the_sentinel() {
        assert_eq!(
            interpret_test(&frame(
                r#"{"choices":[{"delta":{"content":"hi"}}],"usage":null}"#
            ))
            .expect("text"),
            Some(Chunk {
                text: Some("hi".to_string()),
                usage: None
            })
        );
        let usage = interpret_test(&frame(
            r#"{"choices":[],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}"#,
        ))
        .expect("usage chunk")
        .expect("not done")
        .usage
        .expect("usage");
        assert_eq!(usage.total_tokens(), Some(5));
        assert_eq!(interpret_test(&frame("[DONE]")).expect("done"), None);
        assert!(
            interpret_test(&frame(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#))
                .expect("role")
                .is_some()
        );
    }

    #[test]
    fn truncated_or_filtered_replies_fail() {
        for reason in ["length", "content_filter"] {
            let data = format!(
                r#"{{"choices":[{{"delta":{{"content":"part"}},"finish_reason":"{reason}"}}]}}"#
            );
            let error = interpret_test(&frame(&data)).expect_err("incomplete reply");
            assert!(
                matches!(&error, AiError::ApiError(message) if message.contains(reason)),
                "{error}"
            );
        }
        let stop = r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#;
        assert!(interpret_test(&frame(stop)).expect("stop").is_some());
    }

    #[test]
    fn malformed_chunks_fail_closed() {
        for data in [
            "not-json",
            r#"{"choices":[]}"#,
            r#"{"choices":[{"delta":{}}]}"#,
        ] {
            assert!(
                matches!(
                    interpret_test(&frame(data)),
                    Err(AiError::StreamProtocol(_))
                ),
                "{data}"
            );
        }
    }
}
