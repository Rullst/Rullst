//! Strict OpenAI-compatible server-sent-event streaming transport.

use super::*;
use crate::ai::{
    AiCancellation, AiStreamSink, StreamLimits, StreamingAiProvider, guardrails::prepare_messages,
};
use reqwest::header::CONTENT_TYPE;

const MAX_SSE_EVENT_BYTES: usize = 128 * 1_024;

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

        let request = self.request("chat/completions").json(&serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        }));
        let response = tokio::select! {
            () = cancellation.cancelled() => return Err(AiError::Cancelled),
            response = request.send() => response?,
        };
        if !response.status().is_success() {
            return Err(AiError::ApiError(format!(
                "{} returned HTTP {}",
                self.provider_name(),
                response.status()
            )));
        }
        validate_stream_headers(&response)?;

        let mut decoder = SseDecoder::new(MAX_RESPONSE_BYTES);
        let mut stream = response.bytes_stream();
        loop {
            let next = tokio::select! {
                () = cancellation.cancelled() => return Err(AiError::Cancelled),
                next = stream.next() => next,
            };
            let Some(chunk) = next else {
                break;
            };
            let chunk = chunk?;
            for event in decoder.push(&chunk)? {
                match event {
                    SseEvent::Text(text) => sink.send(&text)?,
                    SseEvent::Done => return Ok(()),
                }
            }
        }
        decoder.finish()
    }
}

fn validate_stream_headers(response: &reqwest::Response) -> Result<(), AiError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(AiError::StreamProtocol(
            "declared response length exceeds the stream byte limit",
        ));
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if content_type != Some("text/event-stream") {
        return Err(AiError::StreamProtocol(
            "response content type is not text/event-stream",
        ));
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
enum SseEvent {
    Text(String),
    Done,
}

struct SseDecoder {
    buffer: Vec<u8>,
    response_bytes: usize,
    max_response_bytes: usize,
}

impl SseDecoder {
    fn new(max_response_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            response_bytes: 0,
            max_response_bytes,
        }
    }

    fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, AiError> {
        self.response_bytes = self
            .response_bytes
            .checked_add(chunk.len())
            .ok_or(AiError::StreamProtocol("response byte count overflowed"))?;
        if self.response_bytes > self.max_response_bytes {
            return Err(AiError::StreamProtocol(
                "response bytes exceed the stream byte limit",
            ));
        }
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() > MAX_SSE_EVENT_BYTES && event_boundary(&self.buffer).is_none() {
            return Err(AiError::StreamProtocol(
                "one server-sent event exceeds its byte limit",
            ));
        }

        let mut events = Vec::new();
        while let Some((boundary, delimiter_bytes)) = event_boundary(&self.buffer) {
            if boundary > MAX_SSE_EVENT_BYTES {
                return Err(AiError::StreamProtocol(
                    "one server-sent event exceeds its byte limit",
                ));
            }
            let event = self.buffer[..boundary].to_vec();
            self.buffer.drain(..boundary + delimiter_bytes);
            if let Some(event) = decode_event(&event)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    fn finish(self) -> Result<(), AiError> {
        if self.buffer.iter().all(u8::is_ascii_whitespace) {
            Err(AiError::StreamProtocol(
                "stream ended without the [DONE] sentinel",
            ))
        } else {
            Err(AiError::StreamProtocol(
                "stream ended with an incomplete server-sent event",
            ))
        }
    }
}

fn event_boundary(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if lf <= crlf => Some((lf, 2)),
        (Some(_), Some(crlf)) => Some((crlf, 4)),
        (Some(lf), None) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn decode_event(bytes: &[u8]) -> Result<Option<SseEvent>, AiError> {
    let event = std::str::from_utf8(bytes)
        .map_err(|_| AiError::StreamProtocol("server-sent event is not valid UTF-8"))?;
    let data = event
        .lines()
        .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
        .map(|value| value.strip_prefix(' ').unwrap_or(value))
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() {
        return Ok(None);
    }
    if data == "[DONE]" {
        return Ok(Some(SseEvent::Done));
    }
    let event: serde_json::Value = serde_json::from_str(&data)
        .map_err(|_| AiError::StreamProtocol("server-sent event data is not valid JSON"))?;
    match event["choices"][0]["delta"]["content"].as_str() {
        Some(text) if !text.is_empty() => Ok(Some(SseEvent::Text(text.to_string()))),
        Some(_) | None
            if event["choices"][0]["finish_reason"].is_string()
                || event["choices"][0]["delta"]["role"].is_string() =>
        {
            Ok(None)
        }
        _ => Err(AiError::StreamProtocol(
            "server-sent event has no supported chat delta",
        )),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn decoder_handles_fragmented_lf_and_crlf_events() {
        let mut decoder = SseDecoder::new(1_024);
        assert!(
            decoder
                .push(b"data: {\"choices\":[{\"delta\":{")
                .expect("prefix")
                .is_empty()
        );
        assert_eq!(
            decoder
                .push(b"\"content\":\"hello\"}}]}\n\n")
                .expect("complete event"),
            vec![SseEvent::Text("hello".to_string())]
        );
        assert_eq!(
            decoder.push(b"data: [DONE]\r\n\r\n").expect("done"),
            vec![SseEvent::Done]
        );
    }

    #[test]
    fn decoder_rejects_malformed_truncated_and_oversized_input() {
        let mut invalid = SseDecoder::new(1_024);
        assert!(matches!(
            invalid.push(b"data: not-json\n\n"),
            Err(AiError::StreamProtocol(_))
        ));
        let mut truncated = SseDecoder::new(1_024);
        truncated.push(b"data: {").expect("bounded prefix");
        assert!(matches!(
            truncated.finish(),
            Err(AiError::StreamProtocol(_))
        ));
        let mut oversized = SseDecoder::new(3);
        assert!(matches!(
            oversized.push(b"four"),
            Err(AiError::StreamProtocol(_))
        ));
    }
}
