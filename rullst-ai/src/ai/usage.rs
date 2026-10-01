//! Provider-reported token usage (v13).
//!
//! Counts come only from the provider's own response fields; a provider or
//! response without them yields `None`, never an estimate. Each parser cites
//! the documented field names it reads.

use serde_json::Value;

/// Token counts a provider reported for one request.
///
/// `input_tokens` counts every prompt token, including tokens served from a
/// prompt cache; `cached_input_tokens` is the cached subset when the provider
/// reports it. `total_tokens` is the provider's total when it sends one,
/// otherwise input plus output.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
}

impl TokenUsage {
    /// Creates usage from input and output counts, for custom providers.
    #[must_use]
    pub const fn new(input_tokens: Option<u64>, output_tokens: Option<u64>) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: None,
            cached_input_tokens: None,
        }
    }

    /// Records the provider's own total.
    #[must_use]
    pub const fn with_total_tokens(mut self, total_tokens: u64) -> Self {
        self.total_tokens = Some(total_tokens);
        self
    }

    /// Records how many input tokens were served from a prompt cache.
    #[must_use]
    pub const fn with_cached_input_tokens(mut self, cached_input_tokens: u64) -> Self {
        self.cached_input_tokens = Some(cached_input_tokens);
        self
    }

    /// Prompt tokens, including cached ones.
    #[must_use]
    pub const fn input_tokens(&self) -> Option<u64> {
        self.input_tokens
    }

    /// Generated tokens, including reasoning tokens when the provider counts them.
    #[must_use]
    pub const fn output_tokens(&self) -> Option<u64> {
        self.output_tokens
    }

    /// The provider's total, or input plus output when both are known.
    #[must_use]
    pub fn total_tokens(&self) -> Option<u64> {
        self.total_tokens
            .or_else(|| self.input_tokens?.checked_add(self.output_tokens?))
    }

    /// Input tokens served from a prompt cache, when reported.
    #[must_use]
    pub const fn cached_input_tokens(&self) -> Option<u64> {
        self.cached_input_tokens
    }

    const fn is_empty(&self) -> bool {
        self.input_tokens.is_none()
            && self.output_tokens.is_none()
            && self.total_tokens.is_none()
            && self.cached_input_tokens.is_none()
    }

    /// `None` when no count was present.
    pub(crate) const fn reported(self) -> Option<Self> {
        if self.is_empty() { None } else { Some(self) }
    }

    /// OpenAI Chat Completions `usage`: `prompt_tokens`,
    /// `completion_tokens`, `total_tokens` and
    /// `prompt_tokens_details.cached_tokens`
    /// (<https://platform.openai.com/docs/api-reference/chat/object>).
    /// DeepSeek uses the same names and reports cache hits as
    /// `prompt_cache_hit_tokens`
    /// (<https://api-docs.deepseek.com/api/create-chat-completion>);
    /// OpenAI-compatible servers that send `usage` follow the OpenAI shape.
    pub(crate) fn from_openai(usage: &Value) -> Option<Self> {
        Self {
            input_tokens: usage["prompt_tokens"].as_u64(),
            output_tokens: usage["completion_tokens"].as_u64(),
            total_tokens: usage["total_tokens"].as_u64(),
            cached_input_tokens: usage["prompt_tokens_details"]["cached_tokens"]
                .as_u64()
                .or_else(|| usage["prompt_cache_hit_tokens"].as_u64()),
        }
        .reported()
    }

    /// Anthropic Messages `usage`: `input_tokens`, `output_tokens`,
    /// `cache_creation_input_tokens` and `cache_read_input_tokens`; the total
    /// input is the sum of the three input fields
    /// (<https://docs.anthropic.com/en/api/messages>). Streams report the
    /// same object in `message_start` and cumulative counts in
    /// `message_delta` (<https://docs.anthropic.com/en/docs/build-with-claude/streaming>).
    pub(crate) fn from_anthropic(usage: &Value) -> Option<Self> {
        let uncached = usage["input_tokens"].as_u64();
        let creation = usage["cache_creation_input_tokens"].as_u64();
        let read = usage["cache_read_input_tokens"].as_u64();
        let input_tokens = match (uncached, creation, read) {
            (None, None, None) => None,
            _ => uncached
                .unwrap_or(0)
                .checked_add(creation.unwrap_or(0))
                .and_then(|sum| sum.checked_add(read.unwrap_or(0))),
        };
        Self {
            input_tokens,
            output_tokens: usage["output_tokens"].as_u64(),
            total_tokens: None,
            cached_input_tokens: read,
        }
        .reported()
    }

    /// Gemini `usageMetadata`: `promptTokenCount`, `candidatesTokenCount`,
    /// `thoughtsTokenCount`, `totalTokenCount` and `cachedContentTokenCount`
    /// (<https://ai.google.dev/api/generate-content#UsageMetadata>). Thinking
    /// tokens are generated tokens, so they are counted as output.
    pub(crate) fn from_gemini(metadata: &Value) -> Option<Self> {
        let candidates = metadata["candidatesTokenCount"].as_u64();
        let thoughts = metadata["thoughtsTokenCount"].as_u64();
        let output_tokens = match (candidates, thoughts) {
            (None, None) => None,
            _ => candidates.unwrap_or(0).checked_add(thoughts.unwrap_or(0)),
        };
        Self {
            input_tokens: metadata["promptTokenCount"].as_u64(),
            output_tokens,
            total_tokens: metadata["totalTokenCount"].as_u64(),
            cached_input_tokens: metadata["cachedContentTokenCount"].as_u64(),
        }
        .reported()
    }

    /// Ollama `/api/chat` final response: `prompt_eval_count` and
    /// `eval_count` (<https://github.com/ollama/ollama/blob/main/docs/api.md>).
    pub(crate) fn from_ollama(response: &Value) -> Option<Self> {
        Self::new(
            response["prompt_eval_count"].as_u64(),
            response["eval_count"].as_u64(),
        )
        .reported()
    }
}

