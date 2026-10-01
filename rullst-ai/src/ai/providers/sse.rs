//! Bounded server-sent-event framing and the request loop shared by the
//! streaming transports (OpenAI-compatible, Anthropic and Gemini).
//!
//! Framing follows the `text/event-stream` format
//! (<https://html.spec.whatwg.org/multipage/server-sent-events.html>):
//! events end at a blank line, `data:` lines are joined with line feeds and
//! the optional `event:` line names the event. Each protocol interprets the
//! frames and decides how a stream must end.

use crate::ai::{AiCancellation, AiError};
use futures_util::StreamExt;
use reqwest::header::CONTENT_TYPE;

/// Largest single event accepted.
pub(super) const MAX_SSE_EVENT_BYTES: usize = 128 * 1_024;

/// Longest event delimiter (`\r\n\r\n`) minus one byte: the part of a
/// delimiter that can end one chunk and continue in the next.
const DELIMITER_OVERLAP_BYTES: usize = 3;

/// One event with data.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct SseFrame {
    pub(super) event: Option<String>,
    pub(super) data: String,
}

pub(super) struct SseDecoder {
    buffer: Vec<u8>,
    /// Buffer offset where the next delimiter search resumes. Earlier offsets
    /// were already searched and cannot start a delimiter.
    search_from: usize,
    response_bytes: usize,
    max_response_bytes: usize,
}

impl SseDecoder {
    pub(super) fn new(max_response_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            search_from: 0,
            response_bytes: 0,
            max_response_bytes,
        }
    }

    /// Appends one network chunk and returns every complete event with data.
    ///
    /// Each buffered byte is searched a bounded number of times and consumed
    /// events are removed once per call, so decoding is linear in the stream
    /// length however the server splits it into chunks.
    pub(super) fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseFrame>, AiError> {
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

        let mut frames = Vec::new();
        let mut consumed = 0;
        let mut search_from = self.search_from;
        while let Some((boundary, delimiter_bytes)) =
            event_boundary(&self.buffer, consumed, search_from)
        {
            let event = self
                .buffer
                .get(consumed..boundary)
                .filter(|event| event.len() <= MAX_SSE_EVENT_BYTES)
                .ok_or(AiError::StreamProtocol(
                    "one server-sent event exceeds its byte limit",
                ))?;
            if let Some(frame) = parse_frame(event)? {
                frames.push(frame);
            }
            consumed = boundary + delimiter_bytes;
            search_from = consumed;
        }
        if consumed == 0 && self.buffer.len() > MAX_SSE_EVENT_BYTES {
            return Err(AiError::StreamProtocol(
                "one server-sent event exceeds its byte limit",
            ));
        }
        self.buffer.drain(..consumed);
        self.search_from = self.buffer.len().saturating_sub(DELIMITER_OVERLAP_BYTES);
        Ok(frames)
    }

    /// Whether only whitespace remains after the last complete event.
    pub(super) fn is_clean(&self) -> bool {
        self.buffer.iter().all(u8::is_ascii_whitespace)
    }
}

/// Finds the earliest `\n\n` or `\r\n\r\n` delimiter that starts at or
/// after `max(start, search_from)`, returning its offset and length.
///
/// The two delimiters begin with different bytes, so the first matching
/// offset is the earliest delimiter; every offset is examined once.
fn event_boundary(bytes: &[u8], start: usize, search_from: usize) -> Option<(usize, usize)> {
    (start.max(search_from)..bytes.len()).find_map(|index| {
        let rest = bytes.get(index..)?;
        if rest.starts_with(b"\n\n") {
            Some((index, 2))
        } else if rest.starts_with(b"\r\n\r\n") {
            Some((index, 4))
        } else {
            None
        }
    })
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let value = line.strip_prefix(name)?.strip_prefix(':')?;
    Some(value.strip_prefix(' ').unwrap_or(value))
}

fn parse_frame(bytes: &[u8]) -> Result<Option<SseFrame>, AiError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AiError::StreamProtocol("server-sent event is not valid UTF-8"))?;
    let mut event = None;
    let mut data = Vec::new();
    for line in text.lines().map(|line| line.trim_end_matches('\r')) {
        if let Some(value) = field(line, "data") {
            data.push(value);
        } else if let Some(value) = field(line, "event") {
            event = Some(value.to_string());
        }
    }
    let data = data.join("\n");
    if data.is_empty() {
        return Ok(None);
    }
    Ok(Some(SseFrame { event, data }))
}

/// What a protocol handler wants after one frame.
pub(super) enum Flow {
    Continue,
    Stop,
}

fn transport_error(error: reqwest::Error) -> AiError {
    AiError::RequestError(error.without_url())
}

