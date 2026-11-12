//! DeepSeek's API (<https://api-docs.deepseek.com/>) as a preset of the
//! OpenAI-compatible adapter.
//!
//! Chat, JSON mode and streaming use `/chat/completions`, which DeepSeek
//! implements in the OpenAI shape. JSON Schema output does not: DeepSeek's
//! Chat Completions `response_format` accepts only `text` and `json_object`,
//! so structured output uses its `/responses` endpoint
//! (<https://api-docs.deepseek.com/api/create-response>), whose `text.format`
//! takes a `json_schema`.

use super::{OpenAiCompatibleCapabilities, OpenAiCompatibleProvider, Preset};
use crate::ai::{AiError, StructuredOutputSchema};

const BASE_URL: &str = "https://api.deepseek.com";
const DEFAULT_MODEL: &str = "deepseek-v4-flash";
/// The models DeepSeek documents for `/responses` JSON Schema output;
/// `deepseek-v4-flash` is the older name it still accepts for `deepseek-flash`.
const SCHEMA_MODELS: [&str; 3] = [DEFAULT_MODEL, "deepseek-flash", "deepseek-v4-pro"];

impl OpenAiCompatibleProvider {
    /// DeepSeek's API (`https://api.deepseek.com`) with Bearer `api_key`
    /// and the model `deepseek-v4-flash`; choose another with
    /// [`Self::try_with_model`].
    ///
    /// It declares chat, native JSON mode, streaming with usage and JSON
    /// Schema output (through DeepSeek's `/responses`, for
    /// `deepseek-v4-flash`, `deepseek-flash` and `deepseek-v4-pro`); DeepSeek
    /// offers no embeddings or image input, which fail with
    /// [`AiError::UnsupportedCapability`]. Surrounding whitespace in the key
    /// is ignored; an empty or `mock_*` key selects deterministic offline mode.
    pub fn deepseek(api_key: impl Into<String>) -> Result<Self, AiError> {
        let api_key = api_key.into();
        let mut provider = Self::try_cloud(BASE_URL, api_key.trim(), DEFAULT_MODEL)?
            .with_capabilities(
                OpenAiCompatibleCapabilities::chat_only()
                    .with_json_schema()
                    .with_stream_usage(),
            );
        provider.preset = Preset::DeepSeek;
        Ok(provider)
    }
}

/// Whether DeepSeek documents JSON Schema output for `model`.
pub(super) fn schema_model(model: &str) -> bool {
    SCHEMA_MODELS.contains(&model)
}

/// JSON Schema output through DeepSeek's `/responses`: the text of the first
/// `output_text` part of the first `message` output item.
pub(super) async fn structured_output(
    provider: &OpenAiCompatibleProvider,
    text: &str,
    schema: &StructuredOutputSchema,
) -> Result<String, AiError> {
    let json = provider
        .request_json(
            "responses",
            serde_json::json!({
                "model": provider.model,
                "input": text,
                "text": {"format": {
                    "type": "json_schema",
                    "name": schema.name(),
                    "schema": schema.schema()
                }}
            }),
        )
        .await?;
    output_text(&json)
        .ok_or_else(|| AiError::ApiError("DeepSeek returned no structured output text".to_string()))
}

fn output_text(json: &serde_json::Value) -> Option<String> {
    json["output"]
        .as_array()?
        .iter()
        .find_map(|item| {
            (item["type"] == "message")
                .then(|| item["content"].as_array())
                .flatten()
        })?
        .iter()
        .find_map(|part| {
            (part["type"] == "output_text")
                .then(|| part["text"].as_str())
                .flatten()
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_text_reads_the_first_message_text_part() {
        let json = serde_json::json!({"output": [
            {"type": "reasoning", "content": [{"type": "output_text", "text": "no"}]},
            {"type": "message", "content": [
                {"type": "refusal", "text": "no"},
                {"type": "output_text", "text": "{\"ok\":true}"}
            ]}
        ]});
        assert_eq!(output_text(&json).as_deref(), Some("{\"ok\":true}"));
        for json in [
            serde_json::json!({}),
            serde_json::json!({"output": [{"type": "message", "content": []}]}),
            serde_json::json!({"output": [{"type": "message", "content": [{"type": "output_text"}]}]}),
            serde_json::json!({"output": [{"type": "reasoning", "content": [{"type": "output_text", "text": "x"}]}]}),
        ] {
            assert_eq!(output_text(&json), None, "{json}");
        }
    }

    #[test]
    fn schema_output_is_limited_to_the_documented_models() {
        for model in ["deepseek-v4-flash", "deepseek-flash", "deepseek-v4-pro"] {
            assert!(schema_model(model), "{model}");
        }
        for model in ["deepseek-chat", "deepseek-reasoner", "", "DEEPSEEK-FLASH"] {
            assert!(!schema_model(model), "{model}");
        }
    }
}