/// A complete chat answer with the usage the provider reported, if any.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatCompletion {
    text: String,
    usage: Option<TokenUsage>,
}

impl ChatCompletion {
    /// Creates a completion, for custom providers.
    pub fn new(text: impl Into<String>, usage: Option<TokenUsage>) -> Self {
        Self {
            text: text.into(),
            usage,
        }
    }

    /// The answer text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Consumes the completion and returns the answer text.
    #[must_use]
    pub fn into_text(self) -> String {
        self.text
    }

    /// Provider-reported usage; `None` when the response carried none.
    #[must_use]
    pub const fn usage(&self) -> Option<TokenUsage> {
        self.usage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_and_deepseek_usage_follow_their_documented_fields() {
        let usage = TokenUsage::from_openai(&json!({
            "prompt_tokens": 12, "completion_tokens": 5, "total_tokens": 17,
            "prompt_tokens_details": {"cached_tokens": 4}
        }))
        .expect("usage");
        assert_eq!(usage.input_tokens(), Some(12));
        assert_eq!(usage.output_tokens(), Some(5));
        assert_eq!(usage.total_tokens(), Some(17));
        assert_eq!(usage.cached_input_tokens(), Some(4));
        let deepseek = TokenUsage::from_openai(&json!({
            "prompt_tokens": 9, "completion_tokens": 1, "prompt_cache_hit_tokens": 8
        }))
        .expect("usage");
        assert_eq!(deepseek.cached_input_tokens(), Some(8));
        assert_eq!(deepseek.total_tokens(), Some(10));
        assert_eq!(TokenUsage::from_openai(&Value::Null), None);
        assert_eq!(
            TokenUsage::from_openai(&json!({"prompt_tokens": "7"})),
            None
        );
    }

    #[test]
    fn anthropic_input_includes_cache_reads_and_writes() {
        let usage = TokenUsage::from_anthropic(&json!({
            "input_tokens": 10, "cache_creation_input_tokens": 20,
            "cache_read_input_tokens": 30, "output_tokens": 4
        }))
        .expect("usage");
        assert_eq!(usage.input_tokens(), Some(60));
        assert_eq!(usage.cached_input_tokens(), Some(30));
        assert_eq!(usage.total_tokens(), Some(64));
    }

    #[test]
    fn gemini_counts_thoughts_as_output_and_keeps_its_total() {
        let usage = TokenUsage::from_gemini(&json!({
            "promptTokenCount": 7, "candidatesTokenCount": 3,
            "thoughtsTokenCount": 2, "totalTokenCount": 12, "cachedContentTokenCount": 1
        }))
        .expect("usage");
        assert_eq!(usage.output_tokens(), Some(5));
        assert_eq!(usage.total_tokens(), Some(12));
        assert_eq!(usage.cached_input_tokens(), Some(1));
    }

    #[test]
    fn ollama_counts_and_absent_fields() {
        let usage = TokenUsage::from_ollama(&json!({"prompt_eval_count": 26, "eval_count": 290}))
            .expect("usage");
        assert_eq!(usage.total_tokens(), Some(316));
        assert_eq!(TokenUsage::from_ollama(&json!({"done": true})), None);
        assert_eq!(TokenUsage::new(Some(1), None).total_tokens(), None);
        assert_eq!(
            TokenUsage::new(Some(u64::MAX), Some(1)).total_tokens(),
            None,
            "an overflowing sum is unknown, not wrapped"
        );
    }
}