fn validate_headers(
    response: &reqwest::Response,
    max_response_bytes: usize,
) -> Result<(), AiError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_response_bytes as u64)
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

/// Sends `request`, racing `cancellation` against the request and every body
/// read, and passes each frame to `handle`. Returns `Ok(true)` when the
/// handler stopped the stream and `Ok(false)` when the body ended cleanly.
pub(super) async fn read_stream(
    request: reqwest::RequestBuilder,
    provider: &'static str,
    max_response_bytes: usize,
    cancellation: &AiCancellation,
    mut handle: impl FnMut(SseFrame) -> Result<Flow, AiError> + Send,
) -> Result<bool, AiError> {
    let response = tokio::select! {
        () = cancellation.cancelled() => return Err(AiError::Cancelled),
        response = request.send() => response.map_err(transport_error)?,
    };
    if !response.status().is_success() {
        return Err(AiError::ApiError(format!(
            "{provider} returned HTTP {}",
            response.status()
        )));
    }
    validate_headers(&response, max_response_bytes)?;
    let mut decoder = SseDecoder::new(max_response_bytes);
    let mut stream = response.bytes_stream();
    loop {
        let next = tokio::select! {
            () = cancellation.cancelled() => return Err(AiError::Cancelled),
            next = stream.next() => next,
        };
        let Some(chunk) = next else {
            break;
        };
        for frame in decoder.push(&chunk.map_err(transport_error)?)? {
            if let Flow::Stop = handle(frame)? {
                return Ok(true);
            }
        }
    }
    if decoder.is_clean() {
        Ok(false)
    } else {
        Err(AiError::StreamProtocol(
            "stream ended with an incomplete server-sent event",
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn data(text: &str) -> SseFrame {
        SseFrame {
            event: None,
            data: text.to_string(),
        }
    }

    #[test]
    fn decoder_handles_fragmented_lf_and_crlf_events() {
        let mut decoder = SseDecoder::new(1_024);
        assert!(decoder.push(b"data: {\"a\":").expect("prefix").is_empty());
        assert_eq!(
            decoder.push(b"1}\n\n").expect("complete event"),
            vec![data("{\"a\":1}")]
        );
        assert_eq!(
            decoder
                .push(b"event: ping\r\ndata: [DONE]\r\n\r\n")
                .expect("done"),
            vec![SseFrame {
                event: Some("ping".to_string()),
                data: "[DONE]".to_string()
            }]
        );

        let mut split = SseDecoder::new(1_024);
        assert!(
            split
                .push(b": ping\r\n\r")
                .expect("partial CRLF")
                .is_empty()
        );
        assert!(split.push(b"\ndata: x\n").expect("partial LF").is_empty());
        assert_eq!(split.push(b"\n").expect("split LF"), vec![data("x")]);
        assert!(split.is_clean());
    }

    #[test]
    fn decoder_rejects_invalid_truncated_and_oversized_input() {
        let mut invalid = SseDecoder::new(1_024);
        assert!(matches!(
            invalid.push(b"data: \xff\n\n"),
            Err(AiError::StreamProtocol(_))
        ));
        let mut truncated = SseDecoder::new(1_024);
        truncated.push(b"data: {").expect("bounded prefix");
        assert!(!truncated.is_clean());
        let mut oversized = SseDecoder::new(3);
        assert!(matches!(
            oversized.push(b"four"),
            Err(AiError::StreamProtocol(_))
        ));
    }

    #[test]
    fn byte_at_a_time_and_many_small_events_decode_in_linear_time() {
        // Rescanning the pending buffer on every push costs roughly n^2
        // window comparisons for one maximal event delivered byte by byte,
        // and rescanning or shifting the remainder for every event in one
        // large chunk is quadratic in the chunk size.
        let started = Instant::now();
        let content = "x".repeat(MAX_SSE_EVENT_BYTES - 64);
        let event = format!("data: {content}\r\n\r\n");
        let mut decoder = SseDecoder::new(2 * 1_024 * 1_024);
        let mut frames = Vec::new();
        for byte in event.as_bytes() {
            frames.extend(
                decoder
                    .push(std::slice::from_ref(byte))
                    .expect("bounded event"),
            );
        }
        assert_eq!(frames, vec![data(&content)]);

        let mut decoder = SseDecoder::new(2 * 1_024 * 1_024);
        let mut chunk = ": keep-alive\n\n".repeat(60_000).into_bytes();
        chunk.extend_from_slice(b"data: [DONE]\n\n");
        assert_eq!(
            decoder.push(&chunk).expect("bounded comment events"),
            vec![data("[DONE]")]
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "SSE decoding took {:?}",
            started.elapsed()
        );
    }
}
