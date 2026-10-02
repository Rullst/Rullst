//! Coalesces provider deltas so a long answer fits the stream chunk limit.
//!
//! `StreamingAiClient` accepts at most `max_chunks` (4,096) chunks for one
//! answer, while OpenAI-compatible and Anthropic streams send a delta per
//! token or so: an answer of a few thousand tokens would fail although it is
//! far below the 2 MiB byte limit. [`Coalesced`] buffers the deltas and hands
//! them on so that the chunk limit can never be reached before the byte
//! limit:
//! - a delivery of at least `quantum` bytes (twice the byte limit divided by
//!   the chunk limit, 1 KiB by default) needs at most half of the chunks for a
//!   whole answer;
//! - smaller deliveries, which keep the display live, happen at most every
//!   100 ms and only while the other half of the chunks lasts;
//! - the remainder is delivered when the stream ends.

use async_trait::async_trait;
use rullst_ai::{
    AiCancellation, AiError, AiProvider, AiStreamSink, ChatCompletion, Message,
    ProviderCapabilities, StreamLimits, StreamingAiProvider, TokenUsage,
};
use std::time::{Duration, Instant};

/// Shortest interval between two small deliveries.
const INTERVAL: Duration = Duration::from_millis(100);

/// A streaming provider whose deltas are coalesced before the client's
/// bounded sink counts them.
#[derive(Debug)]
pub(super) struct Coalesced<P>(pub P);

#[async_trait]
impl<P: AiProvider> AiProvider for Coalesced<P> {
    fn provider_name(&self) -> &'static str {
        self.0.provider_name()
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.0.capabilities()
    }

    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        self.0.prompt(text).await
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        self.0.chat(messages).await
    }

    async fn chat_with_usage(&self, messages: &[Message]) -> Result<ChatCompletion, AiError> {
        self.0.chat_with_usage(messages).await
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiError> {
        self.0.embed(text).await
    }
}

#[async_trait]
impl<P: StreamingAiProvider> StreamingAiProvider for Coalesced<P> {
    async fn stream_chat<S>(
        &self,
        messages: &[Message],
        limits: StreamLimits,
        cancellation: &AiCancellation,
        sink: &mut S,
    ) -> Result<(), AiError>
    where
        S: AiStreamSink,
    {
        let mut buffer = Buffer::new(sink, limits);
        self.0
            .stream_chat(messages, limits, cancellation, &mut buffer)
            .await?;
        buffer.finish()
    }
}

struct Buffer<'a, S> {
    sink: &'a mut S,
    pending: String,
    /// Smallest delivery that does not use the small-delivery budget.
    quantum: usize,
    max_chunk_bytes: usize,
    /// Small deliveries left; one chunk stays reserved for the remainder.
    small_left: usize,
    last: Option<Instant>,
    /// Limits too small to coalesce within: deltas pass through unchanged.
    passthrough: bool,
}

impl<'a, S: AiStreamSink> Buffer<'a, S> {
    fn new(sink: &'a mut S, limits: StreamLimits) -> Self {
        let half = limits.max_chunks() / 2;
        let quantum = limits.max_output_bytes().div_ceil(half.max(1));
        Self {
            sink,
            pending: String::new(),
            quantum,
            max_chunk_bytes: limits.max_chunk_bytes(),
            small_left: (limits.max_chunks() - half).saturating_sub(1),
            last: None,
            // A full piece is cut at a character boundary at most 3 bytes
            // below the chunk limit and must still reach the quantum.
            passthrough: half == 0 || quantum + 4 > limits.max_chunk_bytes(),
        }
    }

    fn deliver(&mut self, piece: &str) -> Result<(), AiError> {
        self.last = Some(Instant::now());
        self.sink.send(piece)
    }

    fn finish(mut self) -> Result<(), AiError> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let rest = std::mem::take(&mut self.pending);
        self.sink.send(&rest)
    }
}

impl<S: AiStreamSink> AiStreamSink for Buffer<'_, S> {
    fn send(&mut self, chunk: &str) -> Result<(), AiError> {
        if self.passthrough {
            return self.sink.send(chunk);
        }
        if chunk.len() > self.max_chunk_bytes {
            return Err(AiError::StreamProtocol(
                "one output chunk exceeds its byte limit",
            ));
        }
        self.pending.push_str(chunk);
        while self.pending.len() >= self.quantum {
            let mut end = self.pending.len().min(self.max_chunk_bytes);
            while !self.pending.is_char_boundary(end) {
                end -= 1;
            }
            let piece: String = self.pending.drain(..end).collect();
            self.deliver(&piece)?;
        }
        let due = self.last.is_none_or(|last| last.elapsed() >= INTERVAL);
        if !self.pending.is_empty() && self.small_left > 0 && due {
            self.small_left -= 1;
            let piece = std::mem::take(&mut self.pending);
            self.deliver(&piece)?;
        }
        Ok(())
    }

    fn usage(&mut self, usage: TokenUsage) {
        self.sink.usage(usage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rullst_ai::StreamingAiClient;

    /// Streams `deltas` one by one, like a token-level SSE stream.
    struct Deltas(Vec<String>);

    #[async_trait]
    impl AiProvider for Deltas {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                streaming: true,
                ..ProviderCapabilities::PORTABLE
            }
        }

        async fn prompt(&self, _text: &str) -> Result<String, AiError> {
            Ok(String::new())
        }

        async fn chat(&self, _messages: &[Message]) -> Result<String, AiError> {
            Ok(String::new())
        }

        async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
            Ok(Vec::new())
        }
    }

    #[async_trait]
    impl StreamingAiProvider for Deltas {
        async fn stream_chat<S>(
            &self,
            _messages: &[Message],
            _limits: StreamLimits,
            _cancellation: &AiCancellation,
            sink: &mut S,
        ) -> Result<(), AiError>
        where
            S: AiStreamSink,
        {
            for delta in &self.0 {
                sink.send(delta)?;
            }
            sink.usage(TokenUsage::new(Some(1), Some(2)));
            Ok(())
        }
    }

    fn deltas(count: usize) -> Vec<String> {
        (0..count)
            .map(|index| if index % 3 == 0 { "é" } else { "a" }.to_string())
            .collect()
    }

    #[tokio::test]
    async fn long_token_streams_fit_the_chunk_limit() {
        let parts = deltas(20_000);
        let expected = parts.concat();
        let messages = [Message::user("write a long file")];
        let cancellation = AiCancellation::new();

        let plain = StreamingAiClient::new(Deltas(parts.clone()));
        let failed = plain
            .stream_chat(&messages, &cancellation, &mut |_: &str| Ok(()))
            .await;
        assert!(matches!(failed, Err(AiError::StreamProtocol(_))));

        let client = StreamingAiClient::new(Coalesced(Deltas(parts)));
        let mut text = String::new();
        let summary = client
            .stream_chat(&messages, &cancellation, &mut |chunk: &str| {
                text.push_str(chunk);
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(text, expected);
        assert!(summary.chunks() <= StreamLimits::default().max_chunks());
        assert_eq!(summary.usage(), Some(TokenUsage::new(Some(1), Some(2))));
    }

    #[tokio::test]
    async fn smaller_limits_bound_deliveries_and_bytes() {
        let limits = StreamLimits::try_new(8, 1024, 2048).unwrap();
        let cancellation = AiCancellation::new();
        let messages = [Message::user("hi")];
        let client = StreamingAiClient::new(Coalesced(Deltas(vec!["a".to_string(); 2048])))
            .with_limits(limits);
        let mut deliveries = 0;
        let summary = client
            .stream_chat(&messages, &cancellation, &mut |_: &str| {
                deliveries += 1;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(summary.output_bytes(), 2048);
        assert!(deliveries <= 8, "{deliveries} deliveries");

        let over = StreamingAiClient::new(Coalesced(Deltas(vec!["a".to_string(); 2049])))
            .with_limits(limits);
        let error = over
            .stream_chat(&messages, &cancellation, &mut |_: &str| Ok(()))
            .await;
        assert!(matches!(error, Err(AiError::StreamProtocol(_))));
    }
}
